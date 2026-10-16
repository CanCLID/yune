#!/usr/bin/env python3
"""Classify whether a commit range can change yune-web release bytes or gates."""

from __future__ import annotations

import argparse
import os
import re
import subprocess
from pathlib import Path, PurePosixPath


COPIED_PUBLIC_MARKDOWN = {
    "apps/yune-web/public-demo/README.md",
    "apps/yune-web/public-demo/PROVENANCE.md",
    "apps/yune-web/public-demo/asset-manifest.md",
    "apps/yune-web/public-demo/cache-policy.md",
}

ROOT_RELEASE_PATHS = {
    ".gitattributes",
    ".github/workflows/deploy-yune-web.yml",
    "Cargo.lock",
    "Cargo.toml",
    "scripts/classify_yune_web_release.py",
    "scripts/yune-web-exports.txt",
    "scripts/yune-web-wasm-build.sh",
}

# A push whose own range (github.event.before..github.sha) adds, edits, or
# deletes a milestone marker (WEB06_GOVERNED_PREFIXES) routes to the WEB06
# handoff no-op instead of the legacy build-on-push lane. WEB-06's binding
# browser evidence is attached to a sealed local archive, and GitHub Actions has
# no reviewed immutable handoff for those bytes. The marker check is per push,
# not per deployed range: the next push without a marker builds HEAD, including
# earlier marker bytes, through the legacy lane, which makes no WEB06 claim.
WEB06_GOVERNED_PATHS = {
    ".github/workflows/deploy-yune-web.yml",
    "apps/yune-web/e2e/package.json",
    "apps/yune-web/package-lock.json",
    "apps/yune-web/package.json",
    "apps/yune-web/e2e/production-artifact-verifier.mjs",
    "apps/yune-web/e2e/production-artifact-verifier.test.mjs",
    "apps/yune-web/e2e/public-artifact-verifier.test.mjs",
    "apps/yune-web/e2e/test_verify_archive_dist_identity.py",
    "apps/yune-web/e2e/verify-deployed-artifact.mjs",
    "apps/yune-web/e2e/verify_archive_dist_identity.py",
    "apps/yune-web/public-demo/certify-public-release.sh",
    "scripts/tests/test_yune_web_release_policy.py",
    "scripts/classify_yune_web_release.py",
    "apps/yune-web/src/App.tsx",
    "apps/yune-web/src/Candidate.tsx",
    "apps/yune-web/src/CandidatePanel.tsx",
    "apps/yune-web/src/YuneControlSurface.tsx",
    "apps/yune-web/src/YuneUserdbViewer.tsx",
    "apps/yune-web/src/rime.ts",
    "apps/yune-web/src/types.ts",
    "apps/yune-web/src/worker.ts",
    "apps/yune-web/src/yune-integration/adapter.ts",
    "apps/yune-web/src/yune-integration/private-protocol.ts",
    "apps/yune-web/yune-integration/private-protocol.test.ts",
    "apps/yune-web/yune-integration/web06-private-pipeline.test.ts",
    "apps/yune-web/e2e/startup-benchmark/comparator-browser-endpoint.ts",
    "apps/yune-web/e2e/startup-benchmark/comparator-endpoint.ts",
    "packages/yune-web-runtime/src/observation.ts",
    "packages/yune-web-runtime/src/response.ts",
    "packages/yune-web-runtime/src/runtime.ts",
    "packages/yune-web-runtime/test/fake-module.ts",
    "packages/yune-web-runtime/test/observation.test.ts",
    "packages/yune-web-runtime/test/public-api.test.ts",
}
WEB06_GOVERNED_PREFIXES = (
    "apps/yune-web/e2e/playwright.web06",
    "apps/yune-web/e2e/run-public-web06-",
    "apps/yune-web/e2e/run-web06-",
    "apps/yune-web/e2e/startup-benchmark/web06-",
    "apps/yune-web/e2e/verify-web06-",
    "apps/yune-web/e2e/web06-",
    "apps/yune-web/e2e/yune-web06-",
    "apps/yune-web/src/web06-",
    "apps/yune-web/src/yune-integration/web06-",
    "apps/yune-web/yune-integration/web06-",
)


