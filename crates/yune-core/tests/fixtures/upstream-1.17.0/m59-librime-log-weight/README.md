# M59 librime compiled-log-weight fixture

This is a purpose-built, non-Yune oracle capture for the M59 default-owned
`luna_pinyin` sentence-model path. The source rows are exact slices from pinned
`rime/rime-luna-pinyin` and `rime/rime-essay`; the pinned librime 1.17.0
deployer compiled them into `luna_pinyin.table.bin`.

The exact binary is stored as `luna_pinyin.table.bin.hex` so it remains a
reviewable text fixture. Decoding yields 4,540 bytes with SHA-256
`8286e67cc60aa78c6e47bf871de130ee570bf6fe7dd99c8cc6b445cad73ea5fb`.
`oracle.json` pins the compiler/runtime identities, source hashes, capture
command, and the full five-candidate page.

The load-bearing case is 蓋: its `gai` reading has `99.91%` of the source
weight while `ge` and `he` each have `0.09%`. librime removes the latter two by
the 5% ScriptEncoder rule before storing natural-log weights in `.table.bin`.
Treating those stored logarithms as linear weights falsely synthesizes `遮蓋`;
the oracle page does not contain it.
