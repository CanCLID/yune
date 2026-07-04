# M55 Partial Closeout - 2026-07-04

Verdict: M55 closes partial/no-go at Phase 2. The byte-backed poet storage work is real and retained as a measured storage improvement, but the full M55 ratchet is red on the long Luna sentence rows and one Track B product latency guard. Per the plan's Phase 2 no-go rule, Phase 3/Phase 4 work does not start from this state.

## Evidence Roots

- Phase 0 baseline and ratchet artifact: `docs/reports/evidence/m55-native-match-or-beat/phase-0-baseline/`
- Phase 1 memory attribution: `docs/reports/evidence/m55-native-match-or-beat/phase-1-attribution/`
- Phase 2 poet storage: `docs/reports/evidence/m55-native-match-or-beat/phase-2-poet-storage/`
- Fresh closeout ratchet: `docs/reports/evidence/m55-native-match-or-beat/final/verification-ratchet-no-go-2026-07-04/`

## Final Same-Run Gate

Command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 `
  -OutputRoot docs\reports\evidence\m55-native-match-or-beat\final\verification-ratchet-no-go-2026-07-04 `
  -Iterations 9 -SessionIterations 60 -KeyIterations 80 `
  -TrackAInputs "n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" `
  -DeployProductBeforeBenchmark `
  -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv `
  -FailOnRegression
```

Expected closeout result: nonzero exit, because the M55 ratchet is still red. `threshold-check.csv` records:

| Row | Observed | Ceiling | Status |
| --- | ---: | ---: | --- |
| `ceshiyixiachangjushuruxingnengzenyang` | `5.964x` | `3.267x` | fail |
| `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` | `4.030x` | `2.447x` | fail |
| Track B product long row | `414.059 us` | `375.253 us` | fail |

Same-run passing rows:

| Row | Observed | Ceiling |
| --- | ---: | ---: |
| `n` | `2.776x` | `3.050x` |
| `ni` | `3.063x` | `3.223x` |
| `hao` | `2.138x` | `2.287x` |
| `zhongguo` | `0.264x` | `0.325x` |
| `cszysmsrsd` | `0.490x` | `0.532x` |
| `zybfshmsru` | `0.694x` | `0.770x` |
| startup ratio | `0.553x` | `1.101x` |
| session absolute | `25,261.600 us` | `25,533.310 us` |
| Track A peak working set | `110,542,848 B` | `198,000,000 B` |

## What Landed

- Phase 0 added the full-suite M55 ratchet artifact, including startup/session, all eight Track A rows, Track B absolutes, and win-row ceilings.
- Phase 1 attributed more than 80% of the old unclassified native Luna floor and revised the Tier M memory bar to an evidence-backed `125 MB`.
- Phase 2 introduced a versioned `YUNE-POET/1` artifact, deploy validation, corrupt-artifact rejection, byte-backed runtime reads, product-path candidate parity evidence, and native memory-owner proof.
- The large poet payload owners moved from heap-owned payloads to file-backed poet bytes. The final closeout run reports Track A peak `110,542,848 B`, down from the M52-era `188,383,232 B`.

## Why It Stops

The Phase 2 no-go rule says byte-backed access must still hold the latency ceilings after reasonable access-path work, otherwise M55 closes partial and stops. The bounded follow-up in `phase-2-poet-storage/access-path-followup-no-go-2026-07-03.md` improved the long-row medians but did not clear the ratchet. The fresh closeout run confirms the same no-go shape on current code. Continuing into Phase 3/4 would be a new follow-up decision, not M55 closeout.

## Threshold Handoff

- M52 remains the standing green native Track A regression guard:
  - `docs/reports/evidence/m52-track-a-guardrails-and-disposition/track-a-thresholds.csv`
  - `docs/reports/evidence/m52-track-a-guardrails-and-disposition/final-native-benchmark/threshold-check.csv`
- M55's threshold artifact remains checked in as research/no-go evidence:
  - `docs/reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv`
- M55 does not supersede M52 because the final M55 ratchet is red. No M55 latency ceiling is loosened, no M55 memory ceiling is tightened, and no M55 threshold becomes the standing repo gate.

## Scope Boundaries

- No public C ABI change.
- No retained runtime heap index.
- No browser, product, platform, iOS `phys_footprint`, AI, octagram/plugin, or public performance claim is made from M55.
- M56 and WEB-05 were not implemented as part of this closeout. M56 is unblocked only as the next separate engine milestone because this file makes the M55 threshold handoff explicit.

## Closeout Verification

Commands run on 2026-07-04:

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p yune-rime-api tests::deployment::workspace_update_reuses_prebuilt_artifacts_when_source_is_missing -- --nocapture
cargo test --workspace
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-core --test cantonese_parity
cargo build -p yune-cli
target\debug\yune-cli.exe frontend --shared-data-dir apps\yune-web\public\schema --user-data-dir <fresh-temp-dir> --schema luna_pinyin --sequence "<input> " --output json
git diff --check
git ls-files "*.gram"
git diff --name-only -- "*.gram" "*.marisa" "*.bin"
git ls-files --others --exclude-standard -- "*.gram" "*.marisa" "*.bin"
```

Results:

- `cargo fmt --check`: pass.
- `cargo clippy --workspace --all-targets -- -D warnings`: pass.
- `cargo test --workspace`: pass after updating the deployment prebuilt-reuse test to include the M55 `luna.poet.bin` artifact in its prebuilt set.
- Focused deployment regression: pass.
- `upstream_luna_pinyin_parity`: 12 passed.
- `cantonese_parity`: 37 passed.
- `cargo build -p yune-cli`: pass.
- Current product-path CLI check: green for the five Phase 2 rows; see `product-path-current-cli-2026-07-04.md` and `.json`.
- Fresh final M55 benchmark ratchet: expected nonzero no-go result; see `verification-ratchet-no-go-2026-07-04/threshold-check.csv`.
- `git diff --check`: pass.
- `git ls-files "*.gram"`: no output.
- `git diff --name-only -- "*.gram" "*.marisa" "*.bin"`: no output.
- `git ls-files --others --exclude-standard -- "*.gram" "*.marisa" "*.bin"`: no output after removing the generated `.marisa` probe file from the final benchmark artifact directory.
