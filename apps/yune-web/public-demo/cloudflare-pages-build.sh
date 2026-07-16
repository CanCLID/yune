#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)

# Compatibility entrypoint for local release reproduction. Cloudflare Pages no
# longer runs this command: CI certifies the artifact before direct upload.
bash "$SCRIPT_DIR/build-public-release.sh"
bash "$SCRIPT_DIR/certify-public-release.sh"
