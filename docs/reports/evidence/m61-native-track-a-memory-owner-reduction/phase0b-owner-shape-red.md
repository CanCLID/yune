# Phase 0B Exploratory Owner-Shape Red

Source `67d32a2bea36a391a8a11ea4e725dbfebe118252`, tree
`7e2157b5de2575728f2632fad184a05403342a13`, first passed five fresh owned
rounds with `32/32` signed rows, `160/160` observations, `17/17` candidate
parity, and green Track B guards. Its separately named exploratory byte-backed
round also retained `17/17` and measured a `116,314,112 B` Track A peak, but
the exact owner-shape gate completed red.

The four required mapped `YUNE-POET/3` rows were present and one forbidden
fifth POET row remained:

```text
owner_id=poet.normal_character_code_index
retained_estimate_bytes=11538
byte_class=heap_owned_guarded
non_overlapping_reducible_bytes=0
mapping_mode=sorted Box<[String]>
```

`initial-exploratory-owner-red.csv` is the compact exact five-row POET excerpt;
the full raw owner profile SHA-256 is
`3a8aa717e538b91cdd5bd97966524acb66ac7c42dc589504f72d1a66f3bd3649`.
The red was not retried. It selected the single bounded borrowed-membership
correction measured at `91f59696`.
