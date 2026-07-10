# M59 Increment 2 - TypeDuck activation and unified forward navigation

Implementation commit `e37ee0111de2bde443da0c20ae3b6725f53d11df` is
pushed to `origin/main`. **Verdict: GREEN for M59-NAV-01.** This packet proves
the Increment 2 navigation/profile acceptance surface and records the required
informational checkpoint. It does not close M59 or any D-48 exact-order lane.

## Mechanism

The direct Cantonese harness and synthetic TypeDuck-Windows schema now activate
`SchemaBehaviorProfile::TypeduckJyutping` explicitly, matching the real product
profile marker. No behavior is inferred from schema ID, and no golden file was
edited. The three prior Cantonese failures and both Windows source/compiled
boundary failures changed only because the harnesses now exercise the product
profile; `typeduck-before-after.csv` records the exact values.

Forward navigation now has one private `Engine` policy:

| Profile/input | Forward-navigation preparation |
|---|---|
| Standard | Complete the candidate list |
| TypeDuck, more than two Unicode scalar values | Complete the candidate list |
| TypeDuck, at most two Unicode scalar values | Retain bounded candidates |

`Engine::change_page_by` and `Engine::next_candidate` own that policy.
Physical PageDown and selector next-candidate/page delegate to those methods;
`RimeChangePage` delegates to `Engine::change_page_by`; browser paging uses the
same API path. Regression fixtures use arbitrary schema IDs so a renamed schema
cannot accidentally provide the behavior.

## Native and API acceptance

All commands below were run from the implementation tree that became
`e37ee011`; the commit was then pushed with `HEAD == origin/main`.

| Command/surface | Result |
|---|---|
| `cargo test -p yune-core --test cantonese_parity -- --nocapture` | 41 passed / 0 failed / 0 ignored; 69.29 s |
| `cargo test -p yune-rime-api --test typeduck_windows_boundary -- --nocapture` | 4/4 passed |
| `cargo test -p yune-core forward_navigation -- --nocapture` | 2/2 focused tests passed |
| `cargo test -p yune-rime-api physical_and_api_page_down_share_short_typeduck_bounded_state -- --nocapture` | 1/1 passed |
| `cargo test -p yune-rime-api --test yune_web m58_yune_web -- --nocapture` | 3/3 passed (`beingo`, `zi`, `zijiguk`, including physical PageDown) |
| `cargo test -p yune-rime-api --test yune_web m59_luna_ -- --nocapture` | 4/4 passed |
| `cargo clippy -p yune-core -p yune-rime-api --all-targets -- -D warnings` | passed |
| `cargo fmt --check` and `git diff --check` | passed |

An initial combined orchestration command reached its 124-second wrapper timeout
while the heavy Cantonese test was still running; it produced no assertion
failure. The load-bearing Cantonese command was then rerun alone to completion
with the 41/41 result above. The disposable Cargo target was removed after
verification when C: became low on space.

Two independent reviews returned GO: the first checked contract/mechanism and
selector semantics; the second checked code quality, ABI safety, edge cases,
and test coverage. No C ABI field, function signature, or table format changed.

## Source-current WASM and real-browser acceptance

After `e37ee011` was committed and pushed, the release WASM was rebuilt with
native fallback forbidden:

```sh
export EMSDK="$PWD/target/emsdk"
export EMSDK_NODE="$EMSDK/node/22.16.0_64bit/bin/node.exe"
export PATH="$EMSDK/upstream/emscripten:$EMSDK/node/22.16.0_64bit/bin:$PATH"
export YUNE_WEB_WASM_REQUIRE_EMSCRIPTEN=1
scripts/yune-web-wasm-build.sh
```

The build verified native exports, patched and verified the Emscripten JS glue,
ran the module smoke (`yune_web_response_handled` plus FS write/read), and
verified the release browser WASM. `wasm-opt` was skipped because it could not
validate this Emscripten module; this is an explicit optimizer limitation, not
a native fallback. The JS glue scan supplied the documented export-verification
fallback after `llvm-nm` could not prove every export.

Post-build gates:

- runtime: 5 files / 65 tests passed; TypeScript build passed;
- app: typecheck passed; production build passed;
- focused Playwright: 1/1 passed against `http://127.0.0.1:5173` in 7.2 s.

The generated Playwright capture is imported byte-identically as
`m58-profile-reachability.json` and records:

| Input | Target | PageDown count | Target page |
|---|---|---:|---|
| `beingo` | `畀` | 1 | `畀 鼻 悲 秘 臂 彼` |
| `zi` | `諮` | 4 | `姊 脂 稚 諮 旨 就` |
| `zijiguk` | `諮` | 5 | `姿 姊 脂 稚 諮 旨` |

The `zi` screenshot was visually inspected and shows `諮` as candidate 4 on
that page. `browser-wasm-artifacts.csv` binds all generated JS/WASM, JSON, log,
and screenshot bytes. No generated JS, WASM, PNG, DLL, or compiled table is
copied into this evidence packet.

As an additional, non-repository inspection, the Codex in-app Browser kernel
was attempted but failed before page creation with
`failed to write kernel assets: The system cannot find the path specified. (os error 3)`.
The repository's required source-current Playwright path above is therefore the
load-bearing real-browser evidence. The final full local browser suite remains
a separate M59 closeout gate.

## Checkpoint and packet contents

`informational-checkpoint.md` records the uncaptured transformed-algebra rows,
expanded CJ-1 fixture, complete deterministic 83-group OpenCC same-code scope
inventory, and relative 4a/4b/4c effort/risk. It deliberately does not invent
whole-input Double Pinyin or Bopomofo expected strings.

This packet contains only Markdown, CSV, and JSON:

1. `README.md`
2. `typeduck-before-after.csv`
3. `browser-wasm-artifacts.csv`
4. `informational-checkpoint.md`
5. `opencc-same-code-inventory.csv`
6. `m58-profile-reachability.json`
7. `packet-manifest.csv`

`packet-manifest.csv` hashes the other six files and intentionally omits its
own hash to avoid recursion.
