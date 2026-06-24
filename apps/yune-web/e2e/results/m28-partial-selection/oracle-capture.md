# M28 Oracle Capture

> **Status:** Captured - **Milestone:** M28 (TypeDuck partial candidate selection) - **Updated:** 2026-06-22 - **Type:** evidence

## Command

A temporary direct `RimeSelectCandidate` harness was run against the local TypeDuck v1.1.2 oracle under `target/typeduck-oracle/v1.1.2` with modules `default,dictionary_lookup` and schema `jyut6ping3_mobile`.

The captured fixture is:

- `crates/yune-core/tests/fixtures/typeduck-v1.1.2/jyut6ping3-m28-partial-selection.json`

## Result

For input `caksijathaacoenggeoizi`, TypeDuck v1.1.2 reports candidate index `1` as `測`. Direct `RimeSelectCandidate(session, 1)`:

- returns success;
- emits no `RimeCommit` at the first-selection step;
- keeps `RimeGetInput` as `caksijathaacoenggeoizi`;
- updates preedit to `測si jat haa coeng geoi zi`;
- recomposes the next candidate page for the remaining active span beginning with `si`;
- does not commit raw `sijathaacoenggeoizi`.

The captured default continuation then selects index `0` (`是日下場句子`) and commits `測是日下場句子`. This diverges from the user feel target `測試一下長句子`; the fixture keeps the TypeDuck v1.1.2 oracle values.

## Yune Bridge Boundary

Yune's TypeDuck-Web/native API emits frontend commits immediately. For M28, selecting `測` now emits only `測` and recomposes the active remaining input `sijathaacoenggeoizi`. The browser/native follow-up path completes the oracle final text through selectable components `是日`, `下場`, and `句子`.

The TypeDuck v1.1.2 one-row continuation candidate `是日下場句子` is recorded in the fixture but is not required as a same-rank one-row Yune candidate for this milestone. That remains sentence-composition/ranking parity, outside M28's raw-tail fix.
