# Phase 0B Track B Prerequisite Red

Source `f18b0df2d0149bc2a28cd9bd2c075c34030b5568`, tree
`e4ba5201eab8b8fd8cb24ae14dd49a8c9959aa10`, completed owned runs 1 and 2
with all `32/32` signed rows and `17/17` candidate parity. Run 3 retained
`17/17` and passed 31 signed rows, but
`track-b-product/session_create_select_destroy median_private_bytes` measured
`32,727,040 B` against the unchanged `32,084,378 B` ceiling, an excess of
`642,662 B` (about `2.0%`). All 60 session observations were present.

The exact run-3 threshold receipt SHA-256 is
`96a4cf773d47d7172b3f4af976f6fe7b3fb1f65f6287cf201113b5f7b0f499df`.
This corrects a typo in the separately preserved external blocker note; the
underlying CSV and measurement are unchanged. The partial three-round set was
never called a five-round aggregate, never averaged, and never reused. The
owner-authorized Phase 0B packed-syllabary correction received an entirely new
five-round source-bound set.
