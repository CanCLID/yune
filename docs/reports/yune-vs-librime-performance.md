# Yune vs upstream librime performance report

Date: 2026-06-23

Evidence: [`evidence/yune-vs-librime-2026-06-23/`](./evidence/yune-vs-librime-2026-06-23/)

## Public summary

This benchmark compares Yune's current Rust `yune-rime-api` release DLL against upstream `rime/librime` `1.17.0` on the shared upstream `luna_pinyin` surface: startup/schema selection, session create/select/destroy, and key processing through the C API. It does not measure browser, WASM, React, TypeDuck-Web UI latency, or TypeDuck-only `jyut6ping3` behavior.

On this shared workload, Yune is not faster than upstream librime in any measured row. Upstream librime is substantially faster and uses far less resident memory for `luna_pinyin`:

- Startup/runtime-ready median: Yune `2.722s`; upstream librime `0.029s` (`94.6x` faster for librime).
- Startup resident working-set delta: Yune `208.6 MiB`; upstream librime `0.9 MiB`.
- Session create/select/destroy median: Yune `2.761s`; upstream librime `0.024s` (`115.1x` faster for librime).
- Key-sequence median latency: librime is `23.0x` to `312.9x` faster across `ni`, `hao`, and `zhongguo`.

What should not be claimed from this benchmark:

- No TypeDuck-Web browser startup or typing win is shown here.
- No TypeDuck `jyut6ping3` result is shown here.
- No cold deploy/build timing is shown here; both engines use already-built `luna_pinyin` assets.
- No pure engine-internal comparison is shown here; both engines are measured through the librime-shaped C API.

The practical recommendation is to proceed to P2-WIN-02. M30 should stay closed as an engine-representation memory win for the TypeDuck-expanded-table workload, not as a browser typing-latency win. If a future benchmark-driven engine task is opened, the concrete owner exposed here is upstream-`luna_pinyin` schema/session setup and key processing through the ABI, not the already-closed M30 storage slice.

## Methodology

### Versions and source identity

Yune:

- Repository: `C:\Users\laubonghaudoi\Documents\GitHub\yune`
- Source head recorded by the benchmark: `7112e485674e71a4922ce4707ed35824ca6d268a`
- Build command: `cargo build --release -p yune-rime-api`
- DLL under test: `target\release\yune_rime_api.dll`, copied as `rime.dll` into a transient per-engine run directory.
- DLL SHA-256: `81DB4277CA2A3BF4A1681E4644A21BE488665C3771533E516486B5FA83B3D569`.
- No Rust engine source file was changed for this report; the dirty status recorded in the evidence comes from docs/scripts/report work plus unrelated local plan edits.

Upstream librime:

- Local checkout verified at `C:\Users\laubonghaudoi\Documents\GitHub\librime`.
- Local checkout origin: `https://github.com/rime/librime.git`.
- Local checkout head at verification time: `d71168e9e8c8392ed219dca011dbc76b80727d6c`.
- Tag `1.17.0` resolves to `33e78140250125871856cdc5b42ddc6a5fcd3cd4`.
- Benchmark target binary: the existing upstream-oracle `1.17.0` Windows release asset under `target\upstream-oracle\1.17.0\extract\dist\lib\rime.dll`, not the newer local checkout head and not the TypeDuck fork.
- DLL SHA-256: `86B4C7357D4C6D293CE5589B234D8859CA2AC30923A03BEDFA3926EEAF97FB0B`.

### Machine

The benchmark ran on:

- OS: Microsoft Windows 11 Pro, version `10.0.26200`, build `26200`.
- CPU: AMD Ryzen 9 9950X3D 16-Core Processor, `16` cores / `16` logical processors.
- Memory: `63,762,190,336` bytes physical RAM.
- Machine: Micro-Star International Co., Ltd. `MS-7E84`.

### Schema assets and warm/cold setup

Both engines used the same upstream `luna_pinyin` assets from:

- Shared data: `target\upstream-oracle\1.17.0\rime-shared`
- User/build data: `target\upstream-oracle\1.17.0\rime-user\build`
- Schema: `luna_pinyin`
- Module list: `default`

This is a warm/no-deploy comparison against shared upstream assets. It does not run the deployer or rebuild dictionaries during timing. It is not a strict "both engines consume identical compiled binaries" comparison: upstream librime consumes its native `1.17.0` build artifacts, while Yune receives the same shared and build directories and may fall back to source YAML for upstream compiled sections it does not currently consume. That fallback is part of Yune's current C API path for these assets and is included in the timing.

The harness copies runtime DLLs and schema assets into transient directories under `target\yune-vs-librime-benchmark\yune-vs-librime-2026-06-23\`. The committed evidence folder keeps only logs, metadata, `samples.csv`, and `summary.csv`.

### Harness and workloads

The benchmark harness is [`../../scripts/yune-vs-librime-benchmark.cs`](../../scripts/yune-vs-librime-benchmark.cs), driven by [`../../scripts/benchmark-yune-vs-librime.ps1`](../../scripts/benchmark-yune-vs-librime.ps1).

The runner calls the same librime-shaped C API entry points for both DLLs: setup, initialize, create session, select schema, status/context reads, key processing, clear composition, destroy session, and finalize.

Workloads:

- `startup_warm_shared_assets_runtime_ready`: `RimeSetup` + `RimeInitialize` + `RimeCreateSession` + `RimeSelectSchema("luna_pinyin")` + `RimeGetStatus`, followed by destroy/finalize after timing.
- `session_create_select_destroy`: service already initialized, then create/select/destroy one session.
- `key_sequence_process_with_context`: service and session already initialized, then clear composition, process each character in the input, and read/free context. Inputs: `ni`, `hao`, `zhongguo`.

Iteration counts:

- Startup: `9`
- Session lifecycle: `9`
- Key sequences: `25` per input

The summary uses median and nearest-rank p95 from the raw samples. With only 9 startup/session samples, p95 is best read as a small-sample slow-end value, not a robust production tail estimate.

Memory is measured with Windows `GetProcessMemoryInfo` resident working-set counters from the benchmark process. Absolute resident-set values include the managed C# host and loaded DLLs. Deltas are more useful than absolutes, but they are still process-level, not allocator-level.

Rerun command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-yune-vs-librime.ps1 -OutputRoot docs\reports\evidence\yune-vs-librime-2026-06-23 -Iterations 9 -SessionIterations 9 -KeyIterations 25
```

