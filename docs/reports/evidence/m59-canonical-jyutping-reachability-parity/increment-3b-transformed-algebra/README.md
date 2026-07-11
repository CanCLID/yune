# M59 Increment 3b - schema-general transformed-algebra reachability

This packet records the implementation and verification candidate for M59
Increment 3b. The behavior, provenance, deterministic rebuild, and release
checks below are green. The five-round signed performance ratchet is still
pending an uncontaminated Windows measurement window, so this packet is not
yet a landing or M59-closeout claim.

## Mechanism

Yune now builds a deployed-algebra index from surface spellings to canonical
fetch codes. Admission and consumed spans use the surface spelling, while tone,
syllable, and dictionary metadata retain the canonical raw code. Exact,
completion, prefix-fallback, and leading-reachability paths share the same
bounded/eager ordering and duplicate span-promotion rules. The bare-syllable
guard is alias-aware.

The implementation is schema-general and configuration-driven. It does not use
schema IDs, per-input promotion tables, baked oracle candidate lists, or input
allowlists. `leading_syllable_reachability` defaults on and can be explicitly
disabled. Existing product `prefix_fallback` remains an independent behavior.

The compiled prism path preserves spelling type, credibility, correction
provenance, cumulative algebra, identity/null-map semantics, and large Darts
offsets. Bounded cache entries have explicit row, prefix, key-byte, and
entry-byte caps; requests beyond those caps bypass the cache without evicting a
valid entry.

## Oracle and deployment acceptance

The deploy-path matrix covers the seven contract-required schemas plus a Stroke
null-map control:

1. product Jyutping;
2. canonical Cantonese;
3. Cangjie 5;
4. Luna Pinyin;
5. Luna Pinyin Octagram;
6. Double Pinyin;
7. Bopomofo;
8. Stroke identity/null-map control.

Every row is exercised default-on and explicit-false. Positive targets are
proved absent as whole terms from their source lexicons. Double Pinyin
`hknivs` and Bopomofo `cl3su3j06` use fresh pinned-librime whole-input captures;
their expected text was not invented. The external prism property fixtures
cover collisions, correction aliases, erase rules, cumulative credibility,
invalid-algebra identity behavior, source parsing, and compiled-byte reload.

The authoritative oracle/provenance material is under
[`oracle-capture/`](./oracle-capture/) and
[`correction-oracle-text/`](./correction-oracle-text/). The latter includes
source text, raw capture output, independent prism decoding, binary/tool hashes,
and replay commands without copying product binaries into the evidence tree.

## Verified gates

| Command or surface | Result |
|---|---|
| `cargo fmt --check` | passed |
| `cargo clippy --workspace --all-targets -- -D warnings` | passed |
| `cargo test -p yune-core --lib -- --test-threads=1` | 392 passed / 0 failed |
| `cargo test -p yune-core --test upstream_algebra_properties -- --test-threads=1` | 8 passed / 0 failed |
| schema-general default-on/explicit-false deploy matrix | 1 passed / 0 failed; all eight rows in both modes |
| correction source/compiled exact-order regression | passed |
| product `zi` full-surface selection regression | passed |
| WEB-03 compiled-asset launch tripwire | passed |
| WEB-03 long-input candidate-expansion tripwire | passed |
| Stroke traversal controls | 37- and 59-character traversals passed; 84-character miss remained bounded |
| schema-manifest validation | passed; 59 assets |
| public packaging build | passed; 132,569,840-byte payload |

The 37/59/84 controls prove the affected traversal boundary, but they do not
replace the signed performance ratchet.

Two independent review passes covered contract/mechanism compliance and then
code quality, bounded/eager equivalence, cache behavior, compiled storage,
ordering, and test coverage. Findings were fixed forward and the owning gates
were rerun.

## Deterministic product rebuild

[`asset-rebuild/`](./asset-rebuild/) records two independent clean rebuilds of
the same frozen source tree. All 18 compiled artifacts are byte-identical
between runs; exactly 12 tracked artifacts changed and all imported bytes match
the clean rebuild. Manifest-to-tree validation and the public package build are
green. The packet reports the full size increase and contains no binary
payloads.

## Remaining gate and scope boundary

Before Increment 3b can land, five uncontaminated expanded Track A/Track B runs
must pass the owner-signed aggregate-median ceilings and produce
`gate-verdict.csv`. Measurements taken while an unrelated CPU-heavy export is
active are invalid and are preserved separately rather than selected.

This increment does not close M59 or D-48. The heap/source
fuzzy-versus-abbreviation collision remains explicitly owned by Increment 4b,
alongside the exact abbreviation and segmentation family. REACH-03's automatic
manifest-derived one-to-one acceptance row also remains open; this packet's
fixed seven-required-plus-Stroke matrix does not claim that later closeout gate.
