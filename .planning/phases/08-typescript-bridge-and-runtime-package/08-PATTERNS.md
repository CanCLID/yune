# Phase 08: TypeScript Bridge And Runtime Package - Pattern Map

**Mapped:** 2026-05-04
**Files analyzed:** 12 new/modified files inferred from CONTEXT.md and RESEARCH.md
**Analogs found:** 10 / 12

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `packages/yune-typeduck-runtime/package.json` | config | batch | `.planning/codebase/STACK.md` | constraint-only |
| `packages/yune-typeduck-runtime/tsconfig.json` | config | batch | `.planning/codebase/STACK.md` | constraint-only |
| `packages/yune-typeduck-runtime/src/index.ts` | utility | transform | `scripts/typeduck-exports.txt` | partial-match |
| `packages/yune-typeduck-runtime/src/module.ts` | provider | request-response | `docs/typeduck-web-adapter.md` | role-match |
| `packages/yune-typeduck-runtime/src/response.ts` | utility | transform | `crates/yune-rime-api/tests/typeduck_web.rs` | exact |
| `packages/yune-typeduck-runtime/src/typeduck.ts` | service | request-response | `crates/yune-rime-api/src/typeduck_web.rs` | exact |
| `packages/yune-typeduck-runtime/src/keys.ts` | utility | transform | `crates/yune-rime-api/src/key_table.rs` | exact |
| `packages/yune-typeduck-runtime/test/fake-module.ts` | test | request-response | `crates/yune-rime-api/tests/typeduck_web.rs` | role-match |
| `packages/yune-typeduck-runtime/test/response.test.ts` | test | transform | `crates/yune-rime-api/tests/typeduck_web.rs` | exact |
| `packages/yune-typeduck-runtime/test/typeduck.test.ts` | test | request-response | `crates/yune-rime-api/tests/typeduck_web.rs` | exact |
| `packages/yune-typeduck-runtime/test/keys.test.ts` | test | transform | `crates/yune-rime-api/src/key_table.rs` | role-match |
| `docs/typeduck-web-adapter.md` | documentation | request-response | `docs/typeduck-web-adapter.md` | exact self-update |

## Repository Constraint: No Existing JS/TS Package Tooling

**Source:** `.planning/codebase/STACK.md` lines 71-76

```markdown
**Build:**
- Workspace manifest: `Cargo.toml`.
- Crate manifests: `crates/yune-core/Cargo.toml`, `crates/yune-schema/Cargo.toml`, `crates/yune-rime-api/Cargo.toml`, and `crates/yune-cli/Cargo.toml`.
- Lockfile: `Cargo.lock`.
- No package.json, pyproject, go.mod, or other language package manifests are present.
```

**Apply to:** `package.json`, `tsconfig.json`, and all TS package planning.

**Pattern constraint:** There is no repository-owned TypeScript, JavaScript, npm workspace, bundler, Vitest, or tsconfig pattern to copy. Keep Phase 8 package-local, minimal, deterministic in Node, and do not introduce TypeDuck-Web app scaffolding or browser E2E infrastructure.

## Pattern Assignments

### `packages/yune-typeduck-runtime/package.json` (config, batch)

**Analog:** `.planning/codebase/STACK.md` constraint-only; no real `package.json` analog exists.

**Absence pattern** (lines 71-76):
```markdown
**Build:**
- Workspace manifest: `Cargo.toml`.
- Crate manifests: `crates/yune-core/Cargo.toml`, `crates/yune-schema/Cargo.toml`, `crates/yune-rime-api/Cargo.toml`, and `crates/yune-cli/Cargo.toml`.
- Lockfile: `Cargo.lock`.
- No package.json, pyproject, go.mod, or other language package manifests are present.
```

**Planner guidance:** Create package-local npm metadata only. Prefer `private: true`, package-local scripts for `build` and `test`, and dev dependencies for TypeScript/Vitest only. Do not add root npm workspace tooling unless a plan explicitly justifies it.

---

### `packages/yune-typeduck-runtime/tsconfig.json` (config, batch)

**Analog:** `.planning/codebase/STACK.md` constraint-only; no real `tsconfig.json` analog exists.

