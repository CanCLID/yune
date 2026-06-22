# M28 Follow-Up Upstream Jyutping Oracle Capture

- Captured: 2026-06-22
- Engine: upstream rime/librime 1.17.0
- Schema source: TypeDuck-HK/schema pinned Jyutping YAML
- Schema commit: 1bed1ae6a0ab48055f073774d7dfd152a171c548
- Module list: default
- Runtime note: stock upstream may log `error creating filter: dictionary_lookup_filter`; dictionary_lookup comments are out of scope for this fixture.
- Scenario: auto_composition_default_before_space

## Before Space

- input: cak si jat haa coeng geoi zi
- preedit: cak si jat haa coeng geoi zi
- preview: 測是日下場句子
- candidates: 0:測是日下場句子, 1:測, 2:惻

## After Space

- processed: 1
- commit_text: 測是日下場句子
- remaining_input: null
- preedit: null

Fixture: `crates/yune-core/tests/fixtures/upstream-jyutping/jyutping-m28-followup-composition.json`
