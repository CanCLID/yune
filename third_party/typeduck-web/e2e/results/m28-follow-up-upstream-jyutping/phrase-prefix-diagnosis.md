# M28 Follow-Up Phrase/Ranking Diagnosis

Fixture:
`crates/yune-core/tests/fixtures/upstream-jyutping/jyutping-m28-followup-composition.json`

The accepted hybrid upstream-librime Jyutping fixture captured these first-page
rows for `caksijathaacoenggeoizi`:

1. `測是日下場句子`
2. `測`
3. `惻`

The fixture did not capture a `測試` phrase-prefix row. Under the
fixture-backed-only rule for this follow-up, no `測試` phrase-prefix candidate
was invented or hardcoded.

Red test before the ranking fix:

```text
left: ["測時一下場句子", "測", "惻"]
right: ["測是日下場句子", "測", "惻"]
```

Diagnosis:

- The generation gap was not a missing candidate page row from the accepted
  upstream fixture.
- The measured Yune gap was sentence segmentation/scoring inside the existing
  TypeDuck `jyut6ping3` sentence candidate path.
- Enabling the upstream sentence graph model for Jyutping was rejected during
  implementation because it used spelling-expanded alias rows as character-code
  evidence and generated unrelated fuzzy sentences.
- The landed fix keeps the existing TypeDuck sentence path and raises the
  profile-only `TYPEDUCK_SENTENCE_WORD_PENALTY` from `21.0` to `24.0`, which
  matches the accepted fixture while preserving M21/M24 regression guards.

Focused verification:

```text
cargo test -p yune-core --test cantonese_parity -- m28_followup
cargo test -p yune-rime-api --test typeduck_web -- m28_followup
cargo test -p yune-core --test cantonese_parity -- m21_sentence m21_prediction m24_jigaajiusihaa
cargo test -p yune-core --test cantonese_parity -- m28_partial_selection
```
