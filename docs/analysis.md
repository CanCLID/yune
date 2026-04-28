# Yune Initial Analysis

## Decision

Do not start with a full librime rewrite. Start with a compatibility harness and
AI extension points, then replace modules only after behavior is measurable.

## Compatibility, Not Cloning

Yune should use librime as the oracle for externally observable compatibility,
not as an internal architecture template. Schema semantics, config behavior,
candidate output, C ABI expectations, deployed data compatibility, and frontend
integration should be compared against librime because existing users and
frontends depend on those contracts.

Internal implementation should still be idiomatic Rust and can deliberately
depart from librime's C++ structure when that produces a simpler or stronger
system. Better typed configuration models, deterministic engine state,
pipeline traits, storage abstractions, dictionary indexes, cache invalidation,
incremental rebuilds, test harnesses, and optional AI extension points are all
valid improvements when they preserve compatibility at the boundary.

When Yune intentionally improves behavior beyond librime, the compatibility
boundary must be explicit. Classic input behavior should remain predictable,
and new behavior should be opt-in or isolated behind Yune-native extension
points unless a real frontend or schema migration requires a different choice.

## Why Not Full Rewrite First

librime's value is not only its C++ implementation. The hard parts are:

- RIME schema semantics and patch behavior.
- Key processing and composition edge cases.
- Translators, segmentors, filters, and their order-dependent behavior.
- Dictionary compilation, lookup, user frequency, and prediction.
- Stable C API expected by frontends such as Squirrel, Weasel, ibus-rime, and
  fcitx-rime.
- Existing plugins such as Lua, octagram, predict, and proto.

Rewriting without fixtures would make it hard to know whether a difference is an
improvement or a compatibility regression.

## Compatibility Layers

Yune should separate compatibility into layers:

1. Schema compatibility: YAML structure, component names, patch/import rules.
2. Runtime behavior compatibility: key sequence to context/candidates/commit.
3. Frontend compatibility: C ABI surface shaped like `rime_api.h`.
4. Data compatibility: ability to consume RIME dictionaries or rebuild them.
5. Plugin compatibility: new Yune plugin ABI first; C++ plugin ABI later only if
   there is clear demand.

The first three layers matter most for adoption.

## First Engineering Target

Build a deterministic runner:

- Input: schema fixture plus key sequence.
- Output: preedit, candidates, highlighted index, commit text, status flags.
- Use this runner to compare Yune behavior with librime output.

Only after this runner exists should core modules be rewritten in Rust.

## Current Compatibility Progress

The initial runner now has a companion frontend-style ABI client. The ABI tests
exercise `yune-rime-api` through the exported `RimeApi` function table, which
keeps the test shape closer to real frontend integration than direct Rust calls.

The strongest compatibility progress is currently in these areas:

- Frontend compatibility: sessions, context/status/commit, candidate iteration,
  config access and mutation, deployment helpers, schema lists, module lookup,
  notification callbacks, levers APIs, runtime metadata, plain user dictionary
  operations, and key processing are covered through the ABI surface.
- Config and deployment compatibility: deployed configs now exercise
  librime-style include/patch directives, custom patches, build-info freshness,
  schema deployment, workspace update, task dispatch, and staging/user-data
  behavior.
