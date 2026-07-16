#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
# shellcheck disable=SC1091
. "$SCRIPT_DIR/public-release-toolchain.sh"

WASM_ARTIFACT_DIR="$YUNE_WEB_REPO_ROOT/target/wasm32-unknown-emscripten/release"

ensure_artifact() {
	artifact=$1
	if [ ! -f "$artifact" ]; then
		echo "Missing expected Yune web WASM artifact: $artifact" >&2
		exit 1
	fi
}

cd "$YUNE_WEB_REPO_ROOT"
yune_web_activate_release_toolchain

npm --prefix packages/yune-web-runtime ci
npm --prefix apps/yune-web ci

export YUNE_WEB_WASM_REQUIRE_EMSCRIPTEN=1
scripts/yune-web-wasm-build.sh
ensure_artifact "$WASM_ARTIFACT_DIR/yune-web.js"
ensure_artifact "$WASM_ARTIFACT_DIR/yune-web.wasm"

cp "$WASM_ARTIFACT_DIR/yune-web.js" "$YUNE_WEB_APP_ROOT/public/yune-web.js"
cp "$WASM_ARTIFACT_DIR/yune-web.wasm" "$YUNE_WEB_APP_ROOT/public/yune-web.wasm"

npm --prefix apps/yune-web run build:public
