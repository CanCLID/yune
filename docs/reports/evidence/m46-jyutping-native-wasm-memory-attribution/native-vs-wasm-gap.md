# M46 Phase 0 Native-vs-WASM Gap

## Headline Gap

| Surface | Jyutping measurement | Value |
| --- | --- | ---: |
| Native Track B peak working set | `phase-0-native/summary.csv` | `504,627,200 B` |
| Native Track B steady resident band | `phase-0-native/summary.csv` | `427,356,160-442,966,016 B` |
| Browser Jyutping WASM linear memory | `phase-0-post-web01-single-schema/summary.csv` | `936,509,440 B` (`893.1 MiB`) |
| Browser clean/schema-switch Jyutping capture | `phase-0-schema-switch-current/summary.csv` | `936,509,440 B` (`893.1 MiB`) |

Native Track B and browser WASM are related but not interchangeable. Native can
map selected table/prism bytes from files; Emscripten/WebAssembly has to expose
compiled assets through browser storage and linear-memory structures. Movement
in a native file-backed owner does not automatically reduce
`HEAPU8.buffer.byteLength`.

## Owners That Do Not Transfer Directly

| Native owner/status | Native value | Browser evidence | Transfer read |
| --- | ---: | --- | --- |
| `compact_table.storage` mmap-backed base table | `15,248,382 B` | Jyutping core/full/extras all reach `893.1 MiB` | Native mmap movement is unlikely to move WASM high-water by itself. |
| `jyut6ping3_scolar` source/status bytes | `27,325,622 B` | Scolar family changes requested bytes but not heap | Source/status bytes are payload pressure, not proven linear-memory owner. |
| `compact_table.syllabary_codes` | `4,189,674 B` | No family row changes high-water | Code-string interning is too small to explain either headline. |
| Candidate text/comment logical payload | `2,575,292 B` | No family row changes high-water | Visible owner but not a headline branch yet. |

## WASM-Specific Pressure

The browser evidence points to a deeper WASM runtime/engine materialization
owner:

- Luna settles at `160.0 MiB`, while Jyutping settles at `893.1 MiB`.
- Jyutping remains `893.1 MiB` when the requested family changes from
  `0 B` extras through `4,630,356 B` core to `26,429,822 B` full Jyutping.
- JS heap is only about `5-6 MB` in the asset-family rows, so the headline is
  not ordinary browser JS object growth.
- Native selected storage uses mmap and zero heap mirrors, but browser WASM
  cannot use Windows file mappings inside linear memory.
- Emscripten allocation/growth behavior can leave the heap at the maximum size
  reached during schema initialization even after temporary objects are no
  longer live.

## Expected Transfer

Native owner changes are expected to transfer to browser WASM only if they
reduce Rust/Emscripten heap materialization during Jyutping initialization and
the browser high-water is measured again. The current Phase 0 evidence does not
authorize a native-only owner rewrite as a browser memory win.

Likely non-transfer or low-transfer branches:

- Track B `rsmarisa` code-string spike: owner is `4.2 MB`.
- Payload pruning by asset family: already measured no-go for high-water.
- Native mmap/status byte movement: may reduce disk/resource bytes but not
  `HEAPU8.buffer.byteLength`.

Potential future transfer branches require more proof:

- reducing required lookup-record materialization if dictionary panels can lazy
  load it safely;
- fixing schema-switch lifecycle if retained stale engine/session state is
  forcing duplicate initialization;
- adding allocator/high-water instrumentation inside the WASM runtime so the
  unclassified `893.1 MiB` row can be split.
