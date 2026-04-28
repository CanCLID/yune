# Roadmap

## Completed

### M0: Skeleton

- Rust workspace.
- Core session and candidate types.
- CLI smoke test.
- Initial analysis and architecture notes.

### M1: Compatibility Harness

- Recorded deterministic fixtures, including checked-in composing-state output.
- Run Yune fixtures from CLI.
- Defined JSON output for context, candidates, commit, and status.
- Added workspace tests for deterministic behavior.

### M2: Schema Subset

- Parse a RIME-style schema subset.
- Model processors, segmentors, translators, and filters as named components.
- Support minimal punctuation and echo translation.
- Support selected RIME config patch/include behavior used by compatibility tests.

### M3: Dictionary Prototype

- Implement a simple table dictionary format.
- Support deterministic lookup and candidate ranking.
- Add fixture coverage for pinyin and shape-based schemas.

### M4: AI Hook

- Add a non-blocking candidate reranking trait.
- Provide a mock ranker for tests.
- Keep classic candidate ordering as fallback.

### M5: RIME Frontend Shim

- Implemented a focused RIME-style C ABI subset for sessions, context, status,
  commit, config, levers, schema lists, deployment helpers, modules, runtime
  options/properties, and key processing.
- Added librime-compatible key-table lookup coverage for broad X11 keysym name
  groups, including function keys, keypad keys, modifiers, ISO/XKB/dead keys,
  input-method keys, Latin, kana, Arabic, Cyrillic, Greek, technical,
  publishing/APL, and Hebrew names.
- Aligned core and ABI key handling for navigation, editing, selection, keypad
  keys, Return variants, modifier fallbacks, and
  `menu/alternative_select_keys`.
- Extended simulated key-sequence parsing so known librime key names either
  process as printable characters or parse as ignored no-op events instead of
  failing.
- Validated the compatibility surface with focused Rust tests.

### M6: Frontend-Style ABI And Dictionary Compatibility

- Added a dedicated frontend-style compatibility client that drives
  `yune-rime-api` through the exported `RimeApi` function table instead of
  direct Rust internals.
- Expanded function-table coverage for candidate iteration, version/runtime
  metadata, raw input and caret editing, runtime state, schema labels, config
  load/read/write/update, config cleanup, deployed/schema/user config opening,
  deployment helpers, schema lists, module registration/lookup, notification
  callbacks, levers custom settings, schema selection, switcher hotkeys, and
  user dictionary iteration.
- Added schema-driven table dictionary loading for RIME sessions and validated
  real dictionary candidates through frontend-facing context paging and
  highlight behavior.
- Expanded deployment and maintenance shims for prebuild, deploy, schema/config
  deploy, task dispatch, workspace update, sync, staging freshness, stale user
  copy cleanup, and notification events.
- Expanded config compiler behavior for librime-style `__include`, `__patch`,
  nested references, optional references, list append/merge, explicit root
  patches, custom patches, dependency timestamps, and build-info freshness.
- Expanded levers compatibility for custom settings, switcher schema lists,
  schema metadata, hotkeys, patching list/map config items, and plain text
  user dictionary backup, restore, import, export, iteration, and sync.
- Aligned RIME dictionary parsing with librime/yaml-cpp behavior for required
  header metadata, optional document starts, BOM-prefixed headers, comments,
  quoted mapping keys, mapping-key colons with leading spaces, inline and block
  `columns`, YAML null and quoted-empty values, text-only rows, duplicate rows,
  literal hash-prefixed entries, raw text-column whitespace, and
  numeric-prefix row weights.
- Implemented librime-style `import_tables` handling for schema-loaded
  dictionaries, including YAML null entries, quoted literal names, collection
  entries, quoted collection-shaped names, and double-quoted escape decoding.
- Aligned RIME config hex integer parsing with librime behavior for unsigned
  values that wrap through C `int` conversion.

### M7: Schema Pipeline Compatibility

