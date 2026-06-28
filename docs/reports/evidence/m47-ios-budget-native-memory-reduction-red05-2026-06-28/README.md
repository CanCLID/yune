# M47 RED-05 Native Memory Evidence

Windows proxy evidence from the native `RimeApi` lean probe. These numbers are
`WorkingSetSize`, `PrivateUsage`, and test-local allocator counters, not iOS
`phys_footprint`.

Before reference: `../m47-ios-budget-native-memory-reduction-red04-2026-06-28/current/`.

Current run: `current/`.

Key result: deploy no longer owns the cold-start peak. After RED-05, the run is
steady `56.9 MB` WS / `23.7 MB` private / `16.0 MB` allocator-live with `78.4 MB`
peak WS. The after-deploy sample is `8.8 MB` WS with `12.4 MB` peak and `3.7 MB`
allocator high-water.

Remaining blocker: the peak now occurs during `create_session()` at
`m47:compiled_dictionary:jyut6ping3:after_compact_table_store_parse`.
