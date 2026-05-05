# Phase 9: Browser Filesystem And Persistence - Pattern Map

**Mapped:** 2026-05-05
**Files analyzed:** 5 likely new/modified files
**Analogs found:** 5 / 5

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `packages/yune-typeduck-runtime/src/filesystem.ts` | utility/service | file-I/O + request-response wrappers | `packages/yune-typeduck-runtime/src/typeduck.ts` + `src/response.ts` + `crates/yune-rime-api/src/typeduck_web.rs` | role-match |
| `packages/yune-typeduck-runtime/src/index.ts` | config/barrel | transform | `packages/yune-typeduck-runtime/src/index.ts` | exact |
| `packages/yune-typeduck-runtime/test/fake-filesystem.ts` | test utility | file-I/O + event-driven callback fake | `packages/yune-typeduck-runtime/test/fake-module.ts` | role-match |
| `packages/yune-typeduck-runtime/test/filesystem.test.ts` | test | file-I/O + request-response + callback errors | `packages/yune-typeduck-runtime/test/typeduck.test.ts` + `test/response.test.ts` + `crates/yune-rime-api/tests/typeduck_web.rs` | role-match |
| `docs/typeduck-web-adapter.md` | docs | request-response + file-I/O contract | `docs/typeduck-web-adapter.md` | exact |

## Pattern Assignments

### `packages/yune-typeduck-runtime/src/filesystem.ts` (utility/service, file-I/O + request-response wrappers)

**Primary analogs:**
- `packages/yune-typeduck-runtime/src/typeduck.ts` for TypeScript import/export style, lifecycle composition, and wrapper methods around `TypeDuckRuntime`.
- `packages/yune-typeduck-runtime/src/response.ts` and `packages/yune-typeduck-runtime/src/keys.ts` for deterministic custom error class and parser/validator helpers.
- `crates/yune-rime-api/src/typeduck_web.rs` for the native asset guard that TypeScript preflight must mirror.

**Imports pattern** (`packages/yune-typeduck-runtime/src/typeduck.ts` lines 0-2):
```typescript
import { keyEventToRimeKey, type TypeDuckKeyboardEventLike } from "./keys.js";
import { bindTypeDuckModule, type EmscriptenTypeDuckModule, type TypeDuckBindings } from "./module.js";
import { readTypeDuckResponse, type TypeDuckResponse } from "./response.js";
```

Apply this exact package-local ESM pattern: source imports include `.js` extensions even when importing `.ts` files under NodeNext, and type-only imports are grouped with values from the same module. For Phase 9, imports should be minimal, for example `import type { EmscriptenTypeDuckModule } from "./module.js";` and `import { TypeDuckRuntime, type TypeDuckInitOptions } from "./typeduck.js";` only if a combined init/helper wrapper is implemented.

**Deterministic error class pattern** (`packages/yune-typeduck-runtime/src/typeduck.ts` lines 10-15; `packages/yune-typeduck-runtime/src/response.ts` lines 40-45; `packages/yune-typeduck-runtime/src/keys.ts` lines 14-19):
```typescript
export class TypeDuckLifecycleError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TypeDuckLifecycleError";
  }
}

export class TypeDuckResponseError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TypeDuckResponseError";
  }
}

export class TypeDuckKeyError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TypeDuckKeyError";
  }
}
```

Create `TypeDuckFilesystemError` with the same deterministic shape. Prefer stable, assertion-friendly messages such as `TypeDuck filesystem sync failed`, `Missing TypeDuck filesystem assets: /yune/shared/default.yaml`, or `Invalid TypeDuck logical id: ../typeduck_luna`. If preserving callback causes, add an optional `cause` without making tests depend on platform-specific error formatting.

**Lifecycle composition pattern** (`packages/yune-typeduck-runtime/src/typeduck.ts` lines 27-34, 61-67, 69-86):
```typescript
static init(module: EmscriptenTypeDuckModule, options: TypeDuckInitOptions): TypeDuckRuntime {
  const bindings = bindTypeDuckModule(module);
  const statePtr = bindings.init(options.sharedDataDir, options.userDataDir, options.schemaId);
  if (statePtr === 0) {
    throw new TypeDuckLifecycleError("TypeDuck adapter init failed");
  }
  return new TypeDuckRuntime(bindings, statePtr);
}

deploy(): boolean {
  return this.#bindings.deploy(this.requireLiveState()) !== 0;
}

customize(configId: string, key: string, value: string): boolean {
  return this.#bindings.customize(this.requireLiveState(), configId, key, value) !== 0;
}

cleanup(): void {
  if (this.#cleanedUp) {
    return;
  }
  this.#cleanedUp = true;
  const ptr = this.#statePtr;
  this.#statePtr = 0;
  if (ptr !== 0) {
    this.#bindings.cleanup(ptr);
  }
}

private requireLiveState(): number {
  if (this.#cleanedUp || this.#statePtr === 0) {
    throw new TypeDuckLifecycleError("TypeDuck runtime has been cleaned up");
  }
  return this.#statePtr;
}
```

