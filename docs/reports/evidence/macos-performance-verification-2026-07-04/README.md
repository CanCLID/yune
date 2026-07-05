# macOS Performance Verification - 2026-07-04

This bundle independently reruns the native Yune vs upstream librime performance
checks on this macOS machine. The upstream oracle was built in isolation under
`target/macos-performance-verification/` from the local
`/Users/laufei/Documents/GitHub/librime` checkout, pinned to
`33e78140250125871856cdc5b42ddc6a5fcd3cd4`.

The benchmark shape matches the Windows reports' native interactive path:
`process_key` followed by `get_context/free_context` after every keypress.

## Evidence

- Run 1: `run-1/`
- Run 2: `run-2/`
- Per-run verdicts: `run-*/macos-verdict.md`
- Per-run raw comparison: `run-*/summary-comparison.csv`
- Per-run claim classification: `run-*/claim-shape-check.csv`
- Per-run raw candidate snapshots: `run-*/candidate_snapshots.csv`
- Per-run memory-owner data: `run-*/memory-owner-profile.csv`

## Verdict

| Report claim | Run 1 | Run 2 | macOS classification |
| --- | ---: | ---: | --- |
| `n` slower than librime | 1.044x | 1.107x | confirmed, narrow |
| `ni` slower than librime | 0.875x | 0.895x | contradicted on macOS, narrow |
| `hao` slower than librime | 1.833x | 1.392x | confirmed |
| 37-char row slower than librime | 61.005x | 60.680x | confirmed |
| 59-char row slower than librime | 43.948x | 42.365x | confirmed |
| `zhongguo` faster than librime | 0.242x | 0.247x | confirmed |
| `cszysmsrsd` faster than librime | 5.020x | 5.083x | contradicted on macOS |
| `zybfshmsru` faster than librime | 7.035x | 7.339x | contradicted on macOS |
| startup/session faster rows | 0.500-0.521x | 0.576-0.597x | confirmed on these runs; still noisy/platform-specific |

Candidate text snapshots confirm the report disclosure for `ni`, `hao`, `n`,
`zhongguo`, and both long sentence rows. They contradict the report disclosure
for both abbreviation rows: `cszysmsrsd` and `zybfshmsru` do not match librime's
first candidate page on this pinned macOS oracle. Raw snapshot CSVs retain
comments; the claim classifier compares first-page candidate text/ranking.

macOS memory evidence confirms the relative Track A magnitude: Yune's maximum
observed peak resident set is about 448 MB while librime's is about 16 MB. These
are Darwin resident/peak-resident counters, not Windows private/pagefile
counters.

Browser rows were not rerun here. They remain carried Playwright evidence from
the existing reports, as planned.

## Verification Commands

Passed after the final code changes:

```sh
cargo fmt --check
cargo build --release -p yune-rime-api
cargo clippy -p yune-rime-api --bench native_inprocess_benchmark -- -D warnings
bash -n scripts/benchmark-native-rime-inprocess-macos.sh
```

Oracle checks performed:

```sh
git -C target/macos-performance-verification/librime-src rev-parse HEAD
nm -gU target/macos-performance-verification/librime-src/build/lib/librime.1.17.0.dylib | rg 'rime_get_api'
DYLD_LIBRARY_PATH=target/macos-performance-verification/librime-src/build/lib \
  target/macos-performance-verification/librime-src/build/bin/rime_deployer \
  --build \
  target/macos-performance-verification/oracle-1.17.0/rime-user \
  target/macos-performance-verification/oracle-1.17.0/rime-shared \
  target/macos-performance-verification/oracle-1.17.0/rime-user/build
```

