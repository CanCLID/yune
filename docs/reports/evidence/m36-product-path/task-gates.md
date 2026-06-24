# M36 Task Gates

Raw final evidence:

- Native baseline: [`phase-0-baseline/summary.csv`](./phase-0-baseline/summary.csv)
- Native final: [`phase-4-final/summary.csv`](./phase-4-final/summary.csv)
- Product path status: [`phase-4-final/product_path_status.csv`](./phase-4-final/product_path_status.csv)

Gates:

| Gate | Result |
| --- | --- |
| `cargo fmt --check` | passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| `cargo test -p yune-core --test upstream_luna_pinyin_parity` | passed, `12` tests |
| `cargo test -p yune-core --test cantonese_parity` | passed, `37` tests |
| `cargo test -p yune-rime-api --test typeduck_web -- --test-threads=1` | passed, `29` tests |
| `cargo test --workspace` | passed; includes `typeduck_web` and TypeDuck Windows boundary tests |
| `cargo bench -p yune-rime-api --bench frontend_baselines` | passed; cargo emitted existing duplicate-output-name warnings |
| Native M36 baseline rerun | passed, `phase-0-baseline/` |
| Native M36 final rerun | passed, `phase-4-final/` |
| M36 SVG/XML and local report links | passed |
| `git diff --check` | passed |
| Runtime/browser/TypeDuck-Web patch gates | N/A; no runtime-visible, browser, TypeDuck-Web source, or TypeScript runtime files changed |
