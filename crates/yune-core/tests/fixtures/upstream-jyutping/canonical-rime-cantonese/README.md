# Canonical rime-cantonese fixtures

This directory contains whole-input fixtures captured from upstream
`rime/librime` `1.17.0` at
`33e78140250125871856cdc5b42ddc6a5fcd3cd4` with the pinned canonical
`rime/rime-cantonese` schema and its exact dependencies.

`jyutping-m59-being-whole-input.json` records the complete paged oracle result
for `being`. The authoritative first candidate is `畀嗯`, with preedit
`bei ng`. The capture also proves that `畀嗯` is absent as an exact term from
all six canonical Jyutping dictionary source files and `essay-cantonese.txt`.
Every exact source row for the two oracle-top constituents, `畀` and `嗯`, is
preserved with its file, line number, and source-file hash.

The fixture is a canonical upstream-engine/schema oracle. It is not a TypeDuck
profile fixture and does not derive an expected string from Yune. The capture
was made in a shared dirty Yune worktree, so the fixture says so explicitly;
canonicality is scoped to hash-pinned librime binaries, exact clean upstream
schema repositories, and the hash-pinned capture/probe bytes.

The recorded command in the fixture is the authoritative invocation. For a
replay, replace only `-Output` with a new path under `target/`; the script uses
create-new output semantics and refuses to overwrite existing evidence.