**Rust-only tooling pattern** (lines 43-47):
```markdown
**Build/Dev:**
- Build with Cargo from the workspace root: `cargo build`, `cargo test`, `cargo run -p yune-cli`.
- No build scripts (`build.rs`) are present.
- Root workspace metadata in `Cargo.toml` declares `edition = "2021"`, `license = "BSD-3-Clause"`, `repository = "https://github.com/yune-ime/yune"`, and `rust-version = "1.76"`.
- Root workspace lint declarations in `Cargo.toml` set `unsafe_code = "forbid"` and Clippy `all`/`pedantic` to warn; member manifests do not contain per-crate `[lints]` sections.
```

**Planner guidance:** Since TS config has no analog, use research defaults: strict checking, declaration emit, `rootDir: src`, `outDir: dist`, and browser-compatible runtime source. Keep Node types test-only if needed.

---

### `packages/yune-typeduck-runtime/src/index.ts` (utility, transform)

**Analog:** `scripts/typeduck-exports.txt`

**Canonical public surface seed** (lines 0-10):
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

**Planner guidance:** Public TS exports should expose TypeDuck-shaped wrapper types/functions around these symbols only. Do not expose `RimeApi`, `rime_get_api`, or librime function-table details.

---

### `packages/yune-typeduck-runtime/src/module.ts` (provider, request-response)

**Analog:** `docs/typeduck-web-adapter.md`

**Exported symbol binding pattern** (lines 4-18):
```markdown
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
```

**Emscripten binding pattern** (lines 141-149):
```js
const init = Module.cwrap('yune_typeduck_init', 'number', ['string', 'string', 'string']);
const processKey = Module.cwrap('yune_typeduck_process_key', 'number', ['number', 'number', 'number']);
const responseJson = Module.cwrap('yune_typeduck_response_json', 'number', ['number']);
const freeResponse = Module.cwrap('yune_typeduck_free_response', null, ['number']);
const cleanup = Module.cwrap('yune_typeduck_cleanup', null, ['number']);
```

**Export-contract pattern** (lines 132-139):
```bash
-sEXPORTED_FUNCTIONS=_yune_typeduck_init,_yune_typeduck_process_key,_yune_typeduck_select_candidate,_yune_typeduck_delete_candidate,_yune_typeduck_flip_page,_yune_typeduck_deploy,_yune_typeduck_customize,_yune_typeduck_cleanup,_yune_typeduck_response_json,_yune_typeduck_response_handled,_yune_typeduck_free_response
-sEXPORTED_RUNTIME_METHODS=ccall,cwrap,UTF8ToString
```

**Planner guidance:** Define a narrow injected Module interface around `cwrap` and `UTF8ToString`, bind all 11 canonical symbols once, and fail fast if a binding is missing or not callable.

---

### `packages/yune-typeduck-runtime/src/response.ts` (utility, transform)

**Analog:** `crates/yune-rime-api/tests/typeduck_web.rs`

**Response copy/free test helper pattern** (lines 228-240):
```rust
fn response_json(response: *mut yune_rime_api::YuneTypeDuckResponse) -> Value {
    assert!(!response.is_null());
    let handled: Bool = unsafe { yune_typeduck_response_handled(response) };
    let json = unsafe { yune_typeduck_response_json(response) };
    assert!(!json.is_null());
    let text = unsafe { CStr::from_ptr(json) }
        .to_str()
        .expect("adapter JSON should be valid UTF-8")
        .to_owned();
    unsafe { yune_typeduck_free_response(response) };
    let value: Value = serde_json::from_str(&text).expect("adapter response should parse as JSON");
    assert_eq!(value["handled"].as_bool(), Some(handled == TRUE));
    value
}
```

**Null response contract** (lines 211-225):
```rust
assert!(unsafe { yune_typeduck_process_key(ptr::null_mut(), 'a' as i32, 0) }.is_null());
assert!(unsafe { yune_typeduck_response_json(ptr::null()) }.is_null());
assert_eq!(
    unsafe { yune_typeduck_response_handled(ptr::null()) },
    FALSE
);
unsafe { yune_typeduck_free_response(ptr::null_mut()) };
```