Phase 9 helpers should compose around this class, not replace it and not reach into private state. `deployAndSync` / `customizeAndSync` should call public `runtime.deploy()` / `runtime.customize(...)`, then `syncfs(false)`, and return the original boolean after sync succeeds. Do not alter `TypeDuckRuntime` ownership/freeing or cleanup idempotence.

**Native asset guard to mirror** (`crates/yune-rime-api/src/typeduck_web.rs` lines 498-560):
```rust
fn has_preloaded_runtime_assets(
    shared_data_dir: &CString,
    user_data_dir: &CString,
    schema_id: &CString,
) -> bool {
    let Ok(shared_data_dir) = shared_data_dir.to_str() else {
        return false;
    };
    let Ok(user_data_dir) = user_data_dir.to_str() else {
        return false;
    };
    let Ok(schema_id) = schema_id.to_str() else {
        return false;
    };
    if !is_valid_schema_id(schema_id) {
        return false;
    }

    let shared_data_dir = std::path::Path::new(shared_data_dir);
    let user_data_dir = std::path::Path::new(user_data_dir);
    let build_dir = user_data_dir.join("build");
    let schema_file = format!("{schema_id}.schema.yaml");
    let shared_schema = shared_data_dir.join(&schema_file);

    shared_data_dir.join("default.yaml").is_file()
        && shared_schema.is_file()
        && build_dir.join("default.yaml").is_file()
        && build_dir.join(schema_file).is_file()
        && has_preloaded_dictionary(shared_data_dir, &shared_schema)
}

fn is_valid_schema_id(schema_id: &str) -> bool {
    !schema_id.is_empty()
        && schema_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

fn has_preloaded_dictionary(
    shared_data_dir: &std::path::Path,
    schema_file: &std::path::Path,
) -> bool {
    let Some(dictionary) = required_dictionary(schema_file) else {
        return false;
    };
    shared_data_dir
        .join(format!("{dictionary}.dict.yaml"))
        .is_file()
}

fn required_dictionary(schema_file: &std::path::Path) -> Option<String> {
    let text = std::fs::read_to_string(schema_file).ok()?;
    let yaml: Value = serde_yaml::from_str(&text).ok()?;
    let schema = yaml.as_mapping()?;
    let translator = schema
        .get(Value::String("translator".to_owned()))?
        .as_mapping()?;
    translator
        .get(Value::String("dictionary".to_owned()))?
        .as_str()
        .filter(|dictionary| is_valid_schema_id(dictionary))
        .map(str::to_owned)
}
```

Phase 9 TypeScript should mirror the resulting required paths before native init: `sharedDataDir/default.yaml`, `sharedDataDir/<schemaId>.schema.yaml`, `sharedDataDir/<dictionaryId>.dict.yaml`, `userDataDir/build/default.yaml`, and `userDataDir/build/<schemaId>.schema.yaml`. Because the package has no YAML dependency, prefer requiring an explicit `dictionaryId` and document that it must match the schema’s `translator.dictionary`; do not add a parser unless planning decides mismatch detection requires it.

**Validation helper pattern** (`packages/yune-typeduck-runtime/src/response.ts` lines 149-196):
```typescript
function parseNullable<T>(
  value: unknown,
  parser: (value: unknown) => T,
  missingMessage: string,
): T | null {
  if (value === undefined) {
    throw new TypeDuckResponseError(missingMessage);
  }
  if (value === null) {
    return null;
  }
  return parser(value);
}

function expectRecord(value: unknown, message: string): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new TypeDuckResponseError(message);
  }
  return value as Record<string, unknown>;
}
```

Use small local validators and deterministic messages rather than broad runtime dependencies. For filesystem helpers, this maps to `isTypeDuckLogicalId(id)`, `assertTypeDuckLogicalId(id, label)`, `joinVirtualPath(...)`, `fileExists(fs, path)`, and `assertTypeDuckAssetsReady(...)`.

**Error handling / callback wrapping pattern** (`packages/yune-typeduck-runtime/src/response.ts` lines 47-69 and lines 71-77):
```typescript
export function readTypeDuckResponse(
  responsePtr: number,
  bindings: TypeDuckBindings,
): TypeDuckResponse {
  if (responsePtr === 0) {
    throw new TypeDuckResponseError("TypeDuck adapter returned null response");
  }

  try {
    const jsonPtr = bindings.responseJson(responsePtr);
    if (jsonPtr === 0) {
      throw new TypeDuckResponseError("TypeDuck adapter returned null response JSON");
    }

    const text = bindings.module.UTF8ToString(jsonPtr);
    const parsed = parseResponseJson(text);
    const response = parseTypeDuckResponse(parsed);
    response.handled = bindings.responseHandled(responsePtr) !== 0;
    return response;
  } finally {
    bindings.freeResponse(responsePtr);
  }
}

function parseResponseJson(text: string): unknown {
  try {
    return JSON.parse(text) as unknown;
  } catch {
    throw new TypeDuckResponseError("TypeDuck adapter returned malformed response JSON");
  }
}
```

