#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
# shellcheck disable=SC1091
. "$SCRIPT_DIR/public-release-toolchain.sh"

cd "$YUNE_WEB_REPO_ROOT"
yune_web_activate_release_toolchain

certification_mode=${YUNE_WEB_RELEASE_CERTIFICATION_MODE:-}
case "$certification_mode" in
legacy | measured-web06) ;;
*)
	printf 'YUNE_WEB_RELEASE_CERTIFICATION_MODE must be explicitly selected as legacy or measured-web06.\n' >&2
	exit 1
	;;
esac

# Certification, dependency installation, and local browser guards never need
# deployment authority. Strip every supported Cloudflare/Wrangler credential
# family inside this child shell even when the entrypoint is invoked directly;
# a parent preview/promoter process injects its scoped credentials only into
# the single Wrangler mutation after certification returns.
while IFS='=' read -r environment_name _; do
	case "$environment_name" in
	CLOUDFLARE_* | CF_* | WRANGLER_*) unset "$environment_name" ;;
	esac
done < <(env)

DEFAULT_DIST="$SCRIPT_DIR/dist"
if [ -z "${YUNE_WEB06_CERTIFIED_ARCHIVE:-}" ] && [ ! -f "$DEFAULT_DIST/build-info.json" ]; then
	printf 'Missing source-identified public artifact at %s.\n' "$DEFAULT_DIST" >&2
	exit 1
fi

persistent_certification_root=
alias_temp=
aliased_default_dist=false
default_dist_existed=false
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
		if [ "$default_dist_existed" = true ] && [ ! -e "$DEFAULT_DIST" ]; then
			mv -- "$alias_temp/build-output-dist" "$DEFAULT_DIST" || cleanup_failed=true
		fi
	fi
	if [ -n "$alias_temp" ] && [ "$cleanup_failed" = false ]; then
		rm -rf -- "$alias_temp"
	fi
	if [ "$cleanup_failed" = true ] && [ "$exit_status" -eq 0 ]; then
		exit_status=1
	fi
	exit "$exit_status"
}
trap cleanup EXIT

new_persistent_external_root() {
	requested_root=$1
	kind=$2
	if [ -n "$requested_root" ]; then
		resolved_root=$(PYTHONDONTWRITEBYTECODE=1 python3 scripts/evidence-output-path.py validate \
			--repo-root "$YUNE_WEB_REPO_ROOT" --path "$requested_root")
	else
		resolved_root=$(PYTHONDONTWRITEBYTECODE=1 python3 scripts/evidence-output-path.py default \
			--repo-root "$YUNE_WEB_REPO_ROOT" --kind "$kind")
	fi
	if [ -e "$resolved_root" ]; then
		printf 'Persistent WEB06 certification root already exists: %s\n' "$resolved_root" >&2
		return 1
	fi
	mkdir -p "$(dirname -- "$resolved_root")"
	mkdir "$resolved_root"
	printf '%s\n' "$resolved_root"
}

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