- Expanded schema-loaded processor coverage for librime-style `key_binder`,
  `punctuator`, `recognizer`, `speller`, and `ascii_composer` behavior,
  including preset imports, namespaced prescriptions, paging and redirect
  bindings, switch option updates, punctuation cycling, paired punctuation,
  digit separators, recognizer `use_space`, speller alphabet/delimiter and
  initials/finals gating, speller `use_space`, focused `auto_clear` modes, and
  max-code-length preselection before the next initial, focused `auto_select`
  exact-table unique-candidate commits, focused `auto_select_pattern` gating,
  focused previous-match auto-commit reuse with `express_editor`, focused
  `speller/algebra` lookup expansion for `xlit`, `xform`, `erase`, and
  `derive` spelling rules plus generated-spelling credibility penalties for
  `fuzz`, `abbrev`, and correction formulas, focused `selector` raw-segment
  exclusion for candidate selection, focused layout-sensitive selector
  arrow/page bindings for linear and vertical candidate lists, focused
  schema-configured selector binding overrides,
  focused schema-configured `navigator` binding overrides, focused
  `navigator/syllable_jump_position` delimiter stops, and delimiter-derived
  navigator syllable loop/no-loop boundary behavior, focused `express_editor`
  Return raw-input commits, focused schema-configured `editor/bindings`
  overrides for commit, deletion, `noop`, and modified printable keys, focused
  schema-configured `editor/char_handler` defaults and overrides for
  printable-key `direct_commit`, `add_to_input`, and `noop`, focused
  `chord_composer` printable-key chord serialization on release with
  `algebra`/`output_format` projection, ABI-visible `prompt_format` prompt
  segments while keys are held, plus `commit_raw_input` bindings for the
  original raw key sequence, focused modifier chord options for control, shift,
  alt, super, and caps-lock `Lock` modified printable keys, focused active-chord
  cancellation on non-chording function keys, focused raw-sequence clearing
  after generated chord output direct-commits ASCII and after API-level context
  commits leave generated chord compositions, plus ASCII mode switch-key handling.
- Expanded schema-loaded segmentor coverage for `abc_segmentor`,
  `ascii_segmentor`, `matcher`, namespaced `affix_segmentor`, focused
  `punct_segmentor`, and focused `fallback_segmentor` subsets,
  including `abc_segmentor/extra_tags` propagation onto the current default
  `abc` segment, recognizer-pattern tags, namespace fallback behavior, sorted
  pattern precedence, raw ASCII tags, exclusive affix-tag behavior for prefixed
  reverse lookup, exclusive single-key shape punctuation tags, focused
  `punct_number` digit-separator translation after numeric commits, and raw
  fallback tagging for otherwise unclaimed input.
- Expanded schema-loaded translator coverage for `table_translator`,
  `script_translator`, `r10n_translator`, `reverse_lookup_translator`,
  `history_translator`, `switch_translator`, and `schema_list_translator`,
  including namespace aliases, tag gating, completion toggles, sentence
  fallback, candidate quality, `dictionary_exclude`, `comment_format`, pack
  dictionaries, focused preset-vocabulary weight lookup/scaling for coded
  source entries, focused rule-based table-encoder phrase injection for uncoded
  source rows and preset vocabulary phrases, persisted switcher options, folded
  switch menus, radio defaults, state-label ABI indexing, schema-list ordering,
  schema selection commands, schema-list access-time recency sorting, and
  `switcher/fix_schema_list_order`.
- Expanded schema-loaded filter coverage for `simplifier`, `uniquifier`,
  `single_char_filter`, `charset_filter`/`cjk_minifier`, and
  `reverse_lookup_filter`, including tag gating, namespace aliases, OpenCC
  config selection, excluded types, tip comments, duplicate removal,
  single-character promotion, extended-CJK gating, and reverse-lookup comment
  updates on completion and sentence candidates.
- Added focused full-shape `shape_formatter`/`shape_processor` coverage for
  ABI commits: committed ASCII table text is converted to full-width text under
  `full_shape`, and otherwise unhandled printable ASCII keys are post-processed
  into full-width commits.
- Expanded source dictionary compatibility to retain librime-style `stem`
  column metadata for coded entries, deduped per entry text, as groundwork for
  reverse data and encoder compatibility, and added a focused Rust table-encoder
  primitive for librime-compatible `encoder/rules` formulas, exclude-pattern
  matching, tail-anchor indexing, raw-code encoding, source `.dict.yaml`
  encoder-setting parsing, and focused phrase lookup/injection through that
  parsed rule-based encoder, plus a focused librime-compatible
  `ChecksumComputer`/dictionary-source checksum primitive and table/prism/reverse
  checksum rebuild-plan primitive, plus focused compiled
  `.table.bin`/`.prism.bin`/`.reverse.bin` metadata checksum/version parsing for
  future compiled data freshness checks.
