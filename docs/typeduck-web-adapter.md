# TypeDuck-Web Adapter

Yune exposes a small TypeDuck-Web-shaped C/WASM bridge from `yune-rime-api`. It is a facade over the existing `RimeApi` lifecycle, not a vendored TypeDuck-Web fork or a complete browser package.

## Exported symbols

The adapter exports prefixed symbols so it does not collide with librime-style ABI names:

- `yune_typeduck_init(shared_data_dir, user_data_dir, schema_id) -> *mut YuneTypeDuckState`
- `yune_typeduck_process_key(state, keycode, mask) -> *mut YuneTypeDuckResponse`
- `yune_typeduck_select_candidate(state, index) -> *mut YuneTypeDuckResponse`
- `yune_typeduck_delete_candidate(state, index) -> *mut YuneTypeDuckResponse`
- `yune_typeduck_flip_page(state, backward) -> *mut YuneTypeDuckResponse`
- `yune_typeduck_customize(state, config_id, key, value) -> Bool`
- `yune_typeduck_deploy(state) -> Bool`
- `yune_typeduck_cleanup(state)`
- `yune_typeduck_response_json(response) -> *const c_char`
- `yune_typeduck_response_handled(response) -> Bool`
- `yune_typeduck_free_response(response)`

`backward != 0` flips to the previous page. `backward == 0` flips to the next page. Candidate indices are page-relative.

## Response ownership

Operations that return `YuneTypeDuckResponse` allocate an owned response. Browser or JS glue should copy the JSON string immediately and then call `yune_typeduck_free_response`.

`yune_typeduck_free_response(NULL)` is a no-op. `yune_typeduck_response_json(NULL)` returns `NULL`, and `yune_typeduck_response_handled(NULL)` returns `FALSE`.

## JSON response shape

Responses contain:

```json
{
  "handled": true,
  "commits": ["吧"],
  "context": {
    "input": "ba",
    "preedit": "ba",
    "caret": 2,
    "highlighted": 0,
    "page_size": 5,
    "page_no": 0,
    "is_last_page": false,
    "select_keys": "12345",
    "select_labels": [],
    "candidates": [
      { "text": "八", "comment": "" }
    ]
  },
  "status": {
    "schema_id": "typeduck_luna",
    "schema_name": "TypeDuck Luna",
    "is_disabled": false,
    "is_composing": true,
    "is_ascii_mode": false,
    "is_full_shape": false,
    "is_simplified": false,
    "is_traditional": false,
    "is_ascii_punct": false
  }
}
```

If an operation cannot capture normal state, the response may include an `error` string.

## Lifecycle constraints

The adapter exposes one active process-global Yune/RIME service. Browser callers should treat the pointer returned by `yune_typeduck_init` as the single live TypeDuck state for the current Module instance.

`yune_typeduck_cleanup` destroys the adapter session and finalizes the process-global RIME service. A later init may create a new service, but multiple simultaneous TypeDuck states with different shared/user directories are unsupported by this Phase 7 contract.

## Browser filesystem contract

The Rust adapter only receives C string paths. The JS/Emscripten host is responsible for creating and syncing the virtual filesystem.

Expected layout before calling `yune_typeduck_init`:

- `shared_data_dir`: deploy source files such as `default.yaml`, `<schema>.schema.yaml`, and `<dict>.dict.yaml`.
- `user_data_dir`: user state, custom patches, userdb data, and the deployed `build/` directory.
- `user_data_dir/build`: deployed or preloaded runtime configs used by schema selection and key processing.

For Emscripten, TypeDuck-Web glue should mount MEMFS/IDBFS before calling `yune_typeduck_init`. The schema/dictionary assets must be preloaded into the virtual filesystem before init; Phase 7 native fallback tests require init to fail deterministically when those assets are missing rather than fabricating placeholder browser data.

The persistence sync remains a JS host responsibility until Phase 9. Browser code should sync persistent storage before init and after deploy/customize or userdb-changing flows, but this Rust adapter does not mount IDBFS, choose storage policy, or hide sync failures.

## WASM build/export contract