- Schema pipeline compatibility: focused librime-style subsets now cover
  schema-loaded `key_binder`, `punctuator`, `recognizer`, `ascii_composer`,
  `speller`, `ascii_segmentor`, `matcher`, `affix_segmentor`, `table_translator`,
  `script_translator`, `r10n_translator`, `reverse_lookup_translator`,
  `history_translator`, `switch_translator`, `schema_list_translator`,
  `simplifier`, `uniquifier`, `single_char_filter`,
  `charset_filter`/`cjk_minifier`, and
  `reverse_lookup_filter`, plus focused full-shape
  `shape_processor`/`shape_formatter` behavior through ABI-facing tests. The current
  `speller` coverage is the processor-level spelling gate for alphabet,
  delimiter, initials/finals, `use_space`, focused `auto_clear` modes, and
  max-code-length preselection before the next initial, plus focused
  `auto_select` exact-table unique-candidate commits and
  `auto_select_pattern` gating, and focused previous-match auto-commit reuse
  with `express_editor`, plus focused `speller/algebra` lookup expansion for
  `xlit`, `xform`, `erase`, and `derive` spelling rules and librime-style
  credibility penalties for generated `fuzz`, `abbrev`, and correction
  spellings. The
  current `schema_list_translator`
  coverage includes
  current-schema-first ordering, selection commands, access-time recency sorting,
  and `switcher/fix_schema_list_order`. Focused editor-variant coverage now also
  includes schema-loaded `express_editor` Return committing raw input instead of
  the highlighted candidate, plus schema-configured `editor/bindings` overrides
  for focused commit, deletion, and `noop` behavior through modified printable
  ABI keys, plus focused `editor/char_handler` defaults and overrides for
  printable-key `direct_commit`, `add_to_input`, and `noop`. Segmentor coverage
  now also includes a
  focused `punct_segmentor` path for shape-mapped single ASCII punctuation keys,
  where the punctuation segment is exclusive and suppresses ordinary table
  translation competition, plus a focused `fallback_segmentor` path where an
  otherwise unclaimed segment is tagged `raw` and does not feed default `abc`
  table translation. Focused `selector` coverage now also honors librime's raw
  segment exclusion so fallback/raw compositions are not committed by numeric
  candidate selection keys, and covers layout-sensitive arrow/page bindings for
  horizontal linear, vertical stacked, and vertical linear candidate lists, plus
  schema-configured selector binding overrides including `noop` removal and
  modified printable accept keys. Focused `navigator` coverage now includes
  schema-configured horizontal/vertical binding overrides, `noop` removal, and
  modified printable accept keys, plus delimiter-derived syllable stops,
  `navigator/syllable_jump_position: before_delimiter`, and the librime
  distinction between looping syllable actions and configured no-loop syllable
  actions at delimiter-derived boundaries. Focused `chord_composer` coverage now
  includes schema-loaded printable chording keys, key-release completion,
  alphabet-order chord serialization, `algebra`/`output_format` projection
  before the generated key sequence feeds the ordinary session pipeline,
  ABI-visible `prompt_format` prompt segments while keys are held, and
  `commit_raw_input` bindings that commit the original raw key sequence, plus
  focused ABI-visible modifier chord options for control, shift, alt, super,
  and caps-lock `Lock` modified printable keys, plus active-chord cancellation
  when a non-chording function key interrupts held chord state, plus raw-sequence
  clearing when generated chord output falls through to direct ASCII commits and
  when an API-level context commit leaves a generated chord composition.
  `punct_number`
  translation now keeps digit separators literal, with full-shape formatting,
  instead of applying the ordinary punctuation mapping after a numeric commit.
  The current shape coverage formats committed ASCII text under `full_shape` and
  post-processes otherwise unhandled printable ASCII keys into full-width
  commits.
- Data compatibility: schema-loaded table dictionaries now feed real session
  candidates, and source dictionary parsing handles many librime/yaml-cpp edge
  cases around headers, YAML nulls, quoted scalars, `columns`, `import_tables`,
  duplicate rows, literal hash-prefixed entries, raw text whitespace, row
  weights, focused `stem` column collection for future reverse/encoder use, and
  focused preset-vocabulary weight lookup/scaling for coded entries. A focused
  table-encoder primitive now matches librime's raw-code formula behavior for
  `encoder/rules`, exclude patterns, and tail-anchor indexing, and source
  `.dict.yaml` encoder settings are parsed into dictionary metadata. The parsed
  rule-based table encoder is now wired into a focused source-build path for
  uncoded dictionary phrase rows and preset-vocabulary phrase injection when
  every phrase character can be translated by coded word entries or stems. A
  focused librime-compatible CRC32 checksum primitive now covers the
  `ChecksumComputer` initial-remainder behavior, dictionary source/preset
  vocabulary checksum ordering, table/prism/reverse checksum rebuild decisions
  used by `DictCompiler`, and focused `.table.bin`/`.prism.bin`/`.reverse.bin`
  metadata checksum/version parsing, but learning, compiled reverse-data
  consumption, and actual compiled rebuild execution remain open.