For `FS.syncfs`, use the same principle: convert an unsafe/callback/native-looking boundary into a small deterministic wrapper. Do not swallow errors; reject with `TypeDuckFilesystemError`. Tests should assert the direction boolean (`true` from persistence before init, `false` to persistence after mutation) and failure message.

**Pitfalls for this file:**
- Do not add FS members to `EmscriptenTypeDuckModule`; `module.ts` is deliberately native-symbol-only.
- Do not import Node `path` for browser virtual paths. Use POSIX-like string joins, preserve leading `/`, trim duplicate slashes, and keep schema/dictionary IDs separate from path components.
- Do not fabricate placeholder YAML files. Missing assets must produce deterministic setup/init failure.
- Do not auto-fetch assets, touch DOM globals, or encode TypeDuck-Web app lifecycle policy.
- Do not automatically sync after every `processKey`; current exports do not reveal all userdb mutation boundaries. Provide explicit host sync helpers/docs instead.

---

### `packages/yune-typeduck-runtime/src/index.ts` (config/barrel, transform)

**Analog:** `packages/yune-typeduck-runtime/src/index.ts`

**Export pattern** (lines 0-3):
```typescript
export * from './module.js';
export * from './response.js';
export * from './typeduck.js';
export * from './keys.js';
```

Add the new helper module as another package-local `.js` export, matching the existing barrel style. The file currently uses single quotes and semicolons; keep that style in this barrel even though other TS source files use double quotes.

**Likely Phase 9 change:**
```typescript
export * from './filesystem.js';
```

**Pitfalls for this file:**
- Do not export from `./filesystem.ts`; NodeNext package source uses `.js` extensions in import/export specifiers.
- Do not reorder existing exports unless there is a reason; append the filesystem export to minimize churn.

---

### `packages/yune-typeduck-runtime/test/fake-filesystem.ts` (test utility, file-I/O + event-driven callback fake)

**Analog:** `packages/yune-typeduck-runtime/test/fake-module.ts`

**Imports and test utility shape** (`packages/yune-typeduck-runtime/test/fake-module.ts` lines 0-7, 16-33):
```typescript
import type {
  EmscriptenCType,
  EmscriptenTypeDuckModule,
  EmscriptenWrappedFunction,
  TypeDuckExport,
} from "../src/module";
import { TYPEDUCK_EXPORTS } from "../src/module";

type CallMap = Record<string, unknown[][]>;

interface FakeResponse {
  jsonPtr: number;
  handled: boolean;
  freed: boolean;
}

export class FakeTypeDuckModule implements EmscriptenTypeDuckModule {
  #nextPtr = 1_000;
  #strings = new Map<number, string>();
  #responses = new Map<number, FakeResponse>();
  #exports = new Map<string, EmscriptenWrappedFunction>();
  #calls: CallMap = {};

  initResult = 1;
  processKeyResult = 0;
  selectCandidateResult = 0;
  deleteCandidateResult = 0;
  flipPageResult = 0;
  deployResult = 1;
  customizeResult = 1;

  constructor() {
    this.registerDefaultExports();
  }
```

Mirror this style for `FakeTypeDuckFilesystem`: a class with private `#files` / `#directories` / `#calls` state, public knobs for error injection, and query helpers for assertions. Keep it package-test-local and deterministic.

**Call recording pattern** (`packages/yune-typeduck-runtime/test/fake-module.ts` lines 87-96 and 153-155):
```typescript
calls(symbol: string): unknown[][] {
  return this.#calls[symbol] ?? [];
}

private registerDefaultExports(): void {
  for (const symbol of TYPEDUCK_EXPORTS) {
    this.#calls[symbol] = [];
  }

private record(symbol: string, args: unknown[]): void {
  (this.#calls[symbol] ??= []).push(args);
}
```

For fake FS, record calls like `mkdirTree`, `writeFile`, `readFile`, `analyzePath`, `mount`, and `syncfs`. In tests, assert exact ordering where it matters: mount/ensure dirs before sync, sync from persistence before init, write/verify before native init, deploy/customize before sync-to-persistence.

**Behavior injection pattern** (`packages/yune-typeduck-runtime/test/fake-module.ts` lines 55-64, 91-123):
```typescript
register(symbol: string, fn: EmscriptenWrappedFunction): void {
  this.#exports.set(symbol, fn);
}

remove(symbol: TypeDuckExport): void {
  this.#exports.delete(symbol);
}

response(json: unknown, handled = true): number {
  return this.responseWithJsonPointer(this.string(JSON.stringify(json)), handled);
}
```

For fake FS, expose knobs rather than real browser dependencies, for example: `syncError?: unknown`, `mountError?: unknown`, `mkdirTreeUnavailable` by omitting the method only if the interface allows fallback, or a `writeFile` override. Keep callback behavior synchronous unless a test explicitly needs async; `await` still works with a promise wrapper around synchronous callback invocation.

