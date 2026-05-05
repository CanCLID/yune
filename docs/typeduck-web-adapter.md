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

When using the TypeScript wrapper, treat one Emscripten Module instance as having one active `TypeDuckRuntime` at a time. `runtime.cleanup()` calls `yune_typeduck_cleanup` to finalize the process-global service, is idempotent at the wrapper layer, and zeros the consumed native state pointer before any later teardown path can reuse it. Wrapper operations after `cleanup()` throw deterministic lifecycle errors instead of calling the consumed native state pointer. Raw `yune_typeduck_cleanup(statePtr)` remains a low-level operation for non-wrapper hosts; wrapper callers do not manage that raw cleanup call directly.

## Browser filesystem contract

The Rust adapter only receives C string paths. The JS/Emscripten host is responsible for creating and syncing the virtual filesystem.

Expected layout before calling `yune_typeduck_init`:

- `shared_data_dir`: deploy source files such as `default.yaml`, `<schema>.schema.yaml`, and `<dict>.dict.yaml`.
- `user_data_dir`: user state, custom patches, userdb data, and the deployed `build/` directory.
- `user_data_dir/build`: deployed or preloaded runtime configs used by schema selection and key processing.

For Emscripten, browser glue should mount MEMFS plus IDBFS or an equivalent host persistence backend before calling `yune_typeduck_init`. The repository-owned TypeScript package now exposes DOM-free helpers for this contract:

```typescript
import {
  assertTypeDuckAssetsReady,
  customizeAndSync,
  deployAndSync,
  mountTypeDuckPersistence,
  prepareTypeDuckFilesystem,
  syncAfterUserDataChange,
  syncFromPersistenceBeforeInit,
  syncToPersistenceAfterMutation,
  TypeDuckFilesystemError,
  TypeDuckRuntime,
} from "@yune-ime/typeduck-runtime";
```

Browser callers provide logical `schemaId` and `dictionaryId` values plus explicit asset contents. Both IDs must be nonempty ASCII letters, digits, `_`, or `-`; path-like IDs are rejected before write paths are joined. `dictionaryId` must match the `translator.dictionary` value inside `schemaYaml`; Phase 9 helpers do not parse YAML, so a mismatch can pass helper path preflight while native `yune_typeduck_init` still rejects the layout as missing the schema-selected dictionary. The helper writes exactly the required shared/build files and does not fabricate fallback schema or dictionary data:

```typescript
const fsOptions = {
  sharedDataDir: "/yune/shared",
  userDataDir: "/yune/user",
  schemaId: "typeduck_luna",
  dictionaryId: "typeduck",
  assets: {
    defaultYaml,
    schemaYaml,
    dictionaryYaml,
  },
};

mountTypeDuckPersistence(Module.FS, Module.IDBFS, {}, "/yune");
await syncFromPersistenceBeforeInit(Module.FS); // FS.syncfs(true)
prepareTypeDuckFilesystem(Module.FS, fsOptions);
assertTypeDuckAssetsReady(Module.FS, fsOptions);

const runtime = TypeDuckRuntime.init(Module, {
  sharedDataDir: fsOptions.sharedDataDir,
  userDataDir: fsOptions.userDataDir,
  schemaId: fsOptions.schemaId,
});
```

`TypeDuckFilesystemError` is the deterministic setup/sync error surface. Missing assets include the missing virtual paths, such as `/yune/shared/default.yaml`, `/yune/shared/<schema>.schema.yaml`, `/yune/shared/<dict>.dict.yaml`, `/yune/user/build/default.yaml`, or `/yune/user/build/<schema>.schema.yaml`. Sync failures include a `direction` of `fromPersistence` for before-init populate failures or `toPersistence` for post-mutation flush failures, so callers can distinguish stale persisted state from possible unpersisted in-memory changes.

Persistence timing remains explicit and host-owned:

- Call `syncFromPersistenceBeforeInit(fs)` before runtime initialization to load persisted state with `FS.syncfs(true)`. If this fails, do not initialize; show the failure and retry persistence recovery first.
- Call `syncToPersistenceAfterMutation(fs)` after host-visible filesystem mutations that must survive reload.
- Use `deployAndSync(runtime, fs)` and `customizeAndSync(runtime, fs, configId, key, value)` to call the live runtime mutation first and then flush with `FS.syncfs(false)`. If the flush fails, the runtime mutation may have changed in-memory state that is not durable yet.
- Call `syncAfterUserDataChange(fs)` at explicit host boundaries where userdb data may have changed. Current native exports do not notify the host of every userdb mutation, so this remains a caller-chosen boundary rather than automatic coverage.

Stale deployed config recovery is local-first and deterministic. Recover by syncing from persistence, recreating the shared/user/build layout, preloading the caller-owned default/schema/dictionary assets, verifying all required files, running `deployAndSync` or `customizeAndSync` only with a live runtime when the stale case requires regeneration, and initializing or reinitializing only after required files are complete and the final `FS.syncfs(false)` succeeds.

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

## TypeScript runtime package

Phase 8 added repository-owned bridge code at `packages/yune-typeduck-runtime` with package name `@yune-ime/typeduck-runtime`, and Phase 9 added DOM-free browser filesystem/persistence helpers in the same package. The package is a typed wrapper around the canonical `yune_typeduck_*` adapter symbols plus fake-testable filesystem orchestration helpers for downstream integration; it is not a TypeDuck-Web app scaffold, bundler setup, generated binding pipeline, or browser application policy layer.

Build and test it with package-local npm tooling only:

```bash
npm --prefix packages/yune-typeduck-runtime run build
npm --prefix packages/yune-typeduck-runtime test
```

Import the wrapper, deterministic key mapper, and filesystem helpers from the package:

```typescript
import {
  keyEventToRimeKey,
  prepareTypeDuckFilesystem,
  syncFromPersistenceBeforeInit,
  syncToPersistenceAfterMutation,
  TypeDuckRuntime,
} from "@yune-ime/typeduck-runtime";
```

### Wrapper initialization and Module injection

Construct the wrapper only after the Emscripten Module is initialized and exposes `cwrap` plus `UTF8ToString`. The wrapper binds only the canonical `yune_typeduck_*` symbols listed in this document; retaining those exports and runtime methods remains the job of the Phase 7 build script and host Emscripten flags.

The browser host still owns virtual filesystem readiness before init. A typical wrapper flow is:

```typescript
const fsOptions = {
  sharedDataDir: "/yune/shared",
  userDataDir: "/yune/user",
  schemaId: "luna_pinyin",
  dictionaryId: "luna_pinyin",
  assets: { defaultYaml, schemaYaml, dictionaryYaml },
};

await syncFromPersistenceBeforeInit(Module.FS);
prepareTypeDuckFilesystem(Module.FS, fsOptions);

const runtime = TypeDuckRuntime.init(Module, {
  sharedDataDir: fsOptions.sharedDataDir,
  userDataDir: fsOptions.userDataDir,
  schemaId: fsOptions.schemaId,
});

const response = runtime.processKeyboardEvent({
  key: event.key,
  shiftKey: event.shiftKey,
  ctrlKey: event.ctrlKey,
  altKey: event.altKey,
  metaKey: event.metaKey,
  type: event.type,
});
renderCandidates(response.context?.candidates ?? []);
appendCommits(response.commits);

runtime.cleanup();
await syncToPersistenceAfterMutation(Module.FS);
```

### Wrapper operations and adapter mapping

The public wrapper operations map directly to the adapter contract:

| Wrapper operation | Adapter operation |
|-------------------|-------------------|
| `TypeDuckRuntime.init(...)` | `yune_typeduck_init` |
| `runtime.processKey(keycode, mask)` | `yune_typeduck_process_key` |
| `runtime.processKeyboardEvent(event)` | `keyEventToRimeKey(event)` plus `runtime.processKey(...)` |
| `runtime.selectCandidate(index)` | `yune_typeduck_select_candidate` |
| `runtime.deleteCandidate(index)` | `yune_typeduck_delete_candidate` |
| `runtime.flipPage(backward)` | `yune_typeduck_flip_page` |
| `runtime.deploy()` | `yune_typeduck_deploy` |
| `runtime.customize(configId, key, value)` | `yune_typeduck_customize` |
| `runtime.cleanup()` | `yune_typeduck_cleanup` |