## Results

### Startup and resident memory

Workload: `startup_warm_shared_assets_runtime_ready`

| Engine | Median | p95 | Min | Max | Ready working set | Ready delta | Peak working set |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Yune | `2,722,088.1 us` | `3,122,121.3 us` | `2,676,537.8 us` | `3,122,121.3 us` | `228.7 MiB` | `208.6 MiB` | `249.0 MiB` |
| librime 1.17.0 | `28,763.3 us` | `82,572.0 us` | `23,840.0 us` | `82,572.0 us` | `18.9 MiB` | `0.9 MiB` | `21.2 MiB` |

Median startup ratio: Yune is `94.6x` slower than upstream librime on this warm/no-deploy `luna_pinyin` C API path. The upstream p95 has one visible slow sample (`82.6ms`), but it remains much faster than the fastest Yune startup sample.

### Session lifecycle

Workload: `session_create_select_destroy`

| Engine | Median | p95 | Min | Max | Ready working set | Ready delta | Peak working set |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Yune | `2,761,003.8 us` | `2,784,791.6 us` | `2,736,506.7 us` | `2,784,791.6 us` | `76.4 MiB` | `4.2 MiB` | `249.5 MiB` |
| librime 1.17.0 | `23,996.7 us` | `26,479.8 us` | `22,497.6 us` | `26,479.8 us` | `18.2 MiB` | `0.1 MiB` | `21.3 MiB` |

Median session lifecycle ratio: Yune is `115.1x` slower than upstream librime. The peak column is process high-water and carries prior service initialization; use the ready working-set and delta columns for this per-session row.

### Key processing

Workload: `key_sequence_process_with_context`

| Input | Ops | Yune median | Yune p95 | Yune median/op | librime median | librime p95 | librime median/op | Median ratio |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `ni` | `2` | `5,612.2 us` | `5,734.3 us` | `2,806.1 us` | `29.1 us` | `29.9 us` | `14.6 us` | `192.9x` |
| `hao` | `3` | `11,045.6 us` | `11,568.1 us` | `3,681.9 us` | `35.3 us` | `60.7 us` | `11.8 us` | `312.9x` |
| `zhongguo` | `8` | `31,641.8 us` | `34,107.2 us` | `3,955.2 us` | `1,374.1 us` | `1,444.6 us` | `171.8 us` | `23.0x` |

Yune key-processing rows are stable within this run but much slower than upstream librime on the same C API path. The smaller ratio for `zhongguo` likely reflects upstream's higher cost once the longer input reaches richer candidate/context work, but upstream still remains faster by more than an order of magnitude.

## Interpretation

M26 made the TypeDuck performance problem measurable across native and browser paths, separated browser keydown-to-paint from engine work, and pruned impossible dynamic-correction lengths. M27 then attributed and reduced the TypeDuck-Web startup/runtime-init owner, including hard Windows process-memory evidence. M29 classified the large repeated-benchmark high-water behavior and landed a small no-op regex allocation win. M30's Lever A removed duplicate steady-state expanded-entry storage for spelling-algebra-backed TypeDuck tables and produced a real resident-memory win, while browser startup and typing stayed flat/noisy after a fresh WASM rebuild.

This report is separate from those TypeDuck-specific measurements. It asks how Yune's current Rust C API surface compares to upstream librime on an upstream-supported `luna_pinyin` path. The answer is clear: upstream librime remains dramatically faster and lower-memory for this shared surface.

That result does not invalidate M30. M30's acceptance was about an engine-representation memory slice under the TypeDuck-expanded-table workload; it did not claim a browser typing win and should not be reopened to chase a generic upstream-luna benchmark. It does, however, name a concrete future owner if a named target needs it: `luna_pinyin` schema/session setup and translator/key-processing cost through the ABI.

## Caveats

- Yune is target-driven and uses librime as a compatibility floor, not as a bit-for-bit feature or implementation checklist.
- The benchmark covers only comparable upstream-supported `luna_pinyin` workloads. It does not cover TypeDuck-only `jyut6ping3`, Windows IME integration, browser/WASM runtime behavior, or AI-native paths.
- Both engines use the same C# host and C ABI calls, but memory counters are process-level and include host overhead.
- The run uses a warm/no-deploy shared asset setup. Cold deployment and dictionary rebuild are intentionally out of scope, but Yune may still parse source YAML as part of its current fallback path when it cannot consume an upstream compiled section.
- The upstream binary is the pinned `1.17.0` oracle asset. The local upstream checkout was verified only to confirm provenance and the tag commit; the newer local checkout head was not benchmarked.

## Recommendation

Proceed to P2-WIN-02. Do not schedule more generic engine-startup optimization solely because this report exists. Open a new engine-performance plan only if a named target needs it and the plan names the concrete owner exposed here: upstream-`luna_pinyin` schema/session setup, translator installation, or key-processing cost through the ABI.
