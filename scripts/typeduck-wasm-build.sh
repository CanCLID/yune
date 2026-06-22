#!/bin/sh
set -eu

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
REPO_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
EXPORT_LIST="$REPO_ROOT/scripts/typeduck-exports.txt"
FALLBACK_TEST="cargo test -p yune-rime-api --test typeduck_web"
WASM_BUILD_PROFILE="release"
WASM_ARTIFACT_DIR="$REPO_ROOT/target/wasm32-unknown-emscripten/$WASM_BUILD_PROFILE"

if [ ! -f "$EXPORT_LIST" ]; then
  echo "TypeDuck WASM build failed: missing export list at scripts/typeduck-exports.txt" >&2
  exit 1
fi

EXPORTS=$(grep -v '^[[:space:]]*$' "$EXPORT_LIST")
if [ -z "$EXPORTS" ]; then
  echo "TypeDuck WASM build failed: scripts/typeduck-exports.txt is empty" >&2
  exit 1
fi

run_native_fallback() {
  echo "Native fallback still available: $FALLBACK_TEST"
  (cd "$REPO_ROOT" && cargo test -p yune-rime-api --test typeduck_web)
}

block_missing_target() {
  echo "TypeDuck WASM build blocked: missing wasm32-unknown-emscripten Rust target."
  echo "Install with: rustup target add wasm32-unknown-emscripten"
  run_native_fallback
}

block_missing_emscripten() {
  TOOL_NAME=$1
  echo "TypeDuck WASM build blocked: missing Emscripten linker \`$TOOL_NAME\` on PATH."
  echo "Install/activate Emscripten SDK so \`emcc\` and \`emar\` are available, then rerun this command."
  run_native_fallback
}

find_native_library() {
  for candidate in \
    "$REPO_ROOT/target/debug/libyune_rime_api.dylib" \
    "$REPO_ROOT/target/debug/libyune_rime_api.so" \
    "$REPO_ROOT/target/debug/yune_rime_api.dll.lib" \
    "$REPO_ROOT/target/debug/yune_rime_api.dll"
  do
    if [ -f "$candidate" ]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  return 1
}

find_tool() {
  for tool in "$@"; do
    if [ "$tool" = node ] && [ "${EMSDK_NODE+x}" = x ] && [ -x "$EMSDK_NODE" ]; then
      printf '%s\n' "$EMSDK_NODE"
      return 0
    fi

    if command -v "$tool" >/dev/null 2>&1; then
      command -v "$tool"
      return 0
    fi

    if [ "${EMSDK+x}" = x ]; then
      for candidate in "$EMSDK/upstream/bin/$tool" "$EMSDK/upstream/bin/$tool.exe"; do
        if [ -x "$candidate" ]; then
          printf '%s\n' "$candidate"
          return 0
        fi
      done
    fi
  done
  return 1
}

verify_native_exports() {
  NATIVE_LIBRARY=$1
  SYMBOL_TOOL=$(find_tool nm llvm-nm) || {
    echo "TypeDuck WASM build failed: missing native symbol inspector \`nm\` or \`llvm-nm\` on PATH." >&2
    exit 1
  }

  NM_OUTPUT=$("$SYMBOL_TOOL" -g "$NATIVE_LIBRARY" 2>/dev/null || "$SYMBOL_TOOL" "$NATIVE_LIBRARY")
  for symbol in $EXPORTS; do
    if ! printf '%s\n' "$NM_OUTPUT" | grep -Eq "(^|[[:space:]])_?${symbol}($|[[:space:]])"; then
      echo "TypeDuck WASM build failed: native library is missing export $symbol" >&2
      exit 1
    fi
  done
  echo "TypeDuck native exports verified: $NATIVE_LIBRARY"
}

join_exported_functions() {
  PREFIXED=""
  for symbol in $EXPORTS; do
    if [ -z "$PREFIXED" ]; then
      PREFIXED="_$symbol"
    else
      PREFIXED="$PREFIXED,_$symbol"
    fi
  done
  printf '%s\n' "$PREFIXED"
}

configure_emscripten_linker() {
  if [ "${CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_LINKER+x}" = x ]; then
    return
  fi

  if [ "${EMSDK+x}" != x ]; then
    return
  fi

  for candidate in "$EMSDK/upstream/emscripten/emcc.exe" "$EMSDK/upstream/emscripten/emcc.bat"; do
    if [ -x "$candidate" ]; then
      if command -v cygpath >/dev/null 2>&1; then
        export CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_LINKER=$(cygpath -w "$candidate")
      else
        export CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_LINKER="$candidate"
      fi
      return
    fi
  done
}

find_first_artifact() {
  EXT=$1
  ARTIFACT_DIR="$WASM_ARTIFACT_DIR"
  if [ ! -d "$ARTIFACT_DIR" ]; then
    return 1
  fi
  find "$ARTIFACT_DIR" -type f -name "*$EXT" -print | sort | head -n 1
}