Candidate indices are page-relative, matching the native adapter contract. `deploy` and `customize` are explicit operations; after either operation, the browser host should sync IDBFS or equivalent persistent storage back to disk. Wrapper callers can use `deployAndSync(runtime, fs)` or `customizeAndSync(runtime, fs, configId, key, value)` to preserve the required order without changing `TypeDuckRuntime` lifecycle ownership.

### Wrapper response ownership

Low-level C/WASM adapter operations that return `YuneTypeDuckResponse` allocate owned response pointers that must be freed exactly once. Wrapper callers receive parsed `TypeDuckResponse` objects, not raw response pointers, and should not call `yune_typeduck_free_response` directly.

The wrapper copies the JSON string with `Module.UTF8ToString`, parses and validates the response envelope, reads handled state through `yune_typeduck_response_handled`, and calls `yune_typeduck_free_response` in a centralized `finally` path. Null response pointers, null JSON pointers, malformed JSON, and malformed response envelopes surface as deterministic wrapper errors. Filesystem setup and sync helpers likewise surface missing assets and persistence failures as visible caller errors instead of hiding them behind app policy.

Callers that bypass the wrapper must still follow the low-level rule: copy JSON before `yune_typeduck_free_response`, and free each non-null owned response pointer exactly once.

### Browser key mapping

The wrapper exposes `keyEventToRimeKey(event)` and `processKeyboardEvent(event)`. The accepted event is a narrow DOM-free object with `key`, optional `shiftKey`, `ctrlKey`, `altKey`, `metaKey`, and optional `type`. Mapping uses `event.key` semantics and intentionally does not depend on deprecated keyboard APIs.

Phase 8 covers printable keys, Space, Enter, Backspace, Escape, Delete, arrows, PageUp/PageDown, Home/End, digit selection keys, and common modifiers. Exhaustive cross-browser keyboard edge cases are deferred until Phase 10 observes the real TypeDuck-Web seam.

```typescript
const { keycode, mask } = keyEventToRimeKey({
  key: event.key,
  shiftKey: event.shiftKey,
  ctrlKey: event.ctrlKey,
  altKey: event.altKey,
  metaKey: event.metaKey,
  type: event.type,
});
const response = runtime.processKey(keycode, mask);
```

## Low-level JS flow

Hosts that do not use the TypeScript wrapper can still call the raw C/WASM exports directly:

```js
const init = Module.cwrap('yune_typeduck_init', 'number', ['string', 'string', 'string']);
const processKey = Module.cwrap('yune_typeduck_process_key', 'number', ['number', 'number', 'number']);
const responseJson = Module.cwrap('yune_typeduck_response_json', 'number', ['number']);
const freeResponse = Module.cwrap('yune_typeduck_free_response', null, ['number']);
const cleanup = Module.cwrap('yune_typeduck_cleanup', null, ['number']);

// Low-level hosts must perform the same FS.syncfs(true), layout creation,
// explicit asset preload, and readiness verification described above.
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
// Flush durable filesystem changes with FS.syncfs(false) at the host boundary.
```

## Current scope

This adapter is native-tested through Rust integration tests. Phase 8 adds the package-local TypeScript wrapper, and Phase 9 adds the package-local browser filesystem/persistence helper contract documented above. It does not yet include:

- TypeDuck-Web source cloning, source patches, or bridge replacement; Phase 10 owns that integration.
- Real browser end-to-end coverage; Phase 10 owns browser app validation after the upstream seam is known.
- Generated bindings, broad frontend bundler scaffolding, or root JavaScript workspace setup.
- Browser application policy for choosing storage quota behavior, remote asset discovery, CDN/cache strategy, or service-worker lifecycle; Phase 09 helpers stay local-first and caller-driven.
- Native export expansion for persistence or userdb notification symbols; userdb persistence remains an explicit host sync boundary.
- Multi-instance isolation beyond one active process-global Yune/RIME service.
- AI provider, ranking, context, memory, or privacy behavior; those remain deferred to a future milestone.
