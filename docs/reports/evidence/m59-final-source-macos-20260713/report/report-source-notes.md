# Report source notes

## Identity and protocol

- Yune: tracked source directly recorded clean before each measured round,
  detached at `5879405c7b0f76af4dca7382f00b3e0605386f2c`; after-move cleanliness
  is inferred from the next clean preflight for runs 1–4 and the later clean
  behavior-gate preflight for run 5; during measurement Git reported only the
  transient untracked evidence directory
- librime: clean detached `33e78140250125871856cdc5b42ddc6a5fcd3cd4`
- Five measured macOS rounds, each using 9 startup iterations, 60 session
  iterations, 80 key iterations, product deployment, the exact 17 Track A
  inputs, and the exact Track B input.
- Each round's release-dylib build check was a no-op, but the script's Cargo
  bench calls sequentially rebuilt the crate/harness for roughly 24–30 seconds
  before each lane. No build ran concurrently with a lane; the lane loaded the
  pre-copied fixed dylibs. This remains a thermal/order limitation.
- Rust/Cargo 1.96.1 and Command Line Tools `26.6.0.0.1781586589` with Apple
  clang 21.0.0; full Xcode was not selected, and `xcodebuild` was unavailable
  through the active Command Line Tools directory.
- Signed Windows ceilings are retained as diagnostic references only. They are
  neither changed nor treated as portable macOS acceptance thresholds.
- Output-location deviation: the unmodified benchmark script only permits an
  output root under the repository. Each run therefore wrote to an untracked
  transient directory in the disposable worktree and was moved external after
  completion. No tracked source or binary changed, but the literal
  all-generated-output-external requirement was not met; this is diagnostic
  evidence, not a fully protocol-conforming acceptance packet.

## Bounded sources

The report snapshot is constructed from:

- the five measured macOS run directories preserved externally after capture;
- the committed source-matched final-M59 Windows performance ratchet;
- the signed Increment-0 Windows baseline, unchanged thresholds, and expanded
  ceiling derivation;
- the M57 macOS candidate/model-owner packet;
- two commit-bound focused behavior-gate logs at the measured Yune commit.

Committed source paths and logical external run paths, byte hashes, and
validation results are listed in `source-manifest.csv`,
`artifact-hash-audit.csv`, and `validation-checks.csv` in the packet's
`analysis/` directory. The exact `$HOME` run roots are listed in the packet
README.

## Interpretation boundary

Candidate and logical-owner comparisons are cross-platform. Absolute latency,
RSS, Windows private/pagefile counters, and Apple memory counters are not
directly interchangeable. The report therefore uses the Windows ceiling only
as a diagnostic comparison and makes no Apple `phys_footprint` or macOS
acceptance-threshold claim.

The complete-input candidate snapshots cover page zero. The focused Lane-B
fixture covers all captured pages for seven inputs, and the long-sentence gate
covers deployed 37/59 page order and partial-selection recomposition. This is
strong evidence for the named M59 surfaces, not universal all-prefix/all-page
proof.

## Reproduction

From the packet root, regenerate the CSV analysis with:

```text
python3 analysis/analyze.py \
  --evidence-root <external-evidence-root> \
  --repo-root <yune-repository-root> \
  --output-dir <external-review-root>/analysis
```

Rebuild the bounded SQLite snapshot and report artifact with:

```text
python3 report/build_report_artifact.py \
  --evidence-root <external-evidence-root> \
  --repo-root <yune-repository-root> \
  --analysis-root <external-review-root>/analysis \
  --output-dir <external-review-root>/report
```

The packet analyzer copy was normalized to require explicit roots and to emit
logical paths; its external capture-time original retains absolute defaults.
The report builder also requires explicit roots. Both commands therefore write
only to the reviewer-selected external root. The checked-in CSV, SQLite, and
artifact remain independently auditable without rebuilding them.
