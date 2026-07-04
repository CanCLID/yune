# M55 Phase 2 Native Luna Memory-Owner Probe - 2026-07-03

Verdict: diagnostic owner movement is green; Phase 2 still needs the release full-ratchet evidence.

## Scope

This probe drives the real native `RimeApi` path against the committed product schema bundle and a fresh temporary runtime. It is evidence that the runtime byte-backed poet path is actually selected for `luna_pinyin`, not only covered by synthetic unit tests.

Inputs:

- Schema: `luna_pinyin`
- Shared data: `apps/yune-web/public/schema`
- Runtime behavior: deploy enabled; fresh temporary user/build directory created by the probe
- Probe sequence: `nihao`
- Evidence directory: `native-memory-probe-luna-runtime/`

Command:

```powershell
$env:YUNE_MEM_SCHEMA='luna_pinyin'
$env:YUNE_MEM_DEFAULT='luna_pinyin'
$env:YUNE_MEM_SHARED_DATA_DIR=(Resolve-Path 'apps\yune-web\public\schema').Path
Remove-Item Env:\YUNE_MEM_PREBUILT_BUILD_DIR -ErrorAction SilentlyContinue
Remove-Item Env:\YUNE_MEM_SKIP_DEPLOY -ErrorAction SilentlyContinue
$env:YUNE_MEM_SEQUENCE='nihao'
$env:YUNE_MEM_EVIDENCE_DIR='docs\reports\evidence\m55-native-match-or-beat\phase-2-poet-storage\native-memory-probe-luna-runtime'
cargo test -p yune-rime-api --test native_memory_probe -- --ignored --exact native_memory_probe_reports_working_set --nocapture
```

Observed result: the ignored probe test passed and wrote:

- `native-memory-probe-luna-runtime/owner-attribution.csv`
- `native-memory-probe-luna-runtime/owner-attribution.json`
- `native-memory-probe-luna-runtime/phase-memory.csv`
- `native-memory-probe-luna-runtime/summary.json`

## Owner Rows

The final owner profile reports the large poet owners as file-backed:

| Owner | Class | Bytes | Items | Storage |
| --- | --- | ---: | ---: | --- |
| `poet.entries_by_code` | `mmap_file_backed` | `3,048,137` | `70,805` | `poet_bin:mmap` |
| `poet.vocabulary` | `mmap_file_backed` | `25,492,848` | `421,966` | `poet_bin:mmap` |
| `poet.abbreviation_vocabulary` | `mmap_file_backed` | `713` | `11` | `poet_bin:mmap` |
| `poet.lookup_index` | `heap_owned_guarded` | `159,816` | `19,975` | `SentenceLookupIndex` |

`summary.json` reports:

- Final named owner bytes: `41,886,124`
- Final named heap-owned owner bytes: `826,679`
- Final clean mmap-file-backed estimate: `34,824,788`
- Steady post-typing working set: `100,126,720 B` (`95.5 MB`)
- Steady post-typing private bytes: `62,799,872 B` (`59.9 MB`)

## Boundary

This is diagnostic native probe evidence, not the release full-suite ratchet. Phase 2 still needs the benchmark-produced `memory-owner-profile.csv` plus threshold gate evidence before the plan checkbox can be closed or the memory ceiling can be tightened.