- ABI edge-case compatibility: recent coverage also locks down struct layouts,
  self-versioned cleanup, unread commit buffering, session lifetime after
  finalize, state-label indexing, selected-schema page-size parsing, deployment
  notifications, and sync notifications.

This does not make Yune a complete librime replacement yet. It does make
frontend and dictionary behavior measurable at a much finer granularity, which
is the intended precondition for replacing more engine modules safely.

## Gaps Against Librime

Compared with librime's current source tree, the remaining gaps are structural,
not just missing tests:

- Real frontend integration is still unproven. The ABI shape is covered by a
  frontend-style client, but clients such as Squirrel, Weasel, ibus-rime, and
  fcitx-rime may expose lifetime, notification, deployment, or session edge
  cases that synthetic tests do not.
- The schema pipeline is still a subset. The current focused coverage now
  reaches many high-value gears, but librime's source tree still has deeper
  behavior around `speller` auto-selection and max-code-length interactions,
  previous-match segmentation, editor variants, deeper `navigator` behavior
  such as full candidate/segment span-aware syllable jumps beyond
  delimiter-derived stops,
  deeper `selector` navigator fallback interaction behavior beyond the current
  focused raw-tag, layout, and configured-binding coverage,
  deeper `editor` segment/selection semantics, deeper `chord_composer` behavior
  such as remaining raw-sequence lifecycle edge cases beyond focused API-level
  context commit handling,
  deeper `shape_processor`/`shape_formatter` interactions,
  deeper `punct_segmentor` behavior such as segment-order interactions and
  `punct_number` through larger chains beyond the focused digit-separator
  translator path,
  deeper multi-segment `fallback_segmentor`, and formatter behavior that are not
  yet equivalently modeled. `speller` still needs deeper previous-match segment
  splitting and non-auto-commit composition behavior.
- Existing schema-loaded translator/filter support is intentionally partial.
  Areas such as full spelling algebra beyond the current focused lookup-side
  `xlit`/`xform`/`erase`/`derive` expansion and generated-spelling ranking
  penalties, correction-driven tolerance search, full OpenCC data and conversion
  chains, distribution-scale schema chains, and compiled-data interactions still
  need direct comparison against librime behavior.
- Dictionary compatibility currently focuses on source `.dict.yaml` loading.
  Librime also builds and consumes `.table.bin`, `.prism.bin`, `.reverse.bin`,
  pack dictionaries at compiled-data level, preset-vocabulary phrase injection,
  deeper stem-column consumption through compiled reverse data and encoders,
  table/prism/reverse binary payloads beyond metadata, `DictCompiler`
  source-vs-prebuilt fallback behavior, pack checksum chaining, reverse-db
  `dict_settings` payloads used by UniTE-style encoders, correction data, and
  rebuild execution.
- User dictionary support is currently a plain text compatibility shim. Librime
  also has LevelDB-backed userdb storage, snapshots, recovery, learning,
  transaction rollback, predictive lookup, backdated scans, and frequency update
  behavior.
- Plugin compatibility remains intentionally out of scope for the first
  milestone. Yune should keep its own Rust extension points separate from
  librime's C++ plugin ABI until a real integration requires it.

## AI Integration Position

AI should be an optional ranking/completion layer, not the foundation of basic
input. Classic candidates must remain available with low latency and without
network access.

Initial AI surfaces:

- Candidate reranking filter.
- Contextual phrase completion translator.
- Personalized user dictionary suggestions.
- Privacy-preserving local model bridge.

## Non-Goals For The First Milestone

- Loading existing C++ librime plugins.
- Full binary compatibility with all compiled RIME data formats.
- Cloud inference as a required dependency.
- A new end-user frontend.
