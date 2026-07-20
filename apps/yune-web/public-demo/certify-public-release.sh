#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
# shellcheck disable=SC1091
. "$SCRIPT_DIR/public-release-toolchain.sh"

cd "$YUNE_WEB_REPO_ROOT"
yune_web_activate_release_toolchain

DEFAULT_DIST="$SCRIPT_DIR/dist"
if [ ! -f "$DEFAULT_DIST/build-info.json" ]; then
	printf 'Missing source-identified public artifact at %s.\n' "$DEFAULT_DIST" >&2
	exit 1
fi

certification_temp=
alias_temp=
aliased_default_dist=false
cleanup() {
	exit_status=$?
	trap - EXIT
	set +e
	cleanup_failed=false
	if [ "$aliased_default_dist" = true ]; then
		if [ -L "$DEFAULT_DIST" ]; then
			rm -f -- "$DEFAULT_DIST" || cleanup_failed=true
		elif [ -e "$DEFAULT_DIST" ]; then
			printf 'Refusing to overwrite unexpected path while restoring %s.\n' "$DEFAULT_DIST" >&2
			cleanup_failed=true
		fi
		if [ ! -e "$DEFAULT_DIST" ]; then
			mv -- "$alias_temp/build-output-dist" "$DEFAULT_DIST" || cleanup_failed=true
		fi
	fi
	if [ -n "$alias_temp" ] && [ "$cleanup_failed" = false ]; then
		rm -rf -- "$alias_temp"
	fi
	if [ -n "$certification_temp" ]; then
		if [ "$exit_status" -eq 0 ] && [ "$cleanup_failed" = false ]; then
			rm -rf -- "$certification_temp"
		else
			printf 'Preserved WEB06 certification failure artifacts at %s\n' "$certification_temp" >&2
		fi
	fi
	if [ "$cleanup_failed" = true ] && [ "$exit_status" -eq 0 ]; then
		exit_status=1
	fi
	exit "$exit_status"
}
trap cleanup EXIT

archive_sha256() {
	node -e 'const fs=require("node:fs");const crypto=require("node:crypto");const hash=crypto.createHash("sha256");hash.update(fs.readFileSync(process.argv[1]));process.stdout.write(`${hash.digest("hex")}\n`);' "$1"
}

safe_extract() {
	ARCHIVE="$1" DESTINATION="$2" python3 - <<'PY'
import os
import pathlib
import tarfile

archive = pathlib.Path(os.environ["ARCHIVE"])
destination = pathlib.Path(os.environ["DESTINATION"])
destination.mkdir(parents=True, exist_ok=False)
seen = set()
with tarfile.open(archive, "r:gz") as bundle:
    members = bundle.getmembers()
    for member in members:
        relative = pathlib.PurePosixPath(member.name)
        normalized = relative.as_posix()
        if normalized == "." and member.isdir():
            continue
        if (
            relative.is_absolute()
            or ".." in relative.parts
            or "\\" in member.name
            or not (member.isfile() or member.isdir())
            or normalized == "."
            or normalized in seen
        ):
            raise SystemExit(f"unsafe archive member: {member.name!r}")
        seen.add(normalized)
    bundle.extractall(destination, members=members, filter="data")
PY
}

configured_archive=${YUNE_WEB_CERTIFIED_ARCHIVE:-}
configured_archive_sha256=${YUNE_WEB_CERTIFIED_ARCHIVE_SHA256:-}
configured_dist_root=${YUNE_WEB_CERTIFIED_DIST_ROOT:-}
if [ -n "$configured_archive" ] || [ -n "$configured_archive_sha256" ] || [ -n "$configured_dist_root" ]; then
	if [ -z "$configured_archive" ] || [ -z "$configured_archive_sha256" ] || [ -z "$configured_dist_root" ]; then
		printf 'YUNE_WEB_CERTIFIED_ARCHIVE, YUNE_WEB_CERTIFIED_ARCHIVE_SHA256, and YUNE_WEB_CERTIFIED_DIST_ROOT must be supplied together.\n' >&2
		exit 1
	fi
	printf '%s\n' "$configured_archive_sha256" | grep -Eq '^[0-9a-f]{64}$' || {
		printf 'YUNE_WEB_CERTIFIED_ARCHIVE_SHA256 must be a full lowercase SHA-256.\n' >&2
		exit 1
	}
	[ -f "$configured_archive" ] || {
		printf 'Certified archive does not exist: %s\n' "$configured_archive" >&2
		exit 1
	}
	[ -f "$configured_archive.sha256" ] || {
		printf 'Certified archive sibling digest does not exist: %s.sha256\n' "$configured_archive" >&2
		exit 1
	}
	actual_archive_sha256=$(archive_sha256 "$configured_archive")
	sibling_archive_sha256=$(tr -d '[:space:]' < "$configured_archive.sha256")
	[ "$actual_archive_sha256" = "$configured_archive_sha256" ]
	[ "$actual_archive_sha256" = "$sibling_archive_sha256" ]
	[ -d "$configured_dist_root" ] || {
		printf 'Certified extracted artifact does not exist: %s\n' "$configured_dist_root" >&2
		exit 1
	}
	certified_dist_root=$(CDPATH= cd -- "$configured_dist_root" && pwd -P)
	certified_archive_sha256=$actual_archive_sha256
