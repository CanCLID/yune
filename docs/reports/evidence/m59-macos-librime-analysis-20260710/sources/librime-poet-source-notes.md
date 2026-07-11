# Pinned librime sentence-path source notes

**Upstream:** `rime/librime`
`33e78140250125871856cdc5b42ddc6a5fcd3cd4` (`1.17.0`). The checkout was
clean and detached at that commit during review.

The report's sentence-path comparison rests on these exact source locations:

- `src/rime/gear/translator_commons.h:179` defaults `max_sentences_` to `1`.
- `src/rime/gear/script_translator.cc:495-503` calls plural `MakeSentences`
  only when `max_sentences_ > 1`; otherwise it calls singular `MakeSentence`
  and installs at most that one sentence.
- `src/rime/gear/script_translator.cc:591-612` exposes the sentence before
  continuing into the phrase iterators used to fill the candidate stream.
- `src/rime/gear/poet.cc:232-252` selects the best line, materializes its
  components only after the winner is known, and uses the single-best
  `DynamicProgramming` strategy when `grammar_` is absent.
- `src/rime/gear/poet.cc:255-352` contains the separate plural beam-search
  implementation used when multiple sentences are explicitly requested.

These are source-review notes, not copied upstream code. Candidate-page truth
comes from the pinned external oracle snapshots already captured in the Yune
fixture/evidence record.