**Rust fixture layout analog** (`crates/yune-rime-api/tests/typeduck_web.rs` lines 270-321):
```rust
struct TypeDuckRuntime {
    root: PathBuf,
    shared: PathBuf,
    user: PathBuf,
    shared_c: CString,
    user_c: CString,
    schema_id_c: CString,
}

impl TypeDuckRuntime {
    fn create(label: &str) -> Self {
        let root = unique_temp_dir(label);
        let shared = root.join("shared");
        let user = root.join("user");
        fs::create_dir_all(&shared).expect("shared dir should be created");
        fs::create_dir_all(user.join("build")).expect("staging dir should be created");
        let shared_c =
            CString::new(shared.to_string_lossy().as_ref()).expect("path should be valid");
        let user_c = CString::new(user.to_string_lossy().as_ref()).expect("path should be valid");
        let schema_id_c = CString::new(SCHEMA_ID).expect("schema id should be valid");
        Self {
            root,
            shared,
            user,
            shared_c,
            user_c,
            schema_id_c,
        }
    }

    fn write_schema(&self) {
        self.write_schema_with_dictionary("typeduck");
        self.write_dictionary("typeduck");
    }

    fn write_schema_with_dictionary(&self, dictionary: &str) {
        let default_config =
            "config_version: typeduck-web\nschema_list:\n  - schema: typeduck_luna\n";
        let schema_config = format!(
            "\
schema:\n  schema_id: typeduck_luna\n  name: TypeDuck Luna\nmenu:\n  page_size: 2\n  alternative_select_keys: AB\n  alternative_select_labels: [Alpha, Beta]\nswitches:\n  - name: ascii_mode\n    reset: 0\nengine:\n  translators:\n    - table_translator\ntranslator:\n  dictionary: {dictionary}\n"
        );
        let staging = self.user.join("build");
        fs::write(staging.join("default.yaml"), default_config)
            .expect("staging default config should be written");
        fs::write(staging.join("typeduck_luna.schema.yaml"), &schema_config)
            .expect("staging schema config should be written");
        fs::write(self.shared.join("default.yaml"), default_config)
            .expect("shared default config should be written");
        fs::write(self.shared.join("typeduck_luna.schema.yaml"), schema_config)
            .expect("shared schema config should be written");
    }
```

Use equivalent fixture helper methods in fake FS tests: `writeSchema(dictionaryId)`, `writeDictionary(dictionaryId)`, or helper asset objects. The TypeScript fake should validate virtual paths, not OS paths.

**Pitfalls for this file:**
- Do not read/write the host filesystem; this fake should be in-memory and Vitest-local.
- Do not require browser `indexedDB`, DOM, Emscripten runtime, or TypeDuck-Web checkout.
- Do not make `syncfs` a no-op that hides direction; record the `populate` boolean every time.
- Match source imports in tests: existing fake-module imports source without `.js` (`../src/module`), while test files import runtime modules with `.js`. Follow the nearest analog depending on whether importing types into a test utility or importing executable source in a test.

---

### `packages/yune-typeduck-runtime/test/filesystem.test.ts` (test, file-I/O + request-response + callback errors)

**Primary analogs:**
- `packages/yune-typeduck-runtime/test/typeduck.test.ts` for Vitest structure, fake module use, runtime helper functions, lifecycle assertions, and wrapper method assertions.
- `packages/yune-typeduck-runtime/test/response.test.ts` for deterministic error testing with captured thrown values.
- `crates/yune-rime-api/tests/typeduck_web.rs` for missing assets, wrong dictionary, path-like IDs, and deploy/customize expectations.

**Vitest imports and helper fixture pattern** (`packages/yune-typeduck-runtime/test/typeduck.test.ts` lines 0-26):
```typescript
import { describe, expect, it } from "vitest";

import { keyEventToRimeKey, RIME_KEY, RIME_MASK } from "../src/keys.js";
import { bindTypeDuckModule, TYPEDUCK_EXPORTS, TypeDuckBindingError } from "../src/module.js";
import { TypeDuckLifecycleError, TypeDuckRuntime } from "../src/typeduck.js";
import { FakeTypeDuckModule } from "./fake-module.js";

const statePtr = 42;
const defaultInitPtr = 1;

function responsePayload(overrides: Partial<Record<string, unknown>> = {}) {
  return {
    handled: true,
    commits: ["你"],
    context: null,
    status: null,
    ...overrides,
  };
}

function initializedRuntime(fake = new FakeTypeDuckModule()): TypeDuckRuntime {
  return TypeDuckRuntime.init(fake, {
    sharedDataDir: "/rime/shared",
    userDataDir: "/rime/user",
    schemaId: "typeduck_luna",
  });
}
```

Use this exact test structure: `describe(...)`, `it(...)`, small local fixture builders, and fake module/FS objects. For `filesystem.test.ts`, likely helpers are `filesystemOptions(...)`, `validAssets(...)`, `preparedFilesystem(...)`, and `initializedRuntime(fakeModule)` for deploy/customize wrappers.

