#!/usr/bin/env bash

YUNE_WEB_RELEASE_HELPER_DIR=$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
YUNE_WEB_APP_ROOT=$(CDPATH= cd -- "$YUNE_WEB_RELEASE_HELPER_DIR/.." && pwd)
YUNE_WEB_REPO_ROOT=$(CDPATH= cd -- "$YUNE_WEB_APP_ROOT/../.." && pwd)

YUNE_WEB_EMSDK_VERSION=4.0.23
YUNE_WEB_EMSCRIPTEN_RELEASE_COMMIT=aaa43392544d695232b70eda706d751f18980c2a
YUNE_WEB_EMSDK_REPOSITORY_COMMIT=db04e88298d9916fc51fcd3743045ca3eb695127
YUNE_WEB_RUST_TOOLCHAIN_VERSION=1.96.1
YUNE_WEB_EXPECTED_RUSTC_VERSION="rustc 1.96.1 (31fca3adb 2026-06-26)"
YUNE_WEB_EXPECTED_NODE_VERSION=v22.16.0
YUNE_WEB_EMSDK_DIR=${YUNE_WEB_EMSDK_DIR:-"$YUNE_WEB_REPO_ROOT/.cache/emsdk"}

yune_web_ensure_rustup() {
	if command -v rustup >/dev/null 2>&1; then
		return
	fi

	curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
	# shellcheck disable=SC1091
	. "$HOME/.cargo/env"
}

yune_web_ensure_emscripten() {
	if [ ! -d "$YUNE_WEB_EMSDK_DIR/.git" ]; then
		rm -rf "$YUNE_WEB_EMSDK_DIR"
		mkdir -p "$(dirname "$YUNE_WEB_EMSDK_DIR")"
		git clone --depth 1 https://github.com/emscripten-core/emsdk.git "$YUNE_WEB_EMSDK_DIR"
	fi
	git -C "$YUNE_WEB_EMSDK_DIR" fetch --depth 1 origin "$YUNE_WEB_EMSDK_REPOSITORY_COMMIT"
	git -C "$YUNE_WEB_EMSDK_DIR" checkout --detach "$YUNE_WEB_EMSDK_REPOSITORY_COMMIT"

	resolved_release=$(node -e 'const fs=require("fs"); const tags=JSON.parse(fs.readFileSync(process.argv[1], "utf8")); process.stdout.write(tags.releases[process.argv[2]] ?? "");' "$YUNE_WEB_EMSDK_DIR/emscripten-releases-tags.json" "$YUNE_WEB_EMSDK_VERSION")
	if [ "$resolved_release" != "$YUNE_WEB_EMSCRIPTEN_RELEASE_COMMIT" ]; then
		echo "Pinned Emscripten $YUNE_WEB_EMSDK_VERSION resolved to '$resolved_release', expected $YUNE_WEB_EMSCRIPTEN_RELEASE_COMMIT." >&2
		exit 1
	fi

	(cd "$YUNE_WEB_EMSDK_DIR" && ./emsdk install "$YUNE_WEB_EMSDK_VERSION" && ./emsdk activate "$YUNE_WEB_EMSDK_VERSION")

	pushd "$YUNE_WEB_EMSDK_DIR" >/dev/null
	# shellcheck disable=SC1091
	. ./emsdk_env.sh >/dev/null
	popd >/dev/null

	if ! command -v emcc >/dev/null 2>&1 || ! command -v emar >/dev/null 2>&1; then
		echo "Emscripten SDK was installed but emcc/emar were not activated on PATH." >&2
		exit 1
	fi

	YUNE_WEB_EMCC_VERSION=$(emcc --version | sed -n '1p')
	if ! printf '%s\n' "$YUNE_WEB_EMCC_VERSION" | grep -Fq "$YUNE_WEB_EMSDK_VERSION"; then
		echo "Active emcc does not report pinned version $YUNE_WEB_EMSDK_VERSION: $YUNE_WEB_EMCC_VERSION" >&2
		exit 1
	fi
	active_emsdk_repository_commit=$(git -C "$YUNE_WEB_EMSDK_DIR" rev-parse HEAD)
	if [ "$active_emsdk_repository_commit" != "$YUNE_WEB_EMSDK_REPOSITORY_COMMIT" ]; then
		echo "Active emsdk repository is $active_emsdk_repository_commit, expected $YUNE_WEB_EMSDK_REPOSITORY_COMMIT." >&2
		exit 1
	fi
}

yune_web_activate_release_toolchain() {
	if [ -n "${RUSTFLAGS:-}" ] || [ -n "${CARGO_ENCODED_RUSTFLAGS:-}" ]; then
		echo "Public release builds reject ambient RUSTFLAGS/CARGO_ENCODED_RUSTFLAGS because they change WASM bytes outside the pinned receipt." >&2
		exit 1
	fi

	yune_web_ensure_rustup
	rustup toolchain install "$YUNE_WEB_RUST_TOOLCHAIN_VERSION" --profile minimal
	export RUSTUP_TOOLCHAIN="$YUNE_WEB_RUST_TOOLCHAIN_VERSION"
	rustup target add wasm32-unknown-emscripten
	yune_web_ensure_emscripten

	if [ -z "${EMSDK_NODE:-}" ] || [ ! -x "$EMSDK_NODE" ]; then
		echo "Pinned Emscripten SDK did not expose its Node runtime: ${EMSDK_NODE:-unset}" >&2
		exit 1
	fi
	export PATH="$(dirname "$EMSDK_NODE"):$PATH"

	if [ "$(rustc --version)" != "$YUNE_WEB_EXPECTED_RUSTC_VERSION" ]; then
		echo "Active rustc does not match the public-build pin: $(rustc --version)" >&2
		exit 1
	fi
	if [ "$(node --version)" != "$YUNE_WEB_EXPECTED_NODE_VERSION" ]; then
		echo "Active Node does not match the Emscripten public-build pin: $(node --version)" >&2
		exit 1
	fi

	export YUNE_WEB_REQUIRE_TOOLCHAIN_RECEIPT=1
	export YUNE_WEB_EMSDK_VERSION
	export YUNE_WEB_EMSCRIPTEN_RELEASE_COMMIT
	export YUNE_WEB_EMSDK_REPOSITORY_COMMIT
	export YUNE_WEB_EMCC_VERSION
	export YUNE_WEB_RUSTC_VERSION="$(rustc --version)"
	export YUNE_WEB_NODE_VERSION="$(node --version)"
}