def requires_release(path: str) -> bool:
    normalized = PurePosixPath(path).as_posix()
    if normalized.startswith("./"):
        normalized = normalized[2:]
    if normalized in ROOT_RELEASE_PATHS or normalized in COPIED_PUBLIC_MARKDOWN:
        return True
    # Vite copies this entire directory verbatim, including Markdown assets.
    if normalized.startswith("apps/yune-web/public/"):
        return True
    if normalized.endswith(".md"):
        return False
    if normalized.startswith("packages/yune-web-runtime/"):
        return True
    if normalized.startswith(("crates/yune-core/", "crates/yune-rime-api/")):
        return True
    if not normalized.startswith("apps/yune-web/"):
        return False
    if normalized.startswith(
        (
            "apps/yune-web/e2e/results/",
            "apps/yune-web/patches/",
            "apps/yune-web/source/",
        )
    ):
        return False
    return True


def is_web06_governed_path(path: str) -> bool:
    normalized = PurePosixPath(path).as_posix()
    if normalized.startswith("./"):
        normalized = normalized[2:]
    return normalized in WEB06_GOVERNED_PATHS or normalized.startswith(
        WEB06_GOVERNED_PREFIXES
    )


def is_web06_handoff_marker(path: str) -> bool:
    """Return true only for milestone-unique paths, not reusable app owners."""

    normalized = PurePosixPath(path).as_posix()
    if normalized.startswith("./"):
        normalized = normalized[2:]
    return normalized.startswith(WEB06_GOVERNED_PREFIXES)


def changed_paths(repo_root: Path, base: str, head: str) -> list[str]:
    result = subprocess.run(
        [
            "git",
            "diff",
            "--no-renames",
            "--name-only",
            "-z",
            "--diff-filter=ACDMRTUXB",
            f"{base}..{head}",
        ],
        cwd=repo_root,
        check=True,
        stdout=subprocess.PIPE,
    )
    return [os.fsdecode(path) for path in result.stdout.split(b"\0") if path]


def usable_marker_base(repo_root: Path, candidate: str, head: str) -> bool:
    """A push's before SHA counts only if it is a real ancestor of head.

    Branch creation (all zeros), a force push whose old tip is gone or not an
    ancestor, or a missing value all fail this check.
    """

    if not re.fullmatch(r"[0-9a-f]{40}", candidate) or candidate == "0" * 40:
        return False
    return (
        subprocess.run(
            ["git", "merge-base", "--is-ancestor", candidate, head],
            cwd=repo_root,
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        ).returncode
        == 0
    )


def output_safe(value: str) -> str:
    """Keep one untrusted Git path from injecting a GitHub output record."""

    return value.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", required=True)
    # The push's github.event.before. Unusable values fall back to --base.
    parser.add_argument("--marker-base", default="")
    parser.add_argument("--github-output", type=Path)
    args = parser.parse_args()

    repo_root = args.repo_root.resolve()
    paths = changed_paths(repo_root, args.base, args.head)
    release_paths = [path for path in paths if requires_release(path)]
    marker_base, fallback = args.marker_base, ""
    if not usable_marker_base(repo_root, marker_base, args.head):
        marker_base = args.base
        fallback = " (push before SHA unusable; fell back to release base)"
    marker_range = f"{marker_base}..{args.head}{fallback}"
    web06_markers = [
        path
        for path in changed_paths(repo_root, marker_base, args.head)
        if requires_release(path) and is_web06_handoff_marker(path)
    ]
    required = bool(release_paths)
    reason = (
        f"{len(release_paths)} release-affecting path(s): "
        f"{', '.join(output_safe(path) for path in release_paths[:12])}"
        if required
        else f"no release-affecting paths among {len(paths)} changed path(s)"
    )
    web06_reason = (
        f"{len(web06_markers)} milestone marker(s) in push range {marker_range}: "
        f"{', '.join(output_safe(path) for path in web06_markers[:12])}"
        if web06_markers
        else f"no WEB06 milestone markers in push range {marker_range}"
    )
    lines = [
        f"release_required={'true' if required else 'false'}",
        f"reason={reason}",
        f"web06_handoff_required={'true' if web06_markers else 'false'}",
        f"web06_reason={web06_reason}",
    ]
    print("\n".join(lines))
    if args.github_output is not None:
        with args.github_output.open("a", encoding="utf-8") as output:
            output.write("".join(f"{line}\n" for line in lines))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