**Exact call assertion pattern** (`packages/yune-typeduck-runtime/test/typeduck.test.ts` lines 81-90, 203-216):
```typescript
it("initializes with shared/user directories and schema id", () => {
  const fake = new FakeTypeDuckModule();
  const runtime = initializedRuntime(fake);

  expect(runtime).toBeInstanceOf(TypeDuckRuntime);
  expect(fake.calls("yune_typeduck_init")).toEqual([
    ["/rime/shared", "/rime/user", "typeduck_luna"],
  ]);
});

it("returns booleans from deploy and customize numeric adapter returns", () => {
  const fake = new FakeTypeDuckModule();
  fake.deployResult = 1;
  fake.customizeResult = 0;
  const runtime = initializedRuntime(fake);

  expect(runtime.deploy()).toBe(true);
  expect(runtime.customize("typeduck_luna.schema", "schema/name", "TypeDuck Luna Web")).toBe(false);

  expect(fake.calls("yune_typeduck_deploy")).toEqual([[defaultInitPtr]]);
  expect(fake.calls("yune_typeduck_customize")).toEqual([
    [defaultInitPtr, "typeduck_luna.schema", "schema/name", "TypeDuck Luna Web"],
  ]);
});
```

Add exact assertions for Phase 9: directories created (`/yune/shared`, `/yune/user`, `/yune/user/build`), files written at exact virtual paths, `syncfs(true)` before init or preparation, and `syncfs(false)` after deploy/customize/userdb sync helper.

**Deterministic throw assertions** (`packages/yune-typeduck-runtime/test/typeduck.test.ts` lines 92-103; `packages/yune-typeduck-runtime/test/keys.test.ts` lines 122-127):
```typescript
expect(() =>
  TypeDuckRuntime.init(fake, {
    sharedDataDir: "/rime/shared",
    userDataDir: "/rime/user",
    schemaId: "typeduck_luna",
  }),
).toThrow(new TypeDuckLifecycleError("TypeDuck adapter init failed"));

expect(() => keyEventToRimeKey({ key: "UnidentifiedKey" })).toThrow(TypeDuckKeyError);
expect(() => keyEventToRimeKey({ key: "UnidentifiedKey" })).toThrow(
  "Unsupported TypeDuck key: UnidentifiedKey",
);
```

Use both class and message assertions for `TypeDuckFilesystemError`, especially for invalid IDs and missing assets. Include invalid logical IDs from research: empty string, `../typeduck_luna`, `typeduck/luna`, and `typeduck\\luna`.

**Captured thrown pattern for finally/error side effects** (`packages/yune-typeduck-runtime/test/response.test.ts` lines 79-93):
```typescript
it("throws a deterministic error for malformed JSON and still frees the response", () => {
  const fake = new FakeTypeDuckModule();
  const ptr = fake.responseText("{not json", true);

  let thrown: unknown;
  try {
    readTypeDuckResponse(ptr, bindings(fake));
  } catch (error) {
    thrown = error;
  }

  expect(thrown).toBeInstanceOf(TypeDuckResponseError);
  expect(thrown).toHaveProperty("message", "TypeDuck adapter returned malformed response JSON");
  expect(fake.freedResponses()).toEqual([ptr]);
});
```

Use this when asserting sync failure side effects: if `deployAndSync` calls `runtime.deploy()` then `syncfs(false)` fails, the wrapper should throw `TypeDuckFilesystemError`, and the fake module should show deploy was already called while fake FS records the failed sync direction.

**Native failure-mode test pattern** (`crates/yune-rime-api/tests/typeduck_web.rs` lines 130-204):
```rust
#[test]
fn typeduck_adapter_documents_browser_host_layout_constraints() {
    let _guard = test_guard();
    let runtime = TypeDuckRuntime::create("browser-host-layout");

    assert!(runtime.shared.exists());
    assert!(runtime.user.exists());
    assert!(
        runtime.user.join("build").exists(),
        "browser host fixture must create user_data_dir/build before init"
    );

    let state_without_preloaded_assets = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(
        state_without_preloaded_assets.is_null(),
        "init without preloaded schema/dictionary assets must fail deterministically"
    );

    runtime.write_schema_with_dictionary("typeduck");
    runtime.write_dictionary("stray");
    let state_with_wrong_dictionary = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(
        state_with_wrong_dictionary.is_null(),
        "init must reject preloads that omit the selected schema dictionary"
    );

    let path_like_schema_id = CString::new("../typeduck_luna").expect("schema id should be valid");
    let state_with_path_like_schema_id = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            path_like_schema_id.as_ptr(),
        )
    };
    assert!(
        state_with_path_like_schema_id.is_null(),
        "init must reject path-like schema ids before probing assets"
    );

    runtime.write_dictionary("typeduck");
    let state = unsafe {
        yune_typeduck_init(
            runtime.shared_c.as_ptr(),
            runtime.user_c.as_ptr(),
            runtime.schema_id_c.as_ptr(),
        )
    };
    assert!(!state.is_null());
```

Mirror these cases in Vitest at the helper layer. Phase 9 should fail earlier with `TypeDuckFilesystemError` instead of relying on opaque `TypeDuckLifecycleError("TypeDuck adapter init failed")`.