**JSON response shape source** (`docs/typeduck-web-adapter.md` lines 28-64):
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

**Planner guidance:** Implement one `readResponse`/`withResponse` path that throws on null response pointers and null JSON pointers, copies `UTF8ToString(jsonPtr)`, parses and validates the object shape, and calls `freeResponse(responsePtr)` in `finally` for every non-null response pointer.

---

### `packages/yune-typeduck-runtime/src/typeduck.ts` (service, request-response)

**Analog:** `crates/yune-rime-api/src/typeduck_web.rs`

**Imports and boundary pattern** (lines 0-12):
```rust
use std::{
    ffi::{CStr, CString},
    mem,
    os::raw::{c_char, c_int},
    ptr,
};

use serde_json::json;

use crate::{
    rime_get_api, rime_levers_get_api, Bool, RimeCandidate, RimeCommit, RimeComposition,
    RimeContext, RimeLeversApi, RimeMenu, RimeSessionId, RimeStatus, RimeTraits, FALSE, TRUE,
};
```

**Opaque state/response pattern** (lines 14-27):
```rust
#[repr(C)]
pub struct YuneTypeDuckState {
    session_id: RimeSessionId,
    shared_data_dir: CString,
    user_data_dir: CString,
    schema_id: CString,
    initialized: bool,
}

#[repr(C)]
pub struct YuneTypeDuckResponse {
    handled: Bool,
    json: CString,
}
```

**Init failure returns null** (lines 31-63):
```rust
pub unsafe extern "C" fn yune_typeduck_init(
    shared_data_dir: *const c_char,
    user_data_dir: *const c_char,
    schema_id: *const c_char,
) -> *mut YuneTypeDuckState {
    let Some(shared_data_dir) = cstring_from_ptr(shared_data_dir) else {
        return ptr::null_mut();
    };
    let Some(user_data_dir) = cstring_from_ptr(user_data_dir) else {
        return ptr::null_mut();
    };
    let Some(schema_id) = cstring_from_ptr(schema_id) else {
        return ptr::null_mut();
    };
    let Some(api) = api_table() else {
        return ptr::null_mut();
    };
    if !has_preloaded_runtime_assets(&shared_data_dir, &user_data_dir, &schema_id) {
        return ptr::null_mut();
    }
```

**Operation pattern** (lines 96-113):
```rust
pub unsafe extern "C" fn yune_typeduck_process_key(
    state: *mut YuneTypeDuckState,
    keycode: c_int,
    mask: c_int,
) -> *mut YuneTypeDuckResponse {
    operate(state, |api, session_id| {
        let Some(process_key) = api.process_key else {
            return response(
                FALSE,
                vec![],
                None,
                None,
                Some("process_key API unavailable"),
            );
        };
        let handled = process_key(session_id, keycode, mask);
        response_from_session(api, session_id, handled, None)
    })
}
```

**Cleanup ownership pattern** (lines 269-284):
```rust
pub unsafe extern "C" fn yune_typeduck_cleanup(state: *mut YuneTypeDuckState) {
    if state.is_null() {
        return;
    }
    let mut state = unsafe { Box::from_raw(state) };
    if let Some(api) = api_table() {
        if state.initialized {
            destroy_session(api, state.session_id);
            finalize_api(api);
            state.initialized = false;
        }
    }
}
```

**Planner guidance:** Hide `statePtr` inside a TypeScript runtime class/object. `init` should throw on zero state pointer before returning the runtime. `cleanup` should be idempotent in TypeScript and zero the state pointer before or during cleanup. All non-cleanup operations should call a `requireLiveState` guard and fail deterministically after cleanup.

---

### `packages/yune-typeduck-runtime/src/keys.ts` (utility, transform)

**Analog:** `crates/yune-rime-api/src/key_table.rs`

**RIME/X11 key constants for Phase 8 subset** (lines 6-24):
```rust
const XK_BACKSPACE: c_int = 0xff08;
const XK_TAB: c_int = 0xff09;
const XK_RETURN: c_int = 0xff0d;
const XK_ESCAPE: c_int = 0xff1b;
const XK_DELETE: c_int = 0xffff;
const XK_LEFT: c_int = 0xff51;
const XK_UP: c_int = 0xff52;
const XK_RIGHT: c_int = 0xff53;
const XK_DOWN: c_int = 0xff54;
const XK_HOME: c_int = 0xff50;
const XK_END: c_int = 0xff57;
const XK_PAGE_UP: c_int = 0xff55;
const XK_PAGE_DOWN: c_int = 0xff56;
```

