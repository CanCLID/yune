# M28 History Classification

> **Status:** Captured - **Milestone:** M28 (TypeDuck partial candidate selection) - **Updated:** 2026-06-22 - **Type:** evidence

## Result

The `caksijathaacoenggeoizi` partial-selection behavior is previously missing support, not a regression.

## Evidence

Commands:

```powershell
git log --oneline -- crates/yune-core/src/engine.rs crates/yune-core/src/state.rs
git log -L 1043,1072:crates/yune-core/src/engine.rs --oneline
git log -S "commit_text_for_input" --oneline -- crates/yune-core/src crates/yune-core/tests
git log -S "PartialTable" --oneline -- crates/yune-core/src crates/yune-core/tests
```

Findings:

- `1d5098da Split yune-core engine` introduced the selection path with `segment_start = 0`, `segment_end = input.len()`, `record_commit_with_metadata(...)`, `clear_composition()`, and `Some(text)`.
- `8c565560 M9 typeduck web hr5 hr7 (#3)` added commit intent handling, but kept full-input selection and `clear_composition()`.
- `cd5122a7 fix(fork-parity): preserve Cantonese engine parity` added learning-code preservation, but kept full-input selection and `clear_composition()`.
- `a9ae285d fix: close M21 gap 02 typeduck parity` changed candidate commit text from `candidate.text` to `candidate.commit_text_for_input(&input)`, which made `PartialTable { consumed }` append the raw suffix for commit preview/commit text. It did not add segment-aware recomposition.
- `PartialTable` metadata exists from M17/M21-era translator work, but `Engine::commit_candidate` did not use it to set a consumed segment or preserve a remaining composition.

Conclusion: M28 is feature completion for TypeDuck-profile segment-aware selection, not a regression fix with a last-known-good Yune commit.