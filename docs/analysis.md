# Yune Initial Analysis

## Decision

Do not start with a full librime rewrite. Start with a compatibility harness and
AI extension points, then replace modules only after behavior is measurable.

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

The strongest compatibility progress is currently in two areas:

- Frontend compatibility: sessions, context/status/commit, candidate iteration,
  config access and mutation, deployment helpers, schema lists, module lookup,
  notification callbacks, levers APIs, runtime metadata, plain user dictionary
  operations, and key processing are covered through the ABI surface.
- Config and deployment compatibility: deployed configs now exercise
  librime-style include/patch directives, custom patches, build-info freshness,
  schema deployment, workspace update, task dispatch, and staging/user-data
  behavior.
- Data compatibility: schema-loaded table dictionaries now feed real session
  candidates, and source dictionary parsing handles many librime/yaml-cpp edge
  cases around headers, YAML nulls, quoted scalars, `columns`, `import_tables`,
  duplicate rows, literal hash-prefixed entries, raw text whitespace, and row
  weights.

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
- The schema pipeline is still a subset. Librime's processor, segmentor,
  translator, filter, switch, punctuator, reverse lookup, OpenCC, and schema
  dependency behavior remain larger than the current Yune fixtures.
- Dictionary compatibility currently focuses on source `.dict.yaml` loading.
  Librime also builds and consumes `.table.bin`, `.prism.bin`, `.reverse.bin`,
  pack dictionaries, spelling algebra output, preset vocabulary, stem columns,
  encoder rules, correction data, checksums, and rebuild heuristics.
- User dictionary support is currently a plain text compatibility shim. Librime
  also has LevelDB-backed userdb storage, snapshots, recovery, learning, and
  frequency update behavior.
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
