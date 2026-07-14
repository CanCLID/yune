#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
APP_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
REPO_ROOT=$(CDPATH= cd -- "$APP_ROOT/../.." && pwd)
EMSDK_VERSION=4.0.23
EMSCRIPTEN_RELEASE_COMMIT=aaa43392544d695232b70eda706d751f18980c2a
EMSDK_REPOSITORY_COMMIT=db04e88298d9916fc51fcd3743045ca3eb695127
EMSDK_DIR=${YUNE_WEB_EMSDK_DIR:-"$REPO_ROOT/.cache/emsdk"}
WASM_ARTIFACT_DIR="$REPO_ROOT/target/wasm32-unknown-emscripten/release"
PLAYWRIGHT_BROWSERS_PATH=${PLAYWRIGHT_BROWSERS_PATH:-"$REPO_ROOT/.cache/ms-playwright"}
export PLAYWRIGHT_BROWSERS_PATH

ensure_rustup() {
	if command -v rustup >/dev/null 2>&1; then
		return
	fi

	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
	# shellcheck disable=SC1091
	. "$HOME/.cargo/env"
}

ensure_emscripten() {
	if [ ! -d "$EMSDK_DIR/.git" ]; then
		rm -rf "$EMSDK_DIR"
		mkdir -p "$(dirname "$EMSDK_DIR")"
		git clone --depth 1 https://github.com/emscripten-core/emsdk.git "$EMSDK_DIR"
	fi
	git -C "$EMSDK_DIR" fetch --depth 1 origin "$EMSDK_REPOSITORY_COMMIT"
	git -C "$EMSDK_DIR" checkout --detach "$EMSDK_REPOSITORY_COMMIT"

	resolved_release=$(node -e 'const fs=require("fs"); const tags=JSON.parse(fs.readFileSync(process.argv[1], "utf8")); process.stdout.write(tags.releases[process.argv[2]] ?? "");' "$EMSDK_DIR/emscripten-releases-tags.json" "$EMSDK_VERSION")
	if [ "$resolved_release" != "$EMSCRIPTEN_RELEASE_COMMIT" ]; then
		echo "Pinned Emscripten $EMSDK_VERSION resolved to '$resolved_release', expected $EMSCRIPTEN_RELEASE_COMMIT." >&2
		exit 1
	fi

	(cd "$EMSDK_DIR" && ./emsdk install "$EMSDK_VERSION" && ./emsdk activate "$EMSDK_VERSION")

	pushd "$EMSDK_DIR" >/dev/null
	# shellcheck disable=SC1091
	. ./emsdk_env.sh >/dev/null
	popd >/dev/null

	if ! command -v emcc >/dev/null 2>&1 || ! command -v emar >/dev/null 2>&1; then
		echo "Emscripten SDK was installed but emcc/emar were not activated on PATH." >&2
		exit 1
	fi

	EMCC_VERSION=$(emcc --version | sed -n '1p')
	if ! printf '%s\n' "$EMCC_VERSION" | grep -Fq "$EMSDK_VERSION"; then
		echo "Active emcc does not report pinned version $EMSDK_VERSION: $EMCC_VERSION" >&2
		exit 1
	fi
	active_emsdk_repository_commit=$(git -C "$EMSDK_DIR" rev-parse HEAD)
	if [ "$active_emsdk_repository_commit" != "$EMSDK_REPOSITORY_COMMIT" ]; then
		echo "Active emsdk repository is $active_emsdk_repository_commit, expected $EMSDK_REPOSITORY_COMMIT." >&2
		exit 1
	fi
}

ensure_artifact() {
	artifact=$1
	if [ ! -f "$artifact" ]; then
		echo "Missing expected Yune web WASM artifact: $artifact" >&2
		exit 1
	fi
}

cd "$REPO_ROOT"

ensure_rustup
rustup target add wasm32-unknown-emscripten
ensure_emscripten

export YUNE_WEB_REQUIRE_TOOLCHAIN_RECEIPT=1
export YUNE_WEB_EMSDK_VERSION="$EMSDK_VERSION"
export YUNE_WEB_EMSCRIPTEN_RELEASE_COMMIT="$EMSCRIPTEN_RELEASE_COMMIT"
export YUNE_WEB_EMSDK_REPOSITORY_COMMIT="$EMSDK_REPOSITORY_COMMIT"
export YUNE_WEB_EMCC_VERSION="$EMCC_VERSION"
export YUNE_WEB_RUSTC_VERSION="$(rustc --version)"
export YUNE_WEB_NODE_VERSION="$(node --version)"

npm --prefix packages/yune-web-runtime ci
npm --prefix apps/yune-web ci
npm --prefix apps/yune-web/e2e ci
node apps/yune-web/e2e/node_modules/@playwright/test/cli.js install chromium

export YUNE_WEB_WASM_REQUIRE_EMSCRIPTEN=1
scripts/yune-web-wasm-build.sh
ensure_artifact "$WASM_ARTIFACT_DIR/yune-web.js"
ensure_artifact "$WASM_ARTIFACT_DIR/yune-web.wasm"

cp "$WASM_ARTIFACT_DIR/yune-web.js" "$APP_ROOT/public/yune-web.js"
cp "$WASM_ARTIFACT_DIR/yune-web.wasm" "$APP_ROOT/public/yune-web.wasm"

npm --prefix apps/yune-web run build:public

# Cloudflare publishes only after this source-current public artifact passes.
# This is intentionally the focused latency matrix, not the broad browser suite.
export YUNE_WEB_LATENCY_CPU_THROTTLE=4
export YUNE_WEB_LATENCY_KEY_INTERVAL_MS=250
export YUNE_WEB_LATENCY_P95_MS=750
export YUNE_WEB_LATENCY_MAX_MS=1000
npm --prefix apps/yune-web/e2e run test:e2e:input-latency:public