**Test cases planner should include:**
- `prepareTypeDuckFilesystem` creates shared/user/build dirs and writes all explicit assets.
- `assertTypeDuckAssetsReady` lists missing required virtual paths without fabricating placeholders.
- invalid `schemaId` and invalid `dictionaryId` fail before any write path is joined.
- `syncFromPersistenceBeforeInit` records `syncfs(true)` and rejects callback errors.
- `syncToPersistenceAfterMutation` records `syncfs(false)` and rejects callback errors.
- `deployAndSync` returns `runtime.deploy()` boolean only after `syncfs(false)` succeeds.
- `customizeAndSync` returns `runtime.customize(...)` boolean only after `syncfs(false)` succeeds.
- optional `prepareAndInitTypeDuck` proves ordering: sync-from-persistence, layout/preload/verify, then `TypeDuckRuntime.init`.

**Pitfalls for this file:**
- Do not only assert “syncfs was called”; assert the populate boolean.
- Do not make missing assets pass because the helper wrote placeholders.
- Do not rely on real timers, browser IndexedDB, or Emscripten.
- Do not require Rust/native tests unless Rust adapter files are modified.

---

### `docs/typeduck-web-adapter.md` (docs, request-response + file-I/O contract)

**Analog:** `docs/typeduck-web-adapter.md`

**Existing browser filesystem contract style** (lines 74-87):
```markdown
## Browser filesystem contract

The Rust adapter only receives C string paths. The JS/Emscripten host is responsible for creating and syncing the virtual filesystem.

Expected layout before calling `yune_typeduck_init`:

- `shared_data_dir`: deploy source files such as `default.yaml`, `<schema>.schema.yaml`, and `<dict>.dict.yaml`.
- `user_data_dir`: user state, custom patches, userdb data, and the deployed `build/` directory.
- `user_data_dir/build`: deployed or preloaded runtime configs used by schema selection and key processing.

For Emscripten, TypeDuck-Web glue should mount MEMFS/IDBFS before calling `yune_typeduck_init`. The schema/dictionary assets must be preloaded into the virtual filesystem before init; Phase 7 native fallback tests require init to fail deterministically when those assets are missing rather than fabricating placeholder browser data.

The persistence sync remains a JS host responsibility until Phase 9. Browser code should sync persistent storage before init and after deploy/customize or userdb-changing flows, but this Rust adapter does not mount IDBFS, choose storage policy, or hide sync failures.
```

Update this section from “Phase 9 owns later” to “the TypeScript runtime package now provides DOM-free helpers.” Keep the same concise contract-first style: describe layout bullets, then required assets, then sync timing and recovery. Do not over-document app policy or TypeDuck-Web source patching.

**TypeScript package docs style** (lines 143-158, 160-188):
```markdown
## TypeScript runtime package

Phase 8 adds repository-owned bridge code at `packages/yune-typeduck-runtime` with package name `@yune-ime/typeduck-runtime`. The package is a typed wrapper around the canonical `yune_typeduck_*` adapter symbols for downstream integration; it is not a TypeDuck-Web app scaffold, bundler setup, generated binding pipeline, or browser filesystem orchestration layer.

Build and test it with package-local npm tooling only:

```bash
npm --prefix packages/yune-typeduck-runtime run build
npm --prefix packages/yune-typeduck-runtime test
```

Import the wrapper and deterministic key mapper from the package:

```typescript
import { TypeDuckRuntime, keyEventToRimeKey } from "@yune-ime/typeduck-runtime";
```

### Wrapper initialization and Module injection

Construct the wrapper only after the Emscripten Module is initialized and exposes `cwrap` plus `UTF8ToString`. The wrapper binds only the canonical `yune_typeduck_*` symbols listed in this document; retaining those exports and runtime methods remains the job of the Phase 7 build script and host Emscripten flags.

The browser host still owns virtual filesystem readiness before init. A typical wrapper flow is:

```typescript
await syncIdbfsFromDisk();

const runtime = TypeDuckRuntime.init(Module, {
  sharedDataDir: "/yune/shared",
  userDataDir: "/yune/user",
  schemaId: "luna_pinyin",
});
```

Phase 9 should revise the import example to include helper names selected by implementation, e.g. `prepareTypeDuckFilesystem`, `syncFromPersistenceBeforeInit`, `deployAndSync`, `customizeAndSync`, and `TypeDuckFilesystemError`. Keep code blocks short and runnable-looking.

**Low-level JS flow style** (lines 234-261):
```markdown
## Low-level JS flow

Hosts that do not use the TypeScript wrapper can still call the raw C/WASM exports directly:

```js
const init = Module.cwrap('yune_typeduck_init', 'number', ['string', 'string', 'string']);
const processKey = Module.cwrap('yune_typeduck_process_key', 'number', ['number', 'number', 'number']);
const responseJson = Module.cwrap('yune_typeduck_response_json', 'number', ['number']);
const freeResponse = Module.cwrap('yune_typeduck_free_response', null, ['number']);
const cleanup = Module.cwrap('yune_typeduck_cleanup', null, ['number']);

await syncIdbfsFromDisk();
const state = init('/rime/shared', '/rime/user', 'typeduck_luna');
if (!state) throw new Error('failed to initialize Yune TypeDuck adapter');
```

