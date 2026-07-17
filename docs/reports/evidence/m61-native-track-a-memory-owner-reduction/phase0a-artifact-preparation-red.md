# Phase 0A Exploratory Artifact-Preparation Red

Source `a39c4d868820063dc3deaa42f7fdc9b3aee5e7a6`, tree
`76089019f142a7f76c05bcd745ab30a10af280a6`, had already completed a green
five-round owned set. Its separately named exploratory byte-backed attempt then
failed before usable Yune candidate pages because the generated POET sidecar
was bound to the wrong table/header source checksum. The strict candidate
checker consequently reported a fail-closed shape error with all 17 Yune page
groups absent; this was an artifact-preparation measured red, not a
candidate-behavior comparison.

- Yune DLL SHA-256:
  `957080d019637392e5f5dc0333684d1c85d8b8fa36e1343e267edc443c4c0ef0`;
- benchmark executable SHA-256:
  `0c329a56803acfc62b6663a1bc4a8e9d1f0bdc26796ca3b85086bb3acfee48f7`;
- candidate verdict SHA-256:
  `cbf78e047f00def67c7ee5d78c7263d81b0a47658dbd37f013b9578c1158f886`;
- run-status SHA-256:
  `94ceba20ee8ee9467199ca3c24c91adb498a2a1b85364cfbe72756b4e164a67c`.

The result was preserved externally. No round from this source was reused after
the source-clean POET binding repair.