The intended browser build target is `wasm32-unknown-emscripten`. Phase 7 keeps the adapter in `crates/yune-rime-api`; the crate already builds as `rlib`/`cdylib`, and `src/lib.rs` should remain facade wiring for `typeduck_web` rather than owned browser-build logic. This contract does not require upstream TypeDuck-Web source access; replacing or patching the upstream app is a later integration step.

Use one repository command path for the browser build/export check:

```bash
./scripts/typeduck-wasm-build.sh
```

The command must either build/check the Emscripten output or fail with an actionable blocker. A successful browser build prints verified output such as:

```text
TypeDuck WASM build verified: target/wasm32-unknown-emscripten/debug/yune_rime_api.wasm
```

Missing local browser tooling is a blocker, not a silent skip or a successful browser build. In blocker mode, the script prints `TypeDuck WASM build blocked:` and then runs the deterministic native fallback `cargo test -p yune-rime-api --test typeduck_web`:

```text
TypeDuck WASM build blocked: missing wasm32-unknown-emscripten Rust target.
Install with: rustup target add wasm32-unknown-emscripten
Native fallback still available: cargo test -p yune-rime-api --test typeduck_web
```

```text
TypeDuck WASM build blocked: missing Emscripten linker `emcc` on PATH.
Install/activate Emscripten SDK so `emcc` and `emar` are available, then rerun this command.
Native fallback still available: cargo test -p yune-rime-api --test typeduck_web
```

`scripts/typeduck-exports.txt` is the canonical adapter-owned export list. It contains exactly these non-prefixed symbols:

```text
yune_typeduck_init
yune_typeduck_process_key
yune_typeduck_select_candidate
yune_typeduck_delete_candidate
yune_typeduck_flip_page
yune_typeduck_deploy
yune_typeduck_customize
yune_typeduck_cleanup
yune_typeduck_response_json
yune_typeduck_response_handled
yune_typeduck_free_response
```

Emscripten must receive the same list with underscore-prefixed native names so optimization does not remove JS-callable adapter functions:

```bash
-sEXPORTED_FUNCTIONS=_yune_typeduck_init,_yune_typeduck_process_key,_yune_typeduck_select_candidate,_yune_typeduck_delete_candidate,_yune_typeduck_flip_page,_yune_typeduck_deploy,_yune_typeduck_customize,_yune_typeduck_cleanup,_yune_typeduck_response_json,_yune_typeduck_response_handled,_yune_typeduck_free_response
-sEXPORTED_RUNTIME_METHODS=ccall,cwrap,UTF8ToString
```

The export contract is adapter-specific. It must not add `Rime*`, `rime_get_api`, or librime-shaped function-table symbols to the TypeDuck-Web browser contract.

## Suggested JS flow

```js
const init = Module.cwrap('yune_typeduck_init', 'number', ['string', 'string', 'string']);
const processKey = Module.cwrap('yune_typeduck_process_key', 'number', ['number', 'number', 'number']);
const responseJson = Module.cwrap('yune_typeduck_response_json', 'number', ['number']);
const freeResponse = Module.cwrap('yune_typeduck_free_response', null, ['number']);
const cleanup = Module.cwrap('yune_typeduck_cleanup', null, ['number']);

await syncIdbfsFromDisk();
const state = init('/rime/shared', '/rime/user', 'typeduck_luna');
if (!state) throw new Error('failed to initialize Yune TypeDuck adapter');

const response = processKey(state, keycode, mask);
try {
  const jsonPtr = responseJson(response);
  const payload = JSON.parse(Module.UTF8ToString(jsonPtr));
  renderCandidates(payload.context?.candidates ?? []);
  appendCommits(payload.commits ?? []);
} finally {
  freeResponse(response);
}

cleanup(state);
await syncIdbfsToDisk();
```

`deploy` and `customize` are explicit operations. After either operation, the browser host should sync IDBFS back to persistent storage.

## Current scope

This adapter is native-tested through Rust integration tests. It does not yet include:

- TypeDuck-Web source patches.
- A JS package, bundler config, or generated TypeScript wrapper.
- Browser E2E coverage.
- Browser filesystem persistence orchestration beyond documenting host responsibilities.
- Multi-instance isolation beyond one active process-global Yune/RIME service.
- AI-native ranking or provider integration.