If docs retain low-level flow, add a note that low-level hosts must perform the same required layout/preload/sync order manually. Do not imply the Rust adapter mounts IDBFS.

**Current scope style to update** (lines 263-272):
```markdown
## Current scope

This adapter is native-tested through Rust integration tests, and Phase 8 adds the package-local TypeScript wrapper documented above. It does not yet include:

- Upstream TypeDuck-Web checkout, source patches, or bridge replacement; Phase 10 owns that integration.
- Real browser E2E coverage; Phase 10 owns browser app validation after the upstream seam is known.
- Generated bindings, broad frontend bundler scaffolding, or root JavaScript workspace setup.
- Browser filesystem persistence orchestration beyond documenting host responsibilities; Phase 9 owns MEMFS/IDBFS setup, schema and dictionary asset preload, persistence sync, and recovery paths.
- Multi-instance isolation beyond one active process-global Yune/RIME service.
- AI-native provider, ranking, context, memory, or privacy behavior; those remain deferred to a future milestone.
```

After Phase 9 implementation, remove/update the bullet that says browser persistence orchestration is not included. Keep deferred Phase 10 items intact.

**Docs pitfalls:**
- Do not promise real browser E2E coverage or upstream TypeDuck-Web integration in Phase 9.
- Do not claim automatic persistence of every userdb mutation. Document host sync boundaries: after explicit deploy/customize wrappers, after commit-producing interaction batches when the host chooses, and before cleanup/page lifecycle boundaries when possible.
- Do not present IDBFS sync failures as ignorable. State that failures surface as deterministic `TypeDuckFilesystemError` and may leave in-memory data stale or unpersisted until retried.
- Do not add new native exports to the docs unless implementation actually changes the export list.

## Shared Patterns

### Package-local TypeScript ESM / NodeNext imports

**Source:** `packages/yune-typeduck-runtime/src/typeduck.ts` lines 0-2; `packages/yune-typeduck-runtime/src/index.ts` lines 0-3; `packages/yune-typeduck-runtime/package.json` lines 1-16; `packages/yune-typeduck-runtime/tsconfig.json` lines 1-17

**Apply to:** `src/filesystem.ts`, `src/index.ts`, `test/filesystem.test.ts`, `test/fake-filesystem.ts`

```typescript
import { keyEventToRimeKey, type TypeDuckKeyboardEventLike } from "./keys.js";
import { bindTypeDuckModule, type EmscriptenTypeDuckModule, type TypeDuckBindings } from "./module.js";
import { readTypeDuckResponse, type TypeDuckResponse } from "./response.js";

export * from './module.js';
export * from './response.js';
export * from './typeduck.js';
export * from './keys.js';
```

```json
{
  "type": "module",
  "scripts": {
    "build": "tsc -p tsconfig.json",
    "test": "vitest run"
  }
}
```

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "NodeNext",
    "moduleResolution": "NodeNext",
    "strict": true,
    "declaration": true,
    "rootDir": "src",
    "outDir": "dist",
    "skipLibCheck": true
  },
  "include": [
    "src/**/*.ts"
  ],
  "exclude": [
    "test/**/*.ts",
    "dist"
  ]
}
```

Use package-local tooling only. No new root JS workspace, bundler, browser test runner, or dependencies are indicated by the existing package pattern.

### Narrow native Module boundary

**Source:** `packages/yune-typeduck-runtime/src/module.ts` lines 4-11, 13-25, 67-95

**Apply to:** `src/filesystem.ts`, docs, tests

```typescript
export interface EmscriptenTypeDuckModule {
  cwrap(
    ident: string,
    returnType: EmscriptenCType,
    argTypes: EmscriptenCType[],
  ): EmscriptenWrappedFunction;
  UTF8ToString(ptr: number, maxBytesToRead?: number, ignoreNul?: boolean): string;
}

export const TYPEDUCK_EXPORTS = [
  "yune_typeduck_init",
  "yune_typeduck_process_key",
  "yune_typeduck_select_candidate",
  "yune_typeduck_delete_candidate",
  "yune_typeduck_flip_page",
  "yune_typeduck_deploy",
  "yune_typeduck_customize",
  "yune_typeduck_cleanup",
  "yune_typeduck_response_json",
  "yune_typeduck_response_handled",
  "yune_typeduck_free_response",
] as const;
```

Filesystem concerns should be a separate minimal interface such as `TypeDuckFilesystem`; do not add `FS`, `IDBFS`, `mount`, or `syncfs` to `EmscriptenTypeDuckModule`.

### Deterministic errors and assertions

**Source:** `packages/yune-typeduck-runtime/src/response.ts` lines 40-45, 71-77; `packages/yune-typeduck-runtime/test/response.test.ts` lines 79-93

**Apply to:** `src/filesystem.ts`, `test/filesystem.test.ts`

```typescript
export class TypeDuckResponseError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TypeDuckResponseError";
  }
}