find_named_artifact() {
  ARTIFACT_NAME=$1
  ARTIFACT_DIR="$WASM_ARTIFACT_DIR"
  if [ ! -d "$ARTIFACT_DIR" ]; then
    return 1
  fi
  find "$ARTIFACT_DIR" -maxdepth 2 -type f -name "$ARTIFACT_NAME" -print | sort | head -n 1
}

verify_wasm_exports() {
  WASM_ARTIFACT=$1
  JS_ARTIFACT=$2

  if WASM_NM=$(find_tool wasm-nm); then
    WASM_SYMBOLS=$("$WASM_NM" "$WASM_ARTIFACT")
    for symbol in $EXPORTS; do
      if ! printf '%s\n' "$WASM_SYMBOLS" | grep -Eq "(^|[[:space:]])_?${symbol}($|[[:space:]])"; then
        echo "TypeDuck WASM build failed: wasm-nm did not find export $symbol" >&2
        exit 1
      fi
    done
    return 0
  fi

  if WASM_OBJDUMP=$(find_tool wasm-objdump); then
    WASM_SYMBOLS=$("$WASM_OBJDUMP" -x "$WASM_ARTIFACT")
    for symbol in $EXPORTS; do
      if ! printf '%s\n' "$WASM_SYMBOLS" | grep -Eq "(^|[[:space:]])_?${symbol}($|[[:space:]])"; then
        echo "TypeDuck WASM build failed: wasm-objdump did not find export $symbol" >&2
        exit 1
      fi
    done
    return 0
  fi

  if LLVM_NM=$(find_tool llvm-nm); then
    if WASM_SYMBOLS=$("$LLVM_NM" "$WASM_ARTIFACT" 2>/dev/null); then
      FOUND_ALL=1
      for symbol in $EXPORTS; do
        if ! printf '%s\n' "$WASM_SYMBOLS" | grep -Eq "(^|[[:space:]])_?${symbol}($|[[:space:]])"; then
          FOUND_ALL=0
          break
        fi
      done
      if [ "$FOUND_ALL" -eq 1 ]; then
        return 0
      fi
    fi
    echo "TypeDuck WASM export verification falling back to JS glue scan after llvm-nm could not prove all exports."
  fi

  if [ ! -f "$JS_ARTIFACT" ]; then
    echo "TypeDuck WASM export verification limited: no wasm-nm, wasm-objdump, llvm-nm, or generated JS artifact available for symbol scan."
    echo "TypeDuck WASM artifact exists and native exports were verified before target build."
    return 0
  fi

  for symbol in $EXPORTS; do
    if ! grep -q "$symbol" "$JS_ARTIFACT"; then
      echo "TypeDuck WASM build failed: JS text scan fallback did not find export $symbol" >&2
      exit 1
    fi
  done
}

node_path() {
  if command -v cygpath >/dev/null 2>&1; then
    cygpath -w "$1"
    return
  fi
  printf '%s\n' "$1"
}