configured_archive=${YUNE_WEB06_CERTIFIED_ARCHIVE:-}
configured_archive_sha256=${YUNE_WEB06_CERTIFIED_ARCHIVE_SHA256:-}
configured_dist_root=${YUNE_WEB06_CERTIFIED_DIST_ROOT:-}
if [ -n "$configured_archive" ] || [ -n "$configured_archive_sha256" ] || [ -n "$configured_dist_root" ]; then
	if [ -z "$configured_archive" ] || [ -z "$configured_archive_sha256" ] || [ -z "$configured_dist_root" ]; then
		printf 'YUNE_WEB06_CERTIFIED_ARCHIVE, YUNE_WEB06_CERTIFIED_ARCHIVE_SHA256, and YUNE_WEB06_CERTIFIED_DIST_ROOT must be supplied together.\n' >&2
		exit 1
	fi
	printf '%s\n' "$configured_archive_sha256" | grep -Eq '^[0-9a-f]{64}$' || {
		printf 'YUNE_WEB06_CERTIFIED_ARCHIVE_SHA256 must be a full lowercase SHA-256.\n' >&2
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
	certified_dist_input=$configured_dist_root
	certified_dist_root=$(CDPATH= cd -- "$configured_dist_root" && pwd -P)
	certified_archive_sha256=$actual_archive_sha256
else
	# The local compatibility entrypoint also follows the seal-once contract.
	# CI supplies the already-sealed archive and never enters this fallback. A
	# local fallback uses a persistent external root so a green or red run never
	# deletes the only sealed archive or its evidence.
	persistent_certification_root=$(new_persistent_external_root \
		"${YUNE_WEB_CERTIFICATION_ROOT:-}" web06-release-certification)
	printf 'WEB06 local certification will retain artifacts at %s\n' \
		"$persistent_certification_root" >&2
	configured_archive="$persistent_certification_root/yune-web-dist.tar.gz"
	COPYFILE_DISABLE=1 tar -C "$DEFAULT_DIST" -czf "$configured_archive" .
	certified_archive_sha256=$(archive_sha256 "$configured_archive")
	printf '%s\n' "$certified_archive_sha256" > "$configured_archive.sha256"
	safe_extract "$configured_archive" "$persistent_certification_root/extracted-dist"
	certified_dist_input=$persistent_certification_root/extracted-dist
	certified_dist_root=$(CDPATH= cd -- "$persistent_certification_root/extracted-dist" && pwd -P)
fi

expected_source_commit=${YUNE_WEB06_EXPECTED_SOURCE_COMMIT:-$(git rev-parse HEAD)}
printf '%s\n' "$expected_source_commit" | grep -Eq '^[0-9a-f]{40}$' || {
	printf 'YUNE_WEB06_EXPECTED_SOURCE_COMMIT must be a full lowercase Git SHA.\n' >&2
	exit 1
}
expected_source_tree=${YUNE_WEB06_EXPECTED_SOURCE_TREE:-}
if [ -z "$expected_source_tree" ]; then
	expected_source_tree=$(git rev-parse 'HEAD^{tree}')
fi
printf '%s\n' "$expected_source_tree" | grep -Eq '^[0-9a-f]{40}$' || {
	printf 'YUNE_WEB06_EXPECTED_SOURCE_TREE must be a full lowercase Git tree SHA.\n' >&2
	exit 1
}
actual_source_commit=$(git rev-parse HEAD)
actual_source_tree=$(git rev-parse 'HEAD^{tree}')
[ "$actual_source_commit" = "$expected_source_commit" ] || {
	printf 'Certification code source %s differs from candidate %s.\n' "$actual_source_commit" "$expected_source_commit" >&2
	exit 1
}
[ "$actual_source_tree" = "$expected_source_tree" ] || {
	printf 'Certification code tree %s differs from candidate %s.\n' "$actual_source_tree" "$expected_source_tree" >&2
	exit 1
}
[ -z "$(git status --porcelain --untracked-files=all)" ] || {
	printf 'Certification requires the exact clean candidate source tree.\n' >&2
	exit 1
}

if [ -z "${YUNE_WEB06_EVIDENCE_ROOT:-}" ]; then
	if [ -z "$persistent_certification_root" ]; then
		persistent_certification_root=$(new_persistent_external_root \
			"${YUNE_WEB_CERTIFICATION_ROOT:-}" web06-release-certification)
		printf 'WEB06 local certification will retain evidence at %s\n' \
			"$persistent_certification_root" >&2
	fi
	export YUNE_WEB06_EVIDENCE_ROOT="$persistent_certification_root/web06-evidence"
fi
YUNE_WEB06_EVIDENCE_ROOT=$(PYTHONDONTWRITEBYTECODE=1 python3 scripts/evidence-output-path.py validate \
	--repo-root "$YUNE_WEB_REPO_ROOT" --path "$YUNE_WEB06_EVIDENCE_ROOT")
export YUNE_WEB06_EVIDENCE_ROOT
mkdir -p "$YUNE_WEB06_EVIDENCE_ROOT"

export YUNE_WEB_EXPECTED_DIST="$certified_dist_root"
export YUNE_WEB_EXPECTED_SOURCE_COMMIT="$expected_source_commit"
export YUNE_WEB06_EXPECTED_SOURCE_COMMIT="$expected_source_commit"
export YUNE_WEB06_EXPECTED_SOURCE_TREE="$expected_source_tree"
export YUNE_WEB_LOCAL_ARTIFACT_RECEIPT=${YUNE_WEB_LOCAL_ARTIFACT_RECEIPT:-"$YUNE_WEB06_EVIDENCE_ROOT/local-artifact-verification.json"}
# WEB03 still consumes an extracted directory. Prove that directory is exactly
# the configured archive's file tree before either browser gate can use it.
PYTHONDONTWRITEBYTECODE=1 python3 \
	apps/yune-web/e2e/verify_archive_dist_identity.py \
	--archive "$configured_archive" \
	--dist "$certified_dist_input" \
	--expected-archive-sha256 "$certified_archive_sha256" \
	--receipt "$YUNE_WEB06_EVIDENCE_ROOT/archive-dist-identity.json"
node apps/yune-web/e2e/verify-local-artifact.mjs

# WEB03's public runner intentionally remains unchanged and resolves the fixed
# public-demo/dist path. Temporarily alias that ignored build output to the
# verified extracted archive so the legacy gate measures the sealed bytes.
default_dist_root=
if [ -d "$DEFAULT_DIST" ]; then
	default_dist_root=$(CDPATH= cd -- "$DEFAULT_DIST" && pwd -P)
fi
if [ "$default_dist_root" != "$certified_dist_root" ]; then
	alias_temp=$(mktemp -d "${TMPDIR:-/tmp}/yune-web06-dist-alias.XXXXXX")
	if [ -e "$DEFAULT_DIST" ]; then
		mv -- "$DEFAULT_DIST" "$alias_temp/build-output-dist"
		default_dist_existed=true
	fi
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
run_web03_gate() {
	npm --prefix apps/yune-web/e2e run test:e2e:input-latency:public
}

if [ "$certification_mode" = legacy ]; then
	# The classifier alone selects this mode for a range with no WEB06 marker.
	# It preserves the pre-WEB06 exact-artifact WEB03 gate and makes no WEB06
	# evidence, canary, milestone, or Full claim.
	run_web03_gate
	artifact_manifest_sha256=$(node -e 'const fs=require("node:fs");const crypto=require("node:crypto");const hash=crypto.createHash("sha256");hash.update(fs.readFileSync(process.argv[1]));process.stdout.write(hash.digest("hex"));' "$certified_dist_root/public-artifact-manifest.json")
	legacy_receipt=${YUNE_WEB_LEGACY_CERTIFICATION_RECEIPT:-"$YUNE_WEB06_EVIDENCE_ROOT/legacy-local-certification.json"}
	SOURCE_COMMIT="$expected_source_commit" \
		SOURCE_TREE="$expected_source_tree" \
		ARCHIVE_SHA256="$certified_archive_sha256" \
		ARTIFACT_MANIFEST_SHA256="$artifact_manifest_sha256" \
		RECEIPT="$legacy_receipt" \
		python3 - <<'PY'
import json
import os
from pathlib import Path

receipt = Path(os.environ["RECEIPT"])
receipt.parent.mkdir(parents=True, exist_ok=True)
payload = {
    "version": "yune-web-legacy-local-certification-v1",
    "operation": "legacy-exact-artifact-web03",
    "sourceCommit": os.environ["SOURCE_COMMIT"],
    "sourceTree": os.environ["SOURCE_TREE"],
    "archiveSha256": os.environ["ARCHIVE_SHA256"],
    "artifactManifestSha256": os.environ["ARTIFACT_MANIFEST_SHA256"],
    "web03Status": "passed",
    "web06EvidenceConsumed": False,
    "web06ClaimMade": False,
    "buildInvokedByCertification": False,
    "status": "passed",
}
with receipt.open("x", encoding="utf-8") as stream:
    json.dump(payload, stream, indent=2, sort_keys=True)
    stream.write("\n")
PY
	exit 0
fi

# WEB06 independently validates this exact archive and the source-bound
# FINAL/full attestation before any browser guard begins.
export YUNE_WEB06_CERTIFIED_ARCHIVE="$configured_archive"
export YUNE_WEB06_CERTIFIED_ARCHIVE_SHA256="$certified_archive_sha256"
preflight_root="$YUNE_WEB06_EVIDENCE_ROOT/final-preflight"
YUNE_WEB06_EVIDENCE_ROOT="$preflight_root" \
	YUNE_WEB06_PLAYWRIGHT_OUTPUT_DIR="$preflight_root/playwright" \
	YUNE_WEB06_GATE_SCOPE=release-certification \
	node apps/yune-web/e2e/run-public-web06-gate.mjs \
		--scope release-certification --verify-only

run_web03_gate

# Production-default compatibility uses selector omitted => minimal. It is a
# focused one-shot guard, not a second FINAL/full matrix.
focused_root="$YUNE_WEB06_EVIDENCE_ROOT/focused-default"
YUNE_WEB06_EVIDENCE_ROOT="$focused_root" \
	YUNE_WEB06_PLAYWRIGHT_OUTPUT_DIR="$focused_root/playwright" \
	YUNE_WEB06_GATE_SCOPE=release-certification \
	node apps/yune-web/e2e/run-public-web06-gate.mjs \
		--scope release-certification

artifact_manifest_sha256=$(node -e 'const fs=require("node:fs");const crypto=require("node:crypto");const hash=crypto.createHash("sha256");hash.update(fs.readFileSync(process.argv[1]));process.stdout.write(hash.digest("hex"));' "$certified_dist_root/public-artifact-manifest.json")
final_suite_attestation_sha256=$(archive_sha256 "$YUNE_WEB06_FINAL_SUITE_ATTESTATION")
local_certification_receipt=${YUNE_WEB06_LOCAL_CERTIFICATION_RECEIPT:-"$YUNE_WEB06_EVIDENCE_ROOT/web06-local-certification.json"}
SOURCE_COMMIT="$expected_source_commit" \
	SOURCE_TREE="$expected_source_tree" \
	ARCHIVE_SHA256="$certified_archive_sha256" \
	ARTIFACT_MANIFEST_SHA256="$artifact_manifest_sha256" \
	FINAL_SUITE_ATTESTATION_SHA256="$final_suite_attestation_sha256" \
	RECEIPT="$local_certification_receipt" \
	python3 - <<'PY'
import json
import os
from pathlib import Path

receipt = Path(os.environ["RECEIPT"])
receipt.parent.mkdir(parents=True, exist_ok=True)
payload = {
    "version": "web06-local-release-certification-v1",
    "operation": "local-no-build-certification",
    "sourceCommit": os.environ["SOURCE_COMMIT"],
    "sourceTree": os.environ["SOURCE_TREE"],
    "archiveSha256": os.environ["ARCHIVE_SHA256"],
    "artifactManifestSha256": os.environ["ARTIFACT_MANIFEST_SHA256"],
    "finalSuiteAttestationSha256": os.environ["FINAL_SUITE_ATTESTATION_SHA256"],
    "web03UnchangedStatus": "passed",
    "defaultMinimalCompatibilityStatus": "passed",
    "selectorPolicy": "omitted",
    "buildInvoked": False,
    "status": "passed",
}
with receipt.open("x", encoding="utf-8") as stream:
    json.dump(payload, stream, indent=2, sort_keys=True)
    stream.write("\n")
PY

if [ -n "$persistent_certification_root" ]; then
	printf 'Retained WEB06 certification archive and evidence at %s\n' \
		"$persistent_certification_root" >&2
fi
