#!/usr/bin/env python3
"""Classify whether a commit range can change yune-web release bytes or gates."""

from __future__ import annotations

import argparse
import os
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
    "scripts/yune-web-exports.txt",
    "scripts/yune-web-wasm-build.sh",
}


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


def output_safe(value: str) -> str:
    """Keep one untrusted Git path from injecting a GitHub output record."""

    return value.replace("%", "%25").replace("\r", "%0D").replace("\n", "%0A")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo-root", type=Path, default=Path.cwd())
    parser.add_argument("--base", required=True)
    parser.add_argument("--head", required=True)
    parser.add_argument("--github-output", type=Path)
    args = parser.parse_args()

    paths = changed_paths(args.repo_root.resolve(), args.base, args.head)
    release_paths = [path for path in paths if requires_release(path)]
    required = bool(release_paths)
    reason = (
        f"{len(release_paths)} release-affecting path(s): "
        f"{', '.join(output_safe(path) for path in release_paths[:12])}"
        if required
        else f"no release-affecting paths among {len(paths)} changed path(s)"
    )
    print(f"release_required={'true' if required else 'false'}")
    print(f"reason={reason}")
    if args.github_output is not None:
        with args.github_output.open("a", encoding="utf-8") as output:
            output.write(f"release_required={'true' if required else 'false'}\n")
            output.write(f"reason={reason}\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