- Tightened librime ABI lifecycle behavior for `RimeTraits`, session activity
  cleanup, `RimeFinalize`, deployment notifications, `RimeSyncUserData`,
  unread commit buffering, struct-layout coverage, menu page-size parsing,
  and switch state-label lookup.

## Next

- Treat `/Users/trenton/Projects/librime` as the compatibility oracle for
  user-visible behavior, schema semantics, ABI contracts, and migration support,
  but do not clone librime's internal architecture by default. Prefer idiomatic
  Rust designs, cleaner abstractions, stronger typing, deterministic tests, and
  better algorithms where they preserve or intentionally extend the external
  contract.
- Treat the mechanical refactor track in `docs/refactor-plan.md` as complete
  for the current code shape. Keep future compatibility work within the new
  module boundaries, and only split further when a new behavior slice exposes a
  real ownership problem.
- Apply the refactor lesson to all new work: choose the owning implementation
  module, matching test module, and librime comparison target before writing a
  compatibility slice. Keep `lib.rs`/`main.rs` as facades, and extract a
  temporary spike before it becomes the home for multiple related behaviors.
- Build `yune-cli` into a frontend-surrogate input method that drives
  `yune-rime-api` rather than `yune-core` directly: initialize with real shared
  and user data directories, deploy and select schemas, create sessions, process
  interactive keys, render commits/preedit/candidates/highlight/status after
  each event, and provide a transcript replay mode for comparing key sequences
  against librime. The preparatory CLI module split is complete; the remaining
  work is the actual RIME API-backed frontend implementation.
- Run the current ABI against real frontend clients such as Squirrel, Weasel,
  ibus-rime, fcitx-rime, or fcitx5-rime, and record any struct-layout,
  lifetime, notification, deployment, and session-behavior gaps. Treat the CLI
  frontend as a useful intermediate validation layer, not as proof that native
  frontend integration is complete.
- Continue broadening schema coverage beyond the current focused subset toward
  the remaining librime gear components and deeper semantics: deeper
  `abc_segmentor` segmentation beyond current extra-tag propagation, librime
  `memory` memorization hooks, `poet`/`grammar` contextual weighting,
  `contextual_translation`, `unity_table_encoder`/UniTE behavior, `speller`
  previous-match segment splitting and non-auto-commit composition behavior,
  deeper `editor` variant behavior such as full segment/selection semantics,
  deeper `navigator` candidate/segment span semantics, deeper
  `selector` navigator fallback interactions beyond the current focused coverage,
  deeper `chord_composer` behavior such as remaining raw-sequence lifecycle
  edge cases beyond focused API-level context commit handling,
  deeper `shape_processor`/`shape_formatter` interactions, deeper
  `punct_segmentor` behavior such as segment-order interactions and
  `punct_number` translation through larger chains beyond the focused
  digit-separator path, deeper multi-segment `fallback_segmentor` behavior, full
  spelling algebra beyond the current focused lookup-side
  `xlit`/`xform`/`erase`/`derive` expansion and generated spelling ranking
  penalties, full OpenCC conversion data, and larger real-world
  processor/segmentor/translator/filter chains from distribution schemas.
- Expand dictionary compatibility beyond source `.dict.yaml` parsing toward
  librime's compiled `.table.bin`, `.prism.bin`, `.reverse.bin`, pack
  dictionaries at compiled-data level, deeper preset-vocabulary phrase
  injection, stem-column consumption in compiled reverse data and encoders,
  broader phrase lookup around the focused table-encoder primitive,
  source-vs-prebuilt table fallback behavior, pack checksum chaining, reverse-db
  `dict_settings` payloads used by UniTE-style encoders, correction/tolerance
  search data, compiled binary payload parsing beyond the current checksum
  metadata slice, and rebuild execution.
- Expand user dictionary compatibility beyond the current plain text userdb
  shims toward librime's LevelDB/userdb storage, snapshot merging, recovery,
  learning, transaction rollback, predictive lookup/backdated scan behavior, and
  frequency update semantics.
- Keep plugin compatibility explicit: support Yune-native extension points
  first; defer librime C++ plugin ABI compatibility unless there is a concrete
  frontend or distribution requirement.
- Keep AI/LLM-native work as a separate product layer above the compatibility
  foundation: AI can provide candidates, rerank, use context, and maintain memory
  only through source-labeled, local-first, non-blocking interfaces with strict
  timeout/fallback and privacy policy. Do not let AI behavior replace or block
  classic RIME input paths.