**Modifier bit positions** (lines 124-142):
```rust
const MODIFIERS: &[(usize, &[u8])] = &[
    (0, b"Shift\0"),
    (1, b"Lock\0"),
    (2, b"Control\0"),
    (3, b"Alt\0"),
    (4, b"Mod2\0"),
    (5, b"Mod3\0"),
    (6, b"Mod4\0"),
    (7, b"Mod5\0"),
    (26, b"Super\0"),
    (27, b"Hyper\0"),
    (28, b"Meta\0"),
    (30, b"Release\0"),
];
```

**ASCII printable mapping pattern** (lines 275-373):
```rust
const ASCII_KEY_NAMES: &[(&[u8], c_int)] = &[
    (b"space\0", 0x20),
    (b"0\0", 0x30),
    (b"1\0", 0x31),
    (b"A\0", 0x41),
    (b"Z\0", 0x5a),
    (b"a\0", 0x61),
    (b"z\0", 0x7a),
];
```

**Lookup behavior for unsupported keys** (lines 1540-1551):
```rust
pub unsafe extern "C" fn RimeGetKeycodeByName(name: *const c_char) -> c_int {
    let Some(name) = c_name(name) else {
        return XK_VOID_SYMBOL;
    };
    lookup_keycode(name).unwrap_or(XK_VOID_SYMBOL)
}
```

**Planner guidance:** Implement a DOM-free `KeyboardEvent`-like mapper using `event.key`, not deprecated `keyCode`. Cover printable single-character keys, Space, Enter, Backspace, Escape, arrows, PageUp/PageDown, number selection keys, and modifier masks. Return a deterministic unsupported-key error or sentinel rather than silently mapping unknown keys to a wrong command.

---

### `packages/yune-typeduck-runtime/test/fake-module.ts` (test, request-response)

**Analog:** `crates/yune-rime-api/tests/typeduck_web.rs`

**Test import and operation surface pattern** (lines 0-15):
```rust
use std::{
    ffi::{CStr, CString},
    fs,
    path::PathBuf,
    ptr,
    sync::{Mutex, MutexGuard, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::Value;
use yune_rime_api::{
    rime_get_api, yune_typeduck_cleanup, yune_typeduck_customize, yune_typeduck_delete_candidate,
    yune_typeduck_deploy, yune_typeduck_flip_page, yune_typeduck_free_response, yune_typeduck_init,
    yune_typeduck_process_key, yune_typeduck_response_handled, yune_typeduck_response_json,
    yune_typeduck_select_candidate, Bool, FALSE, TRUE,
};
```

**Runtime fixture pattern to mirror in fake Module state** (lines 243-270):
```rust
struct TypeDuckRuntime {
    root: PathBuf,
    shared: PathBuf,
    user: PathBuf,
    shared_c: CString,
    user_c: CString,
    schema_id_c: CString,
}
```

**Planner guidance:** Fake Module should implement only `cwrap` and `UTF8ToString`. It should map requested symbol names to deterministic fake functions, allocate numeric state/response/string pointers, queue response payloads, and record calls to `free_response`, `cleanup`, `deploy`, and `customize` for assertions.

---

### `packages/yune-typeduck-runtime/test/response.test.ts` (test, transform)

**Analog:** `crates/yune-rime-api/tests/typeduck_web.rs`

**Copy-before-free assertion pattern** (lines 164-172):
```rust
let response = unsafe { yune_typeduck_process_key(state, 'b' as i32, 0) };
let json = unsafe { yune_typeduck_response_json(response) };
assert!(!json.is_null());
let text = unsafe { CStr::from_ptr(json) }
    .to_str()
    .expect("adapter JSON should be valid UTF-8")
    .to_owned();
unsafe { yune_typeduck_free_response(response) };
let value: Value = serde_json::from_str(&text).expect("copied response should parse as JSON");
```