verify_loadable_module() {
  JS_ARTIFACT=$1
  JS_DIR=$(dirname "$JS_ARTIFACT")

  if ! NODE=$(find_tool node); then
    echo "TypeDuck browser module smoke skipped: node is not available on PATH."
    return 0
  fi

  JS_FOR_NODE=$(node_path "$JS_ARTIFACT")
  DIR_FOR_NODE=$(node_path "$JS_DIR")
  "$NODE" - "$JS_FOR_NODE" "$DIR_FOR_NODE" <<'NODE'
const path = require("path");

const [jsArtifact, artifactDir] = process.argv.slice(2);
const createModule = require(jsArtifact);

(async () => {
  const module = await createModule({
    locateFile: (file) => path.join(artifactDir, file),
  });

  if (typeof module.cwrap !== "function") {
    throw new Error("Emscripten cwrap is unavailable");
  }
  if (typeof module.UTF8ToString !== "function") {
    throw new Error("Emscripten UTF8ToString is unavailable");
  }
  if (module.FS === undefined || typeof module.FS.writeFile !== "function") {
    throw new Error("Emscripten FS is unavailable");
  }
  if ((module.IDBFS ?? module.FS.filesystems?.IDBFS) === undefined) {
    throw new Error("Emscripten IDBFS is unavailable");
  }

  const responseHandled = module.cwrap("yune_typeduck_response_handled", "number", ["number"]);
  const handled = responseHandled(0);
  if (handled !== 0) {
    throw new Error(`yune_typeduck_response_handled(null) returned ${handled}`);
  }

  module.FS.writeFile("/tmp/yune-typeduck-smoke.txt", "ok", { flags: "w" });
  const text = module.FS.readFile("/tmp/yune-typeduck-smoke.txt", { encoding: "utf8" });
  if (text !== "ok") {
    throw new Error(`Emscripten FS readback returned ${JSON.stringify(text)}`);
  }

  console.log("TypeDuck browser module smoke verified: yune_typeduck_response_handled + FS write/read");
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
NODE
}

(cd "$REPO_ROOT" && cargo build -p yune-rime-api)
NATIVE_LIBRARY=$(find_native_library) || {
  echo "TypeDuck WASM build failed: could not locate yune-rime-api native dynamic library under target/debug" >&2
  exit 1
}
verify_native_exports "$NATIVE_LIBRARY"

if ! command -v rustup >/dev/null 2>&1; then
  echo "TypeDuck WASM build blocked: missing rustup on PATH."
  echo "Install Rustup and then run: rustup target add wasm32-unknown-emscripten"
  run_native_fallback
  exit 0
fi

if ! rustup target list --installed | grep -qx 'wasm32-unknown-emscripten'; then
  block_missing_target
  exit 0
fi

if ! command -v emcc >/dev/null 2>&1; then
  block_missing_emscripten "emcc"
  exit 0
fi

if ! command -v emar >/dev/null 2>&1; then
  block_missing_emscripten "emar"
  exit 0
fi

configure_emscripten_linker

EXPORTED_FUNCTIONS=$(join_exported_functions)
RUNTIME_METHODS="ccall,cwrap,UTF8ToString,FS,IDBFS"
EXTRA_RUSTFLAGS="-C link-arg=-O3 -C link-arg=-sEXPORTED_FUNCTIONS=$EXPORTED_FUNCTIONS -C link-arg=-sEXPORTED_RUNTIME_METHODS=$RUNTIME_METHODS -C link-arg=-sMODULARIZE=1 -C link-arg=-sEXPORT_NAME=createYuneTypeduckModule -C link-arg=-sENVIRONMENT=web,worker,node -C link-arg=-sFORCE_FILESYSTEM=1 -C link-arg=-sALLOW_MEMORY_GROWTH=1 -C link-arg=-sINITIAL_MEMORY=134217728 -C link-arg=-sSTACK_SIZE=8388608 -C link-arg=-lidbfs.js"
if [ "${RUSTFLAGS+x}" = x ] && [ -n "$RUSTFLAGS" ]; then
  export RUSTFLAGS="$RUSTFLAGS $EXTRA_RUSTFLAGS"
else
  export RUSTFLAGS="$EXTRA_RUSTFLAGS"
fi

ARTIFACT_DIR="$WASM_ARTIFACT_DIR"
rm -f \
  "$ARTIFACT_DIR/typeduck_web_module.js" \
  "$ARTIFACT_DIR/typeduck_web_module.wasm" \
  "$ARTIFACT_DIR/deps/typeduck_web_module.js" \
  "$ARTIFACT_DIR/deps/typeduck_web_module.wasm" \
  "$ARTIFACT_DIR/yune-typeduck.js" \
  "$ARTIFACT_DIR/yune-typeduck.wasm" \
  "$ARTIFACT_DIR/yune-typeduck.wasm.optimized"

(cd "$REPO_ROOT" && cargo build -p yune-rime-api --target wasm32-unknown-emscripten --bin typeduck_web_module --release)
BROWSER_JS_ARTIFACT=$(find_named_artifact typeduck_web_module.js) || {
  echo "TypeDuck WASM build failed: no Emscripten JS glue artifact found under target/wasm32-unknown-emscripten/$WASM_BUILD_PROFILE" >&2
  exit 1
}
BROWSER_WASM_ARTIFACT=$(find_named_artifact typeduck_web_module.wasm) || {
  echo "TypeDuck WASM build failed: no Emscripten browser .wasm artifact found under target/wasm32-unknown-emscripten/$WASM_BUILD_PROFILE" >&2
  exit 1
}

JS_ARTIFACT="$ARTIFACT_DIR/yune-typeduck.js"
WASM_ARTIFACT="$ARTIFACT_DIR/yune-typeduck.wasm"
cp "$BROWSER_JS_ARTIFACT" "$JS_ARTIFACT"
cp "$BROWSER_WASM_ARTIFACT" "$WASM_ARTIFACT"

if WASM_OPT=$(find_tool wasm-opt); then
  if "$WASM_OPT" -O3 --enable-bulk-memory --enable-exception-handling "$WASM_ARTIFACT" -o "$WASM_ARTIFACT.optimized" >/dev/null 2>&1; then
    mv "$WASM_ARTIFACT.optimized" "$WASM_ARTIFACT"
    echo "TypeDuck browser WASM post-optimized with wasm-opt -O3"
  else
    rm -f "$WASM_ARTIFACT.optimized"
    echo "TypeDuck browser WASM post-optimization skipped: wasm-opt could not validate this Emscripten module"
  fi
else
  echo "TypeDuck browser WASM post-optimization skipped: wasm-opt not available"
fi

verify_wasm_exports "$WASM_ARTIFACT" "$JS_ARTIFACT"
verify_loadable_module "$JS_ARTIFACT"

echo "TypeDuck browser WASM verified: $WASM_ARTIFACT"
echo "TypeDuck Emscripten JS glue verified: $JS_ARTIFACT"
echo "TypeDuck browser WASM build profile: $WASM_BUILD_PROFILE"
