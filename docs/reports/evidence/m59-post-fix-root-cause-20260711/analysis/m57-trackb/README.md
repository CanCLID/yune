# M57-to-current Track B audit

## Answer

Track B behavior is unchanged: both M57 complete passes and all five accepted
current rounds produced the exact same five candidate rows, including comments
and page state. The raw candidate CSV SHA-256 is `5fe17ccb53dd8ee40d9ceeb00dc9c7aab0cc30e3dfdf8216914cf675b7e597e9`
for all seven observations.

The underlying work and memory shape did change. All five current rounds agree
exactly on every non-timing M37 counter, so this is deterministic work-shape
movement rather than round noise. Product checksums, compiled readiness, mmap
storage, entry counts, and no-source-fallback status remain stable. The two
compiled Track B table payloads are each 28 bytes larger; their logical
checksums and entry counts are unchanged.

## Scope and grain

- Input: `neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung`.
- M57 reference: `full-pass-1` is the prior report's named anchor;
  `full-pass-2` is retained as a second historical observation.
- Current evidence: `accepted-baseline/run-1` through `run-5`, Yune
  `afb7079b71f7f9353845114ff3e310c0a38b9b87`.
- Schema: `jyut6ping3_mobile`; Track B is a Yune product guard, not a
  Yune-versus-librime peer benchmark.
- Each latency row contains 80 key-sequence samples, 60 session samples, and
  9 startup samples. Each key-sequence sample processes 61 keys.

## Candidate and product status

- Candidate rows: byte-identical across both M57 passes and all five current
  runs (`1 M57 hash`, `1 current hash`).
- First page: `你個人經其實應該支援超場句子輸入先可以用`, `你個`, `你`, `呢`, `尼`.
- Both dictionaries retain the same source/table checksums:
  `jyut6ping3` = `0xf6589c0c`; `jyut6ping3_scolar` = `0x822bccba`.
- Both remain `fresh`, compiled-ready, byte-backed, mmap/mmap, with no source
  fallback and zero table/prism heap mirrors.
- `jyut6ping3` remains 127,143 stored/expanded entries and 114,653 codes;
  `jyut6ping3_scolar` remains an empty table owner in this profile.
- Product-field changes: 2 rows, all accounted for by the
  two 28-byte `byte_source_len` changes.

## Non-timing counter movement

| Counter | M57 pass 1 | Current (all five) | Change |
| --- | ---: | ---: | ---: |
| `track_b_spelling_expansions_considered` | 377280 | 505360 | +33.9% |
| `track_b_exact_lookup_calls` | 9520 | 9920 | +4.2% |
| `track_b_prefix_lookup_calls` | 9440 | 9440 | +0.0% |
| `track_b_candidates_materialized` | 3360 | 6560 | +95.2% |
| `bounded_iterator_selected_total` | 4400 | 9280 | +110.9% |
| `bounded_iterator_full_count_total` | 451600 | 454480 | +0.6% |
| `owned_candidates_materialized` | 337440 | 174560 | -48.3% |
| `candidates_sorted` | 300800 | 146080 | -51.4% |

There are 32 changed non-zero counter rows in the complete
CSV. Two scopes move in opposite directions: Track-B-specific materialization
and bounded selection increase, while global owned candidates and sorting fall.
They are not additive definitions and must not be used as a single work total.
Candidate behavior remains exact despite the changed internal path.

## Normalized memory-owner shape

- 12 owner occurrences are newly reported: six guarded
  translator owners for each of the two translator dictionaries. Their retained
  estimates are zero or 48 bytes; they are structural/accounting additions, not
  a large retained-memory regression.
- 6 non-process owner occurrences changed shape. Product
  status records both compiled tables at +28 bytes, while the owner ledger
  reflects +28 bytes for the main compact-table mapping. The two prism mapping
  pairs grew by 13,327,264 bytes in total, and the schema reload signature grew
  by 679 bytes.
- `poet.entries_by_code`, `poet.lookup_index`, and
  `poet.abbreviation_vocabulary` remain absent/shared-zero for both translators.
- Current peak resident proxy spans 444,940,288 to 468,123,648 bytes, versus
  741,736,448 to 752,746,496 bytes in the two M57 passes. This is same-platform
  observational evidence, not an additive owner total or a causal attribution.

## Latency observations

| Workload | M57 pass 1 | M57 pass 2 | Current five-run median | Current spread | Current vs M57 center |
| --- | ---: | ---: | ---: | ---: | ---: |
| Key sequence (µs/key) | 286.514 | 291.258 | 264.941 | 1.8% | -8.3% |
| Session lifecycle (µs) | 28561.833 | 29242.958 | 29234.292 | 0.5% | +1.1% |
| Startup (µs) | 28608.292 | 37937.375 | 29385.500 | 0.8% | -11.7% |

The current key-sequence median is stable and roughly 8% below the two-pass M57
center. Runs 4 and 5 nevertheless contain high-tail samples (pooled worst
`642.907 µs/key`), so the median improvement
does not justify a tail-latency claim. Startup is especially unsuitable for a
source-change conclusion because the two M57 medians themselves differ sharply.

## Evidence quality and caveats

1. The current five-run source and Yune binary are clean and fixed; the Yune
   dylib hash is `3dd5a414c68f7884884c5dc172b3f0b088d1f5ae19cb983eb0eeb2f95bc6c710`.
2. M57 recorded `c6749cc6` plus dirty modifications in the exact files later
   committed by M57 closeout `a87c6b88`. It is accepted historical evidence,
   but it is weaker clean-commit provenance than the current packet and does not
   record an equivalent five-run dylib hash.
3. Source commits differ, so counter, owner, and latency movements are
   descriptive. They cannot be attributed solely to the Luna page-order repair.
4. Absolute macOS resident memory includes allocator, mappings, loader state,
   and overlap. It is not interchangeable with Windows memory counters.
5. Source/table checksums do not identify the prism artifact. The stable table
   checksums therefore do not erase the separately observed prism-shape change.
6. Comments are compared byte-for-byte through the candidate CSV hash. The
   human-readable table abbreviates them only for readability.

## Files

- `track-b-candidate-audit.csv`: seven exact candidate observations.
- `track-b-product-status-diff.csv`: complete normalized product/checksum fields.
- `track-b-memory-owner-diff.csv`: owner rows normalized without paths/session IDs.
- `track-b-counter-diff.csv`: all non-zero non-timing M37 counters.
- `track-b-latency-observations.csv`: all raw summary observations.
- `track-b-latency-summary.csv`: two-pass M57 and five-run current aggregates.
- `source-manifest.csv`: exact source paths, sizes, and hashes.
- `analyze.py`: reproducible read-only transformation.