**Null behavior pattern** (lines 211-225):
```rust
assert!(unsafe { yune_typeduck_process_key(ptr::null_mut(), 'a' as i32, 0) }.is_null());
assert!(unsafe { yune_typeduck_response_json(ptr::null()) }.is_null());
assert_eq!(
    unsafe { yune_typeduck_response_handled(ptr::null()) },
    FALSE
);
unsafe { yune_typeduck_free_response(ptr::null_mut()) };
```

**Planner guidance:** Tests should prove: successful response frees exactly once; malformed JSON still frees exactly once; null response pointer throws deterministic wrapper error and does not call free; null JSON pointer throws and frees the non-null response pointer; parsed `handled` agrees with adapter handled accessor if that accessor is used.

---

### `packages/yune-typeduck-runtime/test/typeduck.test.ts` (test, request-response)

**Analog:** `crates/yune-rime-api/tests/typeduck_web.rs`

**Process key flow pattern** (lines 27-72):
```rust
let state = unsafe {
    yune_typeduck_init(
        runtime.shared_c.as_ptr(),
        runtime.user_c.as_ptr(),
        runtime.schema_id_c.as_ptr(),
    )
};
assert!(!state.is_null());

let first = response_json(unsafe { yune_typeduck_process_key(state, 'b' as i32, 0) });
assert_eq!(first["handled"], Value::Bool(true));
assert_eq!(first["context"]["input"], Value::String("b".to_owned()));

unsafe { yune_typeduck_cleanup(state) };
```

**Candidate action pattern** (lines 99-124):
```rust
let next_page = response_json(unsafe { yune_typeduck_flip_page(state, FALSE) });
assert_eq!(next_page["handled"], Value::Bool(true));
assert_eq!(next_page["context"]["page_no"], Value::from(1));

let previous_page = response_json(unsafe { yune_typeduck_flip_page(state, TRUE) });
assert_eq!(previous_page["handled"], Value::Bool(true));
assert_eq!(previous_page["context"]["page_no"], Value::from(0));

let deleted = response_json(unsafe { yune_typeduck_delete_candidate(state, 0) });
assert_eq!(deleted["handled"], Value::Bool(true));

let selected = response_json(unsafe { yune_typeduck_select_candidate(state, 0) });
assert_eq!(selected["handled"], Value::Bool(true));
```

**Deploy/customize pattern** (lines 193-200):
```rust
assert_eq!(unsafe { yune_typeduck_deploy(state) }, TRUE);
let config_id = CString::new("typeduck_luna.schema").expect("config id should be valid");
let key = CString::new("schema/name").expect("custom key should be valid");
let value = CString::new("TypeDuck Luna Web").expect("custom value should be valid");
assert_eq!(
    unsafe { yune_typeduck_customize(state, config_id.as_ptr(), key.as_ptr(), value.as_ptr()) },
    TRUE
);
```

**Planner guidance:** Tests should cover wrapper `init`, `processKey`, `selectCandidate`, `deleteCandidate`, `flipPage`, `deploy`, `customize`, and `cleanup`. Add lifecycle misuse tests: init failure throws before exposing runtime, cleanup calls adapter once, second cleanup is no-op, and operations after cleanup throw without calling adapter operations.

---

### `packages/yune-typeduck-runtime/test/keys.test.ts` (test, transform)

**Analog:** `crates/yune-rime-api/src/key_table.rs`

**Named-key constants to assert** (lines 144-164):
```rust
const NAMED_KEYS: &[(&[u8], c_int)] = &[
    (b"BackSpace\0", XK_BACKSPACE),
    (b"Tab\0", XK_TAB),
    (b"Return\0", XK_RETURN),
    (b"Escape\0", XK_ESCAPE),
    (b"Delete\0", XK_DELETE),
    (b"Left\0", XK_LEFT),
    (b"Up\0", XK_UP),
    (b"Right\0", XK_RIGHT),
    (b"Down\0", XK_DOWN),
    (b"Page_Up\0", XK_PAGE_UP),
    (b"Prior\0", XK_PAGE_UP),
    (b"Next\0", XK_PAGE_DOWN),
    (b"Page_Down\0", XK_PAGE_DOWN),
];
```

