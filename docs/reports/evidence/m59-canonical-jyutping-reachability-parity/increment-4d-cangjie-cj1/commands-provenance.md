# Increment 4d commands and provenance

## Canonical identities

- Yune commit: `38e759f6ac0c79512713c33533df465e908538db`
- Yune tree: `948504ed5ef79771cf647e8ad9429fcfb5f43fde`
- Yune release DLL SHA-256:
  `a0fe13b7a5df3669d09425f48adaa5e2821ac578df00962cffc2d95a0420652e`
- librime: `33e78140250125871856cdc5b42ddc6a5fcd3cd4`
- official `rime.dll` SHA-256:
  `86b4c7357d4c6d293ce5589b234d8859ca2ac30923a03bedfa3926eeaf97fb0b`
- official Windows main archive SHA-256:
  `7478c7caa4ff6b37de86daba1f7ce4a994a4f5ba24872a820fb2b3a9b01fed15`
- official Windows dependency archive SHA-256:
  `9ef5608d8a54ff52bbad7a9b4128de42b232f8e3dd1f5fd3bff42a0b1bacd7e8`
- pinned Cangjie oracle fixture SHA-256:
  `24408c3b2b83db516ae1382d2ba743b41ead50c7c026aee2837a01137c7ecbcf`
- competing fixture SHA-256:
  `dbb9b3a1a5e6fcec4357914d7f6cedcdbca1c90e1bd40350b21cf475fa2e5122`
- competing-fixture generator SHA-256:
  `1d652dfbac827b2fa12305ce2251cedab1a246426f4e2e480894f3473b8ceed5`
- staging tool SHA-256:
  `1a1b68396e82bf7f451656012df18f7e695a648009b0b4ffbe6ad511b419d65f`
- historical capture tool SHA-256:
  `c2614bc7f068d89903d7b0af3856f286f07194e35f505806251aa2ba887aa45c`
- strict comparator SHA-256:
  `d20eccc78822dd612eefd39966586a5c87cd5bbe8be4386634a20c52c139f612`

The untouched captures retain their full effective invocations and external
paths. The staging manifest retains the exact source/staged identities. The
five performance runs retain commands, effective invocations, environment,
source/binary/tree bindings, and benchmark receipts under
`performance-ratchet/run1` through `run5`.

The two official archive hashes were independently checked against the GitHub
API asset digests before extraction. Only their text provenance is recorded;
the archives and extracted binary payloads are not copied into this packet.

## Public-API 504/504 audit

```powershell
$env:CARGO_TARGET_DIR='C:\m59-4d-kcount-audit-target'
cargo run --manifest-path C:\m59-4d-kcount-audit\Cargo.toml -- C:\m59-4d-final-marked-24eb2348\work\user\build
```

Result:

```text
syllabary_count=19975
owned_canonical_k_count=504
byte_backed_canonical_k_count=504
```

The packet copies the audit's `Cargo.toml`, `Cargo.lock`, and `src/main.rs` as
text under `audit/`; no audit target or deployed binary is copied.

## Performance aggregate

```powershell
python scripts/aggregate-native-ratchet.py `
  --thresholds docs/reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv `
  --expected-runs 5 `
  --run C:/m59-4d-perf-final-38e759f6/run1 `
  --run C:/m59-4d-perf-final-38e759f6/run2 `
  --run C:/m59-4d-perf-final-38e759f6/run3 `
  --run C:/m59-4d-perf-final-38e759f6/run4 `
  --run C:/m59-4d-perf-final-38e759f6/run5 `
  --output C:/m59-4d-perf-final-38e759f6/gate-verdict.csv
```

The provenance document records tool version 8, tool SHA-256
`ce36704f6524e70887d465713bec7bf5e1dcd8ddf9b4858b238b1cd252b7c4bf`,
unchanged threshold SHA-256
`e74e77b4dd5b253e0c2b5f4b12cc1e0279784d3c3fbf02006b5f8f18fccacdba`,
and `build,reuse,reuse,reuse,reuse` binary identity.
