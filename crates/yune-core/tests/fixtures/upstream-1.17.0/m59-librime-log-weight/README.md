# M59 librime compiled-log-weight fixture

This is a purpose-built, non-Yune oracle capture for the M59 default-owned
`luna_pinyin` sentence-model path. The main source rows are exact slices from
pinned `rime/rime-luna-pinyin` and `rime/rime-essay`; a small 94%/5%/1% 足
control is purpose-built to bite ScriptEncoder's exact boundary. The pinned
librime 1.17.0 deployer compiled all rows into `luna_pinyin.table.bin`.

The exact binary is stored as `luna_pinyin.table.bin.hex` so it remains a
reviewable text fixture. Decoding yields 4,744 bytes with SHA-256
`34784ffd5af9bdc79926a00057cbf8c201a64473a2334acd748685e2d1fd6405`.
`oracle.json` pins the compiler/runtime identities, source hashes, capture
command, the full five-candidate page, and both boundary-control pages. The
preserved external raw candidate CSV has SHA-256
`98a9eebcc0b286a9cf6b5691dfd2d5080fedea1c0f170de9e00fa7b5a7e65f7c`.

The primary load-bearing case is 蓋: its `gai` reading has `99.91%` of the
source weight while `ge` and `he` each have `0.09%`. librime retains all three
original word entries in `.table.bin` and stores their natural-log weights, but
`EntryCollector::TranslateWord` excludes the two low-share readings from
ScriptEncoder phrase expansion. Treating the stored logarithms as linear
weights falsely synthesizes `遮蓋`; the oracle page does not contain it.

The boundary control gives 足 the readings `zu=94%`, `ju=5%`, and `zhu=1%`.
Pinned librime includes `ju` in phrase expansion, so `changju` visibly contains
`長足`; it excludes `zhu`, so `changzhu` does not. This externally compiled
pair proves that the 5% comparison is inclusive.