**Modifier tests source** (lines 1510-1526):
```rust
pub unsafe extern "C" fn RimeGetModifierByName(name: *const c_char) -> c_int {
    let Some(name) = c_name(name) else {
        return 0;
    };
    MODIFIERS
        .iter()
        .find_map(|(index, modifier)| {
            (name == *modifier).then_some(1_i32.checked_shl(*index as u32).unwrap_or(0))
        })
        .unwrap_or(0)
}
```

**Planner guidance:** Tests should construct plain objects like `{ key: 'a' }` or `{ key: 'ArrowLeft', ctrlKey: true }`, not DOM `KeyboardEvent` instances. Assert exact keycode/mask pairs for printable chars, special keys, PageUp/PageDown, arrows, Space, Enter, Backspace, Escape, Shift/Control/Alt/Meta. Assert unsupported keys fail deterministically.

---

### `docs/typeduck-web-adapter.md` (documentation, request-response)

**Analog:** self-update from existing lifecycle, filesystem, and current-scope sections.

**Lifecycle wording to preserve/extend** (lines 66-70):
```markdown
The adapter exposes one active process-global Yune/RIME service. Browser callers should treat the pointer returned by `yune_typeduck_init` as the single live TypeDuck state for the current Module instance.

`yune_typeduck_cleanup` destroys the adapter session and finalizes the process-global RIME service. A later init may create a new service, but multiple simultaneous TypeDuck states with different shared/user directories are unsupported by this Phase 7 contract.
```

**Filesystem host-responsibility wording to preserve** (lines 72-84):
```markdown
The Rust adapter only receives C string paths. The JS/Emscripten host is responsible for creating and syncing the virtual filesystem.

Expected layout before calling `yune_typeduck_init`:

- `shared_data_dir`: deploy source files such as `default.yaml`, `<schema>.schema.yaml`, and `<dict>.dict.yaml`.
- `user_data_dir`: user state, custom patches, userdb data, and the deployed `build/` directory.
- `user_data_dir/build`: deployed or preloaded runtime configs used by schema selection and key processing.
```

**Current out-of-scope pattern** (lines 170-180):
```markdown
This adapter is native-tested through Rust integration tests. It does not yet include:

- TypeDuck-Web source patches.
- A JS package, bundler config, or generated TypeScript wrapper.
- Browser E2E coverage.
- Browser filesystem persistence orchestration beyond documenting host responsibilities.
- Multi-instance isolation beyond one active process-global Yune/RIME service.
- AI-native ranking or provider integration.
```

**Planner guidance:** Update this document only to reference the new TS runtime package and its lifecycle contract. Do not claim Phase 8 adds browser E2E, MEMFS/IDBFS orchestration, TypeDuck-Web app patches, or multi-instance isolation.

## Shared Patterns

### Canonical Adapter Symbol Contract

**Source:** `scripts/typeduck-exports.txt` lines 0-10
**Apply to:** `src/module.ts`, `src/index.ts`, `src/typeduck.ts`, fake Module tests, docs update

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

### Response Ownership

**Source:** `docs/typeduck-web-adapter.md` lines 22-27 and 154-162
**Apply to:** `src/response.ts`, `src/typeduck.ts`, `test/response.test.ts`, `test/typeduck.test.ts`

```markdown
Operations that return `YuneTypeDuckResponse` allocate an owned response. Browser or JS glue should copy the JSON string immediately and then call `yune_typeduck_free_response`.

`yune_typeduck_free_response(NULL)` is a no-op. `yune_typeduck_response_json(NULL)` returns `NULL`, and `yune_typeduck_response_handled(NULL)` returns `FALSE`.
```

```js
const response = processKey(state, keycode, mask);
try {
  const jsonPtr = responseJson(response);
  const payload = JSON.parse(Module.UTF8ToString(jsonPtr));
  renderCandidates(payload.context?.candidates ?? []);
  appendCommits(payload.commits ?? []);
} finally {
  freeResponse(response);
}
```

### Lifecycle Guard

**Source:** `crates/yune-rime-api/src/typeduck_web.rs` lines 269-284
**Apply to:** `src/typeduck.ts`, `test/typeduck.test.ts`, docs update

