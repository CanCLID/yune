# M54 Clean-Room Design

Date: 2026-07-01.

M54 implements native octagram-compatible grammar support in Yune. It does not
implement the librime C++ plugin ABI, dynamic plugin loading, Lua/predict/proto,
contextual translation, AI ranking, frontend work, or public performance claims.

## Source Boundary

The primary behavior source is the checked-in oracle output:

- `crates/yune-core/tests/fixtures/upstream-octagram/lotem-luna-pinyin-octagram.json`
- `crates/yune-core/tests/fixtures/upstream-octagram/rime-lmdg-luna-pinyin-validation.json`

Implementation may use factual compatibility facts needed to consume `.gram`
data: file marker, metadata fields, little-endian numeric layout, Darts
double-array units, gram-key encoding, scaling constants, and schema field
names. It must not copy GPL implementation text, helper structure, or source
layout from `librime-octagram`.

## Runtime Shape

Yune already has the required engine seam:
`crates/yune-core/src/poet/mod.rs` defines `Grammar::query(context, word,
is_rear)` and `NullGrammar`. M54 should add an `OctagramGrammar` provider behind
that trait and keep `NullGrammar` as the default when no model is configured.

Expected module ownership:

- `crates/yune-core/src/poet/octagram.rs` owns `.gram` parsing, gram-key
  encoding, lookup, and scoring tests.
- `crates/yune-core/src/poet/mod.rs` owns threading a `&dyn Grammar` through
  sentence/lattice scoring.
- `crates/yune-rime-api/src/schema_install.rs` owns schema mapping from
  `grammar/language` and penalty fields into the core grammar provider.

## `.gram` Format

The supported M54 subset is `Rime::Grammar/1.0`.

Metadata facts:

- 32-byte NUL-padded format marker.
- `db_checksum: u32`.
- `double_array_size: u32`.
- relative pointer to the Darts double-array payload.
- Darts units are little-endian `u32` values.
- Stored values are quantized log values using scale `10000`.
- Lookup returns at most 8 prefix matches.

Parser behavior:

- Reject files shorter than the metadata header.
- Reject missing or non-`Rime::Grammar/1.0` markers.
- Reject out-of-bounds relative pointers.
- Reject `double_array_size == 0`.
- Reject payload byte lengths that cannot contain exactly `double_array_size`
  `u32` units.
- Validate through `DartsDoubleArray::from_units`.
- Missing model, invalid resource ID, or rejected file must leave the schema on
  `NullGrammar` and record an explicit load failure/deferral note rather than
  silently claiming support.

Memory behavior:

- Native builds should prefer mmap-backed byte ownership for large third-party
  models, matching existing compiled table/prism storage policy.
- WASM builds may use owned bytes because browser/package exposure is outside
  M54; no browser memory claim is made.
- Memory-owner rows should identify octagram model bytes separately from
  `poet.vocabulary` and `poet.entries_by_code`.

## Gram-Key Encoding

The gram trie is keyed by an octagram-specific binary string, not UTF-8 text.
M54 implements this as an independent Rust encoder:

- ASCII code points encode as themselves, except NUL encodes as `0xE0`.
- Code points in `[0x4000, 0xA000)` encode as two bytes unless their low byte is
  zero; the high-byte lane is offset so encoded bytes avoid NUL.
- Other non-ASCII code points encode as a variable-width 7-bit sequence with a
  leading byte in the `0xE0..0xEF` range.
- Encoded-string iteration must advance by this encoded character width, not by
  UTF-8 byte width.

Unit tests should cover ASCII, NUL, common CJK, low-byte-zero CJK, and a
non-CJK code point.

## Scoring Semantics

Configuration fields:

- `grammar/language`
- `grammar/collocation_max_length`
- `grammar/collocation_min_length`
- `grammar/collocation_penalty`
- `grammar/non_collocation_penalty`
- `grammar/weak_collocation_penalty`
- `grammar/rear_penalty`

Defaults follow the octagram-compatible behavior captured by the oracle:
max length 4, min length 3, collocation penalty `-12`,
non-collocation penalty `-12`, weak-collocation penalty `-24`, rear penalty
`-18`, unless schema config overrides them.

Query behavior:

- Empty context or missing model starts from `non_collocation_penalty`; when
  `is_rear` is true, a full-word `$` match may still improve the score.
- Only the last two prior words should contribute to the grammar context for
  M54 sentence/lattice scoring. The full accumulated sentence must not be passed
  as context.
- The word query is truncated to the same encoded-character budget as the
  context side.
- For each suffix of the encoded context, run trie lookup against the suffix plus
  the encoded word prefix and keep the best scaled match plus the applicable
  penalty.
- Use the weak penalty when the collocation is shorter than
  `collocation_min_length` unless the match covers the whole context query and
  whole word query.
- If `is_rear` is true and the full word query has a rear-boundary `$` match,
  consider that value plus `rear_penalty`.
- Preserve existing tie ordering after scores are computed.

## Schema Mapping

M54 supports `grammar/language` only as a logical resource ID that resolves to
`<language>.gram` in selected runtime data. It must not accept arbitrary file
paths.

The named M54 target is upstream `luna_pinyin` with an explicit grammar model.
Schemas with no `.gram` keep current null-grammar behavior and do not load
octagram data. Broader plugin/model behavior remains recorded as deferred.

For Task 1 evidence, both external lanes force
`translator/contextual_suggestions: false`. Contextual translation remains out
of scope.

## Regression Evidence

Implementation closeout must prove:

- Captured octagram fixture rows match Yune candidate text/order.
- Existing upstream `luna_pinyin` null-grammar sentence/lattice fixtures remain
  unchanged.
- Existing TypeDuck `jyut6ping3` behavior remains profile-gated and unchanged.
- `rime_get_api()`, `RimeCandidate`, and TypeDuck profile accessors are
  unchanged.
- Docs distinguish native octagram-compatible grammar support from librime C++
  plugin ABI support.