else
	# The local compatibility entrypoint also follows the seal-once contract.
	# CI supplies the already-sealed archive and never enters this fallback.
	certification_temp=$(mktemp -d "${TMPDIR:-/tmp}/yune-web06-certify.XXXXXX")
	configured_archive="$certification_temp/yune-web-dist.tar.gz"
	tar -C "$DEFAULT_DIST" -czf "$configured_archive" .
	certified_archive_sha256=$(archive_sha256 "$configured_archive")
	printf '%s\n' "$certified_archive_sha256" > "$configured_archive.sha256"
	safe_extract "$configured_archive" "$certification_temp/extracted-dist"
	certified_dist_root=$(CDPATH= cd -- "$certification_temp/extracted-dist" && pwd -P)
fi

expected_source_commit=${YUNE_WEB_EXPECTED_SOURCE_COMMIT:-$(git rev-parse HEAD)}
printf '%s\n' "$expected_source_commit" | grep -Eq '^[0-9a-f]{40}$' || {
	printf 'YUNE_WEB_EXPECTED_SOURCE_COMMIT must be a full lowercase Git SHA.\n' >&2
	exit 1
}

if [ -z "${YUNE_WEB_WEB06_EVIDENCE_DIR:-}" ]; then
	if [ -z "$certification_temp" ]; then
		certification_temp=$(mktemp -d "${TMPDIR:-/tmp}/yune-web06-certify.XXXXXX")
	fi
	export YUNE_WEB_WEB06_EVIDENCE_DIR="$certification_temp/web06-evidence"
fi
mkdir -p "$YUNE_WEB_WEB06_EVIDENCE_DIR"

export YUNE_WEB_EXPECTED_DIST="$certified_dist_root"
export YUNE_WEB_EXPECTED_SOURCE_COMMIT="$expected_source_commit"
export YUNE_WEB_LOCAL_ARTIFACT_RECEIPT=${YUNE_WEB_LOCAL_ARTIFACT_RECEIPT:-"$YUNE_WEB_WEB06_EVIDENCE_DIR/local-artifact-verification.json"}
node apps/yune-web/e2e/verify-local-artifact.mjs

# WEB03's public runner intentionally remains unchanged and resolves the fixed
# public-demo/dist path. Temporarily alias that ignored build output to the
# verified extracted archive so the legacy gate measures the sealed bytes.
default_dist_root=$(CDPATH= cd -- "$DEFAULT_DIST" && pwd -P)
if [ "$default_dist_root" != "$certified_dist_root" ]; then
	alias_temp=$(mktemp -d "${TMPDIR:-/tmp}/yune-web06-dist-alias.XXXXXX")
	mv -- "$DEFAULT_DIST" "$alias_temp/build-output-dist"
	aliased_default_dist=true
	ln -s "$certified_dist_root" "$DEFAULT_DIST"
fi

export PLAYWRIGHT_BROWSERS_PATH=${PLAYWRIGHT_BROWSERS_PATH:-"$YUNE_WEB_REPO_ROOT/.cache/ms-playwright"}
npm --prefix apps/yune-web/e2e ci
node apps/yune-web/e2e/node_modules/@playwright/test/cli.js install --with-deps chromium

# These are the binding WEB03-11 values. Diagnostic overrides are deliberately
# overwritten so this entrypoint can produce only release-grade evidence.
export YUNE_WEB_LATENCY_CPU_THROTTLE=4
export YUNE_WEB_LATENCY_KEY_INTERVAL_MS=250
export YUNE_WEB_LATENCY_P95_MS=750
export YUNE_WEB_LATENCY_MAX_MS=1000
npm --prefix apps/yune-web/e2e run test:e2e:input-latency:public

# WEB06 is a distinct focused gate. It receives the plain extracted root, not
# the temporary WEB03 alias, and its runner rejects a missing archive identity.
export YUNE_WEB_CERTIFIED_ARCHIVE="$configured_archive"
export YUNE_WEB_CERTIFIED_ARCHIVE_SHA256="$certified_archive_sha256"
export YUNE_WEB_WEB06_DIST_ROOT="$certified_dist_root"
export YUNE_WEB_WEB06_GATE_SCOPE=full
npm --prefix apps/yune-web/e2e run test:e2e:web06:public
