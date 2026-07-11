# Increment 4a signed performance ratchet

Verdict: `32/32` aggregate rows pass. All five accepted runs measured
implementation commit `ca52ec427111e2ec36b2a80dfe7b25b6f2d3c456` with:

- Yune DLL SHA-256 `7ed2dc4468524d6e9c21fd5559f4fe6f49f19eb7d90dc6f5d044f200246391e8`;
- upstream `rime.dll` SHA-256
  `86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b`;
- product schema-tree SHA-256
  `1e0c9ccfa2a208af0359a29a6e6b6153dd2d632a977e875858d86c3ce2cdd046`;
- signed-threshold SHA-256
  `e74e77b4dd5b253e0c2b5f4b12cc1e0279784d3c3fbf02006b5f8f18fccacdba`;
- the same complete 17-input Track A and one-input Track B sets; and
- product deployment plus `9 / 60 / 80` startup/session/key iterations.

Replay from the repository root:

```powershell
$root = 'docs/reports/evidence/m59-canonical-jyutping-reachability-parity/increment-4a-sentence-ordering/performance-ratchet'
python -B scripts/aggregate-native-ratchet.py `
  --thresholds docs/reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv `
  --expected-runs 5 `
  --run "$root/run1" --run "$root/run2" --run "$root/run3" `
  --run "$root/run4" --run "$root/run5" `
  --output "$root/gate-verdict.csv"
```

The command exits `0`; [`gate-verdict.csv`](./gate-verdict.csv) has exactly 32
`pass` rows. `gate-verdict.provenance.json` binds five distinct accepted run
paths and their strict provenance inputs. The root candidate snapshot is an
audit copy; every accepted run retains its generated copy, and all six files
have the same SHA-256
`306758a6b5489275b83fda80fcf9772d8250a1992c265c5f6bdd9f0395415385`.

Tracked benchmark text is canonical UTF-8/LF. Canonicalization changes only
CRLF record endings; embedded candidate-comment control bytes remain intact.
The complete raw capture tree is preserved outside the tracked packet.

## Preserved attempts

No completed accepted run was replaced.

- `failed-attempts/run2-attempt0-execution-policy/` records a pre-benchmark
  `PSSecurityException`; no output directory or measurement was produced.
- `failed-attempts/run2-attempt1-space-delimited-input/` preserves a complete
  but invalid invocation. PowerShell bound the 17 values as one space-delimited
  workload, so its threshold check correctly reports 17 signed rows `missing`.
  It is excluded from the aggregate in full and retained with its command,
  environment, measurements, provenance, and disposition.
- Accepted logical run 2 is attempt 2. Its comma-delimited encoding exactly
  matches run 1 and produces all 17 Track A rows.

The capture ran without `-FailOnRegression`, preserving individual observations
instead of aborting or adapting. Every accepted per-run threshold file is itself
`32/32` pass; the median aggregate is still the binding gate.

Only the curated nine text files per run are retained. Raw engine directories,
deploy payloads, `.marisa` files, samples, and logs are excluded.
