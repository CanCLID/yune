# M56 Final Closeout Evidence

Date: 2026-07-04

## Verdict

M56 closes complete.

- No public C ABI widening.
- No behavior change on defined happy paths.
- M56 named blocked rows: none.
- Corrected closeout asset policy: `apps/yune-web/public/schema` remains the
  WEB-03 launch set; M56 does not add default product `*.poet.bin` artifacts
  or newly generated product dictionaries to the public payload.
- Pre-existing M55 Phase 3R sentence-lattice rows remain ignored blocked rows
  in `upstream_luna_pinyin_parity`; they are not M56 regressions.
- Track B product prerequisite present:
  `apps/yune-web/source/public/schema`.

## Product Asset Gates

```powershell
npm.cmd --prefix apps/yune-web run check:schema-manifest
npm.cmd --prefix apps/yune-web run build:public
```

Results:

- `check:schema-manifest`: pass. The check validates
  `apps/yune-web/public/schema-asset-manifest.json` and
  `apps/yune-web/public-demo/schema-asset-manifest.json` against actual bytes,
  SHA-256 values, worker-referenced assets, manifest equality, and the
  no-`*.poet.bin` default-payload rule.
- `build:public`: pass.

## Full Quality Gate

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm.cmd --prefix packages/yune-web-runtime test
npm.cmd --prefix packages/yune-web-runtime run build
```

Results:

- `cargo fmt --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test --workspace`: pass
  - Includes `cold_start_conformance`: `1 passed`, final runtime `180.96s`
  - Includes `yune_web`: `38 passed`, `2 ignored`
- `npm.cmd --prefix packages/yune-web-runtime test`: pass, `5` files and
  `65` tests
- `npm.cmd --prefix packages/yune-web-runtime run build`: pass

## Parity Suites

```powershell
cargo test -p yune-core --test upstream_luna_pinyin_parity
cargo test -p yune-core --test cantonese_parity
cargo test -p yune-rime-api --test yune_web
```

Results:

- `upstream_luna_pinyin_parity`: `14 passed`, `13 ignored`
- `cantonese_parity`: `37 passed`
- `yune_web`: `38 passed`, `2 ignored`

## ABI Abuse And Panic Boundary

```powershell
$env:RUST_BACKTRACE='1'; cargo test -p yune-rime-api --test abi_abuse -- --nocapture; Remove-Item Env:RUST_BACKTRACE
```

Result: `5 passed`, `0 failed`.

## Standing M55 Ratchet

The M56 closeout ratchet uses the M55 corrective threshold artifact:

`docs/reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv`

Final command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/benchmark-native-rime-inprocess.ps1 `
  -OutputRoot docs\reports\evidence\m56-productization-hardening\final\ratchet-run `
  -Iterations 9 -SessionIterations 60 -KeyIterations 80 `
  -TrackAInputs "n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru" `
  -TrackBInputs "neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung" `
  -DeployProductBeforeBenchmark `
  -TrackAThresholds docs\reports\evidence\m55-native-match-or-beat\thresholds\m55-thresholds.csv `
  -FailOnRegression
```

Result: pass, `threshold-check.csv` reports `23` passing rows and no missing or
failed rows.

Performance read:

- The M56 hardening run passes the standing M55 corrective ratchet; it does not
  establish a new faster baseline.
- Some short-key and sentence-row ratios drift upward versus corrective run D but
  remain inside the committed ceilings: `n` `2.785x` / `2.890x`, `ni`
  `2.573x` / `2.666x`, `hao` `1.677x` / `1.731x`, 37-char `1.981x` /
  `2.094x`, and 59-char `1.525x` / `1.625x`.
- The likely owner is the added ABI/deploy guard surface around the real
  `process_key`/context-read path plus normal same-machine run noise. M56
  therefore records a green guard pass with tight headroom, not "no measurable
  cost."

Note: the threshold artifact is authoritative for the 59-character Track A row
spelling (`zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`).

Key output files:

- `ratchet-run/threshold-check.csv`
- `ratchet-run/summary-comparison.csv`
- `ratchet-run/track-a-yune/summary.csv`
- `ratchet-run/track-a-librime-1.17.0/summary.csv`
- `ratchet-run/track-b-yune-product/summary.csv`

## Residual Named Follow-ups

These are not M56 close blockers because the support contract names the
boundary, but they remain visible follow-up work:

- `M56-FU-ABI-01`: dangling or wrong-ownership non-null C pointers remain
  caller undefined behavior; M56 guards null/degenerate validly addressable
  inputs but cannot make arbitrary foreign pointers safe.
- `M56-FU-ABI-02`: broader concurrent mutation/race stress beyond valid
  cross-thread session lookup is not promised by the threading contract.
- `M56-FU-POISON-01`: session-registry poison recovery is covered through the
  public ABI path; exhaustive poisoning of every process-global mutex remains
  future hardening if a later frontend exposes a concrete risk.

## Closeout Artifacts

- Frontend handoff: `final/frontend-handoff.md`
- Completed plan: `docs/plans/completed/m56-plan-engine-productization-hardening.md`
- Support contract: `docs/contracts/engine-support-contract.md`
- Roadmap: `docs/roadmap.md`
- Requirements: `docs/requirements.md`
- Milestone history: `docs/ledgers/milestone-history.md`
