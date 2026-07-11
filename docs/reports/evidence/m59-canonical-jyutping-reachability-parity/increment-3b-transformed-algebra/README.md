# M59 Increment 3b - schema-general transformed-algebra reachability

This packet records the accepted implementation and verification evidence for
M59 Increment 3b. The schema-general behavior, provenance, deterministic
rebuild, release checks, and owner-signed five-round Windows ratchet are green.
The implementation is frozen at `2cb7e411`; this closes M59-REACH-02, but it
does not close M59 or any D-48 ordering lane.

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

The landing repair visits large prism alias families in descriptor order and
stops only at the bounded request window. It never treats the 64-prefix cache
eligibility threshold as a behavior cap: filtered aliases beyond 64 remain
reachable, while unbounded/PageDown translation remains complete. Prism
serialization now uses compact deterministic trie nodes and flat descriptor
storage; a fresh full product rebuild remained byte-identical.

The final native repair keeps the compiled-prism corruption contract intact
without retaining validation pages in the runtime working set. Native schema
installation maps the same open immutable prism file twice, performs the full
range/tip/NUL/UTF-8 validation through a temporary view, retains only numeric or
owned layout plus a cold runtime view, and then drops the validation view.
Malformed present artifacts still fail closed with no source fallback. The
ordinary owned-byte/WASM parser remains single-source, and no ABI, schema, or
compiled format changed.

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
| `cargo test -p yune-core --lib` | 405 passed / 0 failed |
| `cargo test -p yune-rime-api --lib tests::dictionary_data` | 18 passed / 0 failed |
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

## Signed performance acceptance

[`performance-ratchet/`](./performance-ratchet/) preserves the five complete,
non-cherry-picked 17-input Track A plus Track B rounds from `2cb7e411`, the fixed
DLL hash, per-run provenance, and executable aggregate verdict. All 32 aggregate
rows pass, with all 32 rows green in every individual run. The Track B latency
median is `334.516 us` against `347.975 us`; key working-set median is
`70,316,032` against `88,012,390` bytes; deploy-peak median is `271,618,048`
against `562,033,050` bytes; and session working-set median is `50,483,200`
against `66,872,115` bytes. The 37-character row is `1.993x <= 2.339x`; the
59-character row is `1.606x <= 1.748x`.

All five candidate snapshot files are byte-identical (SHA-256
`a38515eab47d661c30ffb1136e41472d2844578125fdf1f150df77d817b1f9f0`),
including the five-row Track B product page.

The packet also preserves two rejected predecessors instead of hiding their
measurements: `b29f983c` missed the session-working-set ceiling after two
disk-full setup failures, and `0ad14990` produced five valid rounds whose
session-working-set median was `67,006,464 > 66,872,115` bytes. Neither rejected
packet was re-baselined or selected into the accepted five-run aggregate.

## Deterministic product rebuild

[`asset-rebuild/`](./asset-rebuild/) records two independent clean rebuilds of
the same frozen source tree. All 18 compiled artifacts are byte-identical
between runs; exactly 12 tracked artifacts changed and all imported bytes match
the clean rebuild. Manifest-to-tree validation and the public package build are
green. The packet reports the full size increase and contains no binary
payloads.

After the compact-builder repair, a new full product deployment regenerated
`jyut6ping3_mobile.prism.bin` at `32,140,173` bytes with SHA-256
`26d30c52ea35c6d72e63f0e8261f2ca4a6b6e43a82e1572b0a3c803ec29fe6b2`,
exactly matching the tracked pre-repair Increment 3b artifact.

## Remaining M59 scope boundary

Increment 3b is accepted; Increment 4a is next. This increment does not close
M59 or D-48. The heap/source
fuzzy-versus-abbreviation collision remains explicitly owned by Increment 4b,
alongside the exact abbreviation and segmentation family. REACH-03's automatic
manifest-derived one-to-one acceptance row also remains open; this packet's
fixed seven-required-plus-Stroke matrix does not claim that later closeout gate.
