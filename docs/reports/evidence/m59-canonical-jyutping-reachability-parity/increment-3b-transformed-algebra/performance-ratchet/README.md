# Increment 3b signed performance ratchet

This directory is the curated, executable five-run acceptance packet for
Increment 3b implementation commit
`2cb7e411c7f76f8206d5d2b04e61eb39a2087b4c`.

All five accepted runs measured the same 3,799,552-byte release DLL, SHA-256
`adddf54683e681c1bfe783db504fa4160a249d68b7a25ae0a0fc8df51e171a3a`,
against the same 17 Track A inputs and one Track B product input. Product
deployment was enabled and the startup/session/key iteration counts were
`9 / 60 / 80`. The strict per-run provenance also fixes the upstream DLL and
product schema tree hashes.

The checked-in packet is replayable from the repository root:

```powershell
$root = 'docs/reports/evidence/m59-canonical-jyutping-reachability-parity/increment-3b-transformed-algebra/performance-ratchet'
python scripts/aggregate-native-ratchet.py `
  --thresholds docs/reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv `
  --expected-runs 5 `
  --run "$root/run1" --run "$root/run2" --run "$root/run3" `
  --run "$root/run4" --run "$root/run5" `
  --output "$root/gate-verdict.csv"
```

The aggregator exits `0`; all 32 aggregate rows pass. Each accepted run keeps
its candidate snapshot, command record, environment, external provenance,
M37 metrics, memory-owner profile, summary comparison, and threshold check.
`gate-verdict.provenance.json` binds those raw inputs to the replayed verdict.

## Accepted Track B observations

All five observations are retained below for every signed Track B row.

| Workload / metric | Run 1 | Run 2 | Run 3 | Run 4 | Run 5 | Median | Ceiling | Verdict |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| key / latency (us) | 332.216 | 334.544 | 323.757 | 334.516 | 335.472 | 334.516 | 347.975 | pass |
| key / median working set (bytes) | 70,295,552 | 70,049,792 | 70,316,032 | 70,516,736 | 70,545,408 | 70,316,032 | 88,012,390 | pass |
| key / peak working set (bytes) | 271,441,920 | 271,618,048 | 271,749,120 | 272,031,744 | 271,499,264 | 271,618,048 | 562,033,050 | pass |
| key / median private (bytes) | 33,710,080 | 33,579,008 | 33,869,824 | 34,033,664 | 34,074,624 | 33,869,824 | 39,460,045 | pass |
| session / latency (us) | 37,245.0 | 36,903.4 | 36,604.8 | 36,863.5 | 37,055.9 | 36,903.4 | 39,289.800 | pass |
| session / median working set (bytes) | 50,618,368 | 50,540,544 | 50,171,904 | 50,061,312 | 50,483,200 | 50,483,200 | 66,872,115 | pass |
| session / peak working set (bytes) | 271,441,920 | 271,618,048 | 271,749,120 | 272,031,744 | 271,499,264 | 271,618,048 | 562,033,050 | pass |
| session / median private (bytes) | 28,340,224 | 28,807,168 | 26,775,552 | 28,053,504 | 27,574,272 | 28,053,504 | 32,084,378 | pass |
| warm startup / latency (us) | 35,894.5 | 36,244.6 | 35,761.5 | 36,362.5 | 35,943.3 | 35,943.3 | 38,825.050 | pass |
| warm startup / median working set (bytes) | 67,416,064 | 67,383,296 | 66,977,792 | 67,129,344 | 67,219,456 | 67,219,456 | 86,196,634 | pass |
| warm startup / peak working set (bytes) | 271,441,920 | 271,618,048 | 271,749,120 | 272,031,744 | 271,499,264 | 271,618,048 | 562,033,050 | pass |
| warm startup / median private (bytes) | 32,821,248 | 33,267,712 | 31,223,808 | 32,956,416 | 31,899,648 | 32,821,248 | 37,865,062 | pass |

The 37-character Track A row observed ratios
`1.986 / 1.993 / 2.023 / 2.031 / 1.981`; its median is `1.993` against a
`2.339` ceiling. The 59-character row observed
`1.629 / 1.564 / 1.606 / 1.646 / 1.572`; its median is `1.606` against a
`1.748` ceiling. Both pass.

The candidate snapshot is byte-identical across all five accepted runs:
22,653 bytes, SHA-256
`a38515eab47d661c30ffb1136e41472d2844578125fdf1f150df77d817b1f9f0`.
The root `candidate-snapshots.csv` is an audit copy of that shared snapshot;
the original generated copy remains in every run directory.

## Rejected packets

`rejected/0ad14990/` preserves the complete five-run packet for commit
`0ad14990eda40deff99875ef019787116e3d5792`. It was 31/32: all five Track B
session median-working-set observations were red (`67,158,016`, `66,969,600`,
`67,182,592`, `67,006,464`, and `66,924,544` bytes), producing a
`67,006,464` median against the signed `66,872,115` ceiling.

`rejected/b29f983c/` preserves the aggregate over complete attempts
`run1`, `run4`, `run5`, `run6`, and `run7` for commit
`b29f983cf437d4f623d487e1a52e93d36224b060`. It was also 31/32: the five
session observations were `66,932,736`, `66,883,584`, `67,260,416`,
`67,284,992`, and `66,662,400` bytes, producing a `66,932,736` median against
the same `66,872,115` ceiling. Attempt `run2` stopped during deploy setup with
exit `101` after the C: drive filled; its generated commands and environment
are retained. Attempt `run3` stopped during the release build with exit `101`
before a generated run record was written. The source packet contained no
failure log for either setup attempt, so none was reconstructed.

The rejected subtrees retain their original verdict/provenance sidecars and,
for every complete run, threshold check, summary comparison, environment, and
external provenance. No DLL, Marisa table, product asset, or other binary
payload is copied into this evidence packet.