```rust
pub unsafe extern "C" fn yune_typeduck_cleanup(state: *mut YuneTypeDuckState) {
    if state.is_null() {
        return;
    }
    let mut state = unsafe { Box::from_raw(state) };
    if let Some(api) = api_table() {
        if state.initialized {
            destroy_session(api, state.session_id);
            finalize_api(api);
            state.initialized = false;
        }
    }
}
```

### Request/Response Operation Wrapper

**Source:** `crates/yune-rime-api/src/typeduck_web.rs` lines 319-340
**Apply to:** `src/typeduck.ts`, fake Module tests

```rust
fn operate(
    state: *mut YuneTypeDuckState,
    operation: impl FnOnce(&crate::RimeApi, RimeSessionId) -> *mut YuneTypeDuckResponse,
) -> *mut YuneTypeDuckResponse {
    if state.is_null() {
        return ptr::null_mut();
    }
    let Some(api) = api_table() else {
        return response(FALSE, vec![], None, None, Some("RimeApi unavailable"));
    };
    let state = unsafe { &*state };
    if !state.initialized || state.session_id == 0 {
        return response(
            FALSE,
            vec![],
            None,
            None,
            Some("TypeDuck state is not initialized"),
        );
    }
    operation(api, state.session_id)
}
```

### JSON Shape

**Source:** `crates/yune-rime-api/src/typeduck_web.rs` lines 354-375
**Apply to:** `src/response.ts`, `test/response.test.ts`, docs update

```rust
fn response(
    handled: Bool,
    commits: Vec<String>,
    context: Option<serde_json::Value>,
    status: Option<serde_json::Value>,
    error: Option<&str>,
) -> *mut YuneTypeDuckResponse {
    let mut payload = json!({
        "handled": handled == TRUE,
        "commits": commits,
        "context": context,
        "status": status,
    });
    if let Some(error) = error {
        payload["error"] = json!(error);
    }
```

### Key Mapping Constants

**Source:** `crates/yune-rime-api/src/key_table.rs` lines 6-24 and 124-142
**Apply to:** `src/keys.ts`, `test/keys.test.ts`

```rust
const XK_BACKSPACE: c_int = 0xff08;
const XK_RETURN: c_int = 0xff0d;
const XK_ESCAPE: c_int = 0xff1b;
const XK_DELETE: c_int = 0xffff;
const XK_LEFT: c_int = 0xff51;
const XK_UP: c_int = 0xff52;
const XK_RIGHT: c_int = 0xff53;
const XK_DOWN: c_int = 0xff54;
const XK_PAGE_UP: c_int = 0xff55;
const XK_PAGE_DOWN: c_int = 0xff56;
```

```rust
(0, b"Shift\0"),
(2, b"Control\0"),
(3, b"Alt\0"),
(26, b"Super\0"),
(27, b"Hyper\0"),
(28, b"Meta\0"),
(30, b"Release\0"),
```

### Test Serialization for Process-Global Runtime

**Source:** `crates/yune-rime-api/tests/typeduck_web.rs` lines 19-25
**Apply to:** TS tests if they share mutable fake Module global state; Rust tests already require this for real process-global adapter state

```rust
fn test_guard() -> MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
```

**Planner guidance:** TS fake Module tests should avoid true process-global state where possible. If a shared fake registry is used, serialize or reset it per test.

## No Analog Found

Files with no close codebase match; planner should use RESEARCH.md patterns and standard TS/Vitest practices instead:

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `packages/yune-typeduck-runtime/package.json` | config | batch | Repository has no `package.json`, npm workspace, or JS package convention. |
| `packages/yune-typeduck-runtime/tsconfig.json` | config | batch | Repository has no `tsconfig.json` or TS compiler convention. |

## Metadata

**Analog search scope:** repository root excluding `.git`, `target`, and worktree duplicates; focused analogs in `docs/`, `scripts/`, `crates/yune-rime-api/src/`, and `crates/yune-rime-api/tests/`.
**Files scanned:** 231 source/planning files after excluding build artifacts and `.claude` worktrees.
**Pattern extraction date:** 2026-05-04
