# M61 Restored-Tree Closeout Test Summary

Verdict: **PASS with targeted recovery from preserved deterministic test-
contract reds.** This is not a claim that the literal `cargo test --workspace`
invocation exited zero.

The source-bound restored-tree runner used commit
`01a62f2a6cd2b3d668545a110de8c7c3fc2fbb10`, tree
`f1c36a0079d85628f5cbef140bd94288930cc2e8`. Its first 19 exact plan gates all
exited zero. Gate 20 ran the literal workspace command and exited `101` after a
successful `596`-test prefix, when four stale all-pages assertions made
`cantonese_parity` report `37 passed / 4 failed`. The raw red has SHA-256
`695dec04fc9fcc5f30b363fb4dedf2c17c31c06171427b633d8f8eee082fbd18`;
the unchanged tracked status receipt has SHA-256
`cd84a7e4d56b7d2f70e160dd0aeb22e2e0b42c03118a8a2298633daa603593ce`.

The bounded recovery sequence was:

1. The first serial Cantonese retry was deliberately interrupted before a
   verdict and is not reused. Its interruption receipt has SHA-256
   `8f90723d167c87ffb46b03169eb8086a24dac3efff7610483239c1dd4c4b2fb0`.
2. Retry 2 passed source-current formatting, serial `cantonese_parity` `41/41`,
   and the real deployed profile-page guard `1/1`.
3. Retry 3 passed exact strict workspace Clippy and the eight never-reached
   core integration targets (`69 passed / 8 ignored`), then preserved a second
   deterministic contract red in the API library (`363 passed / 1 failed / 1
   ignored`). Its red log has SHA-256
   `3ff4a64c0515d2cec0b82b79008a5093e1c61112d4a80080522bd7381281ab29`.
4. Retry 4 passed source-current formatting and exact strict workspace Clippy,
   the complete corrected API library (`364 passed / 1 ignored`), every
   previously unreached API bin and integration target (`114 passed / 3
   ignored`), and the remaining API/core doc groups (`0` tests each). Its
   status and completion receipts have SHA-256
   `e8817e5f73a8d3507d40f2351bc29b91a7e37dd3853191acf91d88aee1a2d1af`
   and `815abfa7112a13263dc5dcf3ebfcc433d15c411744c663b9ffdc64be79a46f2a`.

The nonduplicated successful accounting is `1,184 passed / 12 ignored`: `596`
from the retained workspace prefix, `41` from the complete serial Cantonese
target, `69` from the core tail, `364` from the complete API library, and `114`
from the API bin/integration tail. The `37` passing tests inside the failed
Cantonese target and `363` inside the failed API library are excluded. The
focused deployed-page `1/1` is not added because the full `yune_web` target
contains the same guard.

The two corrections are test-only contract locks. They change no runtime,
candidate behavior, page size, fixture, schema, ABI, measurement, threshold, or
disposition-D result. No production-default final-set gate is claimed.