function parseResponseJson(text: string): unknown {
  try {
    return JSON.parse(text) as unknown;
  } catch {
    throw new TypeDuckResponseError("TypeDuck adapter returned malformed response JSON");
  }
}
```

```typescript
let thrown: unknown;
try {
  readTypeDuckResponse(ptr, bindings(fake));
} catch (error) {
  thrown = error;
}

expect(thrown).toBeInstanceOf(TypeDuckResponseError);
expect(thrown).toHaveProperty("message", "TypeDuck adapter returned malformed response JSON");
expect(fake.freedResponses()).toEqual([ptr]);
```

Phase 9 errors should be classed, stable, and tested by both instance and message where behavior matters.

### Browser asset preflight and logical ID validation

**Source:** `crates/yune-rime-api/src/typeduck_web.rs` lines 62-64, 498-560; `crates/yune-rime-api/tests/typeduck_web.rs` lines 130-204

**Apply to:** `src/filesystem.ts`, `test/filesystem.test.ts`, docs

```rust
if !has_preloaded_runtime_assets(&shared_data_dir, &user_data_dir, &schema_id) {
    return ptr::null_mut();
}

fn is_valid_schema_id(schema_id: &str) -> bool {
    !schema_id.is_empty()
        && schema_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}
```

Required TypeScript preflight paths must match the native guard:
- `${sharedDataDir}/default.yaml`
- `${sharedDataDir}/${schemaId}.schema.yaml`
- `${sharedDataDir}/${dictionaryId}.dict.yaml`
- `${userDataDir}/build/default.yaml`
- `${userDataDir}/build/${schemaId}.schema.yaml`

### Runtime mutation sync wrappers

**Source:** `packages/yune-typeduck-runtime/src/typeduck.ts` lines 61-67; `docs/typeduck-web-adapter.md` lines 206-207

**Apply to:** `src/filesystem.ts`, `test/filesystem.test.ts`, docs

```typescript
deploy(): boolean {
  return this.#bindings.deploy(this.requireLiveState()) !== 0;
}

customize(configId: string, key: string, value: string): boolean {
  return this.#bindings.customize(this.requireLiveState(), configId, key, value) !== 0;
}
```

```markdown
Candidate indices are page-relative, matching the native adapter contract. `deploy` and `customize` are explicit operations; after either operation, the browser host should sync IDBFS or equivalent persistent storage back to disk.
```

`deployAndSync` and `customizeAndSync` should preserve the runtime method return values and add persistence sync after the native mutation. If sync fails after a successful native mutation, surface `TypeDuckFilesystemError` and document that in-memory changes may need retry sync.

### Verification commands

**Source:** `packages/yune-typeduck-runtime/package.json` lines 9-12; `docs/typeduck-web-adapter.md` lines 147-152; `09-RESEARCH.md` lines 486-499

**Apply to:** all Phase 9 plans

```bash
npm --prefix /Users/trenton/Projects/yune/packages/yune-typeduck-runtime test
npm --prefix /Users/trenton/Projects/yune/packages/yune-typeduck-runtime run build
```

Optional only if Rust adapter/export list changes:

```bash
/Users/trenton/.cargo/bin/cargo test -p yune-rime-api typeduck_web
```

## No Analog Found

All likely Phase 9 files have close local analogs. No new source file lacks an implementation/test/docs pattern.

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| None | — | — | Existing TypeScript runtime, fake module tests, native browser adapter tests, and adapter docs cover the needed patterns. |

## Implementation Pitfalls Checklist

- Keep `packages/yune-typeduck-runtime/src/module.ts` narrow; define filesystem interfaces separately.
- Use `.js` specifiers for package source imports/exports under NodeNext.
- Do not add DOM, browser automation, `fetch`, service worker, or TypeDuck-Web app code.
- Reject path-like schema/dictionary IDs before joining virtual paths.
- Do not fabricate `default.yaml`, schema YAML, or dict YAML placeholders.
- Mirror all five native required files, including `userDataDir/build` copies and dictionary file.
- Test sync direction booleans, not just that sync happened.
- Surface callback errors as deterministic `TypeDuckFilesystemError`.
- Compose with `TypeDuckRuntime`; do not change lifecycle/freeing semantics.
- Document userdb persistence as host-owned sync boundaries, not fully automatic mutation detection.

## Metadata

**Analog search scope:** `/Users/trenton/Projects/yune/packages/yune-typeduck-runtime/src`, `/Users/trenton/Projects/yune/packages/yune-typeduck-runtime/test`, `/Users/trenton/Projects/yune/docs`, `/Users/trenton/Projects/yune/crates/yune-rime-api/src`, `/Users/trenton/Projects/yune/crates/yune-rime-api/tests`
**Files scanned:** 17 relevant files listed/read, including Phase 9 context/research, TypeScript runtime sources/tests, adapter docs, native adapter, and native tests
**Pattern extraction date:** 2026-05-05
