# Independent macOS Verification of the Yune-vs-librime Performance Reports

Date: 2026-07-04 · Verifier: independent session (this machine)

This is an independent re-check of the two Windows-authored performance reports
— [`yune-vs-librime-performance.md`](../../yune-vs-librime-performance.md) and
[`yune-vs-librime-root-cause-analysis.md`](../../yune-vs-librime-root-cause-analysis.md)
— on macOS / Apple Silicon, against a locally built upstream librime oracle. It
re-runs the benchmark (a third run, alongside the two prior runs in this
bundle), re-derives every claim from the raw CSVs, profiles the hot path, and
adversarially checks the conclusions.

## Machine and build provenance

- **Host:** MacBook Air, Apple Silicon (arm64), macOS 26.5.1 (Darwin 25.5.0).
- **librime oracle:** built locally from `/Users/laufei/Documents/GitHub/librime`,
  pinned to `33e78140250125871856cdc5b42ddc6a5fcd3cd4`, `CMAKE_BUILD_TYPE=Release`,
  arm64 non-fat. Symbol `_rime_get_api` present.
- **Yune:** `cargo build --release` — `opt-level=3, lto=true, codegen-units=1`,
  arm64 non-fat. Same `luna_pinyin` schema data, deployed by the same
  `rime_deployer`.
- **Harness:** `native_inprocess_benchmark`, identical timing loop to the
  Windows reports (per keypress: `process_key` then `get_context/free_context`;
  `median_us` = `total_us / char_count`). The only source change vs the Windows
  harness is macOS memory sampling, and it is sampled **outside** the timed
  region — verified from the diff.

## Fairness

**FAIR.** The timed loop contains no per-engine branching; both engines run the
identical path. Both dylibs are optimized arm64 (no Rosetta). Same schema data,
same pinned oracle, same iteration counts (9 / 60 / 80). The one asymmetry — the
Yune build dir receives a 31 MB `luna_pinyin.poet.bin` that librime does not —
makes Yune do *more* setup, so it cannot flatter Yune. (In the default config
that `.bin` is ignored anyway; see the config note below.)

## Reproducibility

Three runs (`run-1`, `run-2`, `independent-run-3`) agree tightly. Per-key median
Yune/librime ratios:

| Dimension | Windows report | run-1 | run-2 | run-3 | Status on macOS |
| --- | ---: | ---: | ---: | ---: | --- |
| 37-char sentence | `1.913x` | `61.01x` | `60.68x` | `58.79x` | direction holds, magnitude ~31x worse |
| 59-char sentence | `1.528x` | `43.95x` | `42.37x` | `40.69x` | direction holds, magnitude ~27x worse |
| `cszysmsrsd` (10-char abbr) | `0.381x` **win** | `5.02x` | `5.08x` | `4.97x` | **CONTRADICTED — win becomes ~5x loss** |
| `zybfshmsru` (8-char abbr) | `0.564x` **win** | `7.04x` | `7.34x` | `7.25x` | **CONTRADICTED — win becomes ~7x loss** |
| `zhongguo` (common word) | `0.255x` **win** | `0.242x` | `0.247x` | `0.229x` | **HOLDS** (Yune faster) |
| `n` | `2.636x` | `1.044x` | `1.107x` | `1.063x` | direction holds, magnitude far smaller |
| `ni` | `2.433x` | `0.875x` | `0.895x` | `0.969x` | **CONTRADICTED — becomes a tie/slight win** |
| `hao` | `1.574x` | `1.833x` | `1.392x` | `1.252x` | holds (noisy row) |
| startup | `0.895x` | `0.500x` | `0.576x` | `0.588x` | direction holds (report flags run-noisy) |
| session | `0.864x` | `0.521x` | `0.597x` | `0.616x` | direction holds (report flags run-noisy) |
| Track A peak memory | `185.7 MB` vs `13.5 MB` | ~`448 MB` vs ~`15 MB` | — | — | direction holds; Yune peak ~2.4x higher |

## The load-bearing finding: same-config, the owned poet is ~18x slower on macOS

Both the Windows dashboard and every macOS run use the **shipping default**
(owned poet). This is verified: `compiled_poet_consumption_enabled()`
([`schema_install.rs:2074`](../../../../crates/yune-rime-api/src/schema_install.rs))
loads the byte-backed `poet.bin` **only** when `YUNE_POET_BYTE_BACKED=1`; that
env var is never set on macOS, so the owned sentence model runs — exactly the
dashboard's "shipping default, owned poet, 185.7 MB" lane.

Comparing the **same config** against the checked-in Windows owned-default
evidence (`m55-native-match-or-beat/corrective-2026-07-04/run-c-owned-default/`),
per-key `translator_median_us`, with the dictionary table on
`rsmarisa_byte_backed`/`mmap` on **both** platforms:

| Row | Windows-owned translator | macOS-owned translator | ratio | raw table lookup (Win → mac) |
| --- | ---: | ---: | ---: | ---: |
| `zhongguo` | `39.4 us` | `23.1 us` | `0.59x` (mac **faster**) | `2.2 → 3.0 us` |
| 37-char | `569.1 us` | `10,461.1 us` | **`18.4x`** | `30.4 → 46.1 us` |
| 59-char | `1,011.0 us` | `17,854.4 us` | **`17.7x`** | `37.8 → 52.7 us` |
| `cszysmsrsd` | `457.7 us` | `4,334.2 us` | **`9.5x`** | `0.8 → 1.1 us` |
| `zybfshmsru` | `463.2 us` | `4,240.8 us` | **`9.2x`** | `0.8 → 1.1 us` |

The dictionary/table lookup is comparable on both platforms, and the short
common word is *faster* on macOS. The 9-18x gap is isolated **entirely** to the
owned poet sentence-translator on long / wide-lattice inputs.

The headline ratio (58.8x, vs the report's 1.9x) is larger than 18x because two
platform shifts stack: **Yune's owned translator is ~18x slower on macOS** *and*
**librime is ~1.7x faster on macOS** (37-char librime `295 us` Windows → `177 us`
macOS — Apple Silicon is simply fast). `18 × 1.7 ≈ 30`.

## Root cause — a defective sentence model built on macOS

**Correction (supersedes this report's first-draft "memory-latency"
hypothesis).** The real cause — from GPT's M57 analysis, re-verified here against
the raw counters — is that on macOS Yune constructs a *structurally different,
defective* Luna sentence model from the compiled table, so it does far more (and
wrong) work. It is not a hardware/latency effect. Same config (owned poet), macOS
vs the checked-in Windows `run-c-owned-default`, from `memory-owner-profile.csv`:

| poet owner (item_count) | Windows owned | macOS owned |
| --- | ---: | ---: |
| `entries_by_code` | `513,353` | `191,984` |
| `lookup_index` | `332,604` | `31,262` |
| `vocabulary` | `421,966` | `421,966` |
| `abbreviation_vocabulary` | **`11`** | **`421,966`** |

macOS builds a third of the entries, a tenth of the M40 lookup index, and the
full `421,966`-row essay as the abbreviation vocabulary instead of the curated
11-row M42 set. The `m37_metrics` counters follow (Windows → macOS): 37-char
`vocabulary_entries_considered` `168 → 9,741`; abbreviation-row
`abbreviation_span_discovery_calls` `9 → 0` (the abbreviation path is never
reached). Both platforms take the *same* code path (`checksum_status =
accepted_upstream_marisa_import_checksum`, `source_fallback = false`,
`selected_storage = rsmarisa_byte_backed`); the only difference is the table
**bytes** (same `stored_entries = 498,564`, different `table_checksum` —
`rime_deployer` serialized a byte-different MARISA of the same content). With
`YUNE_POET_BYTE_BACKED=1` the model is **identically** broken, so it is the
shared model-construction-from-table path, not the owned-vs-byte-backed poet
choice. M57 later traced the construction mismatch to target-scoped checksum
recognition: Yune accepted the Windows upstream Luna MARISA checksum pair but
not the macOS pair. The completed repair and evidence are under
[`M57`](../../../plans/completed/m57-plan-macos-track-a-sentence-model-parity.md)
and
[`m57-macos-track-a-sentence-model-parity/`](../m57-macos-track-a-sentence-model-parity/).

The profile below explains *where* the resulting extra work is spent — it is the
symptom, not the cause. `sample` on a symboled release dylib (opt-level=3, LTO,
unstripped) over the 37-char row, leaf attribution (~8k main-thread samples):

- `UpstreamSentenceModel::vocabulary_chars_match_input_prefix_from_owned` — #1
  leaf (~45%), + `_platform_memcmp` it calls (~41%) + SipHash HashMap probes
  (`hash_one`/`Hasher::write`, ~13%) ≈ **~91%** of samples.
- `malloc`-family leaves ≈ **2%** — the allocator is **not** the bottleneck.

The hot function ([`poet/mod.rs:2840`](../../../../crates/yune-core/src/poet/mod.rs))
is a recursive validator: for each candidate vocabulary word it walks the word's
characters, does a `HashMap` lookup per char and a `remaining.starts_with(code)`
byte-compare (→ `memcmp`), recursing. It is invoked across every span of the
sentence lattice over a **~160 MB owned vocabulary** (`poet.vocabulary` 82 MB +
`poet.abbreviation_vocabulary` 78 MB, from `memory-owner-profile.csv`).

Scaling (length sweep on prefixes of the 37-char sentence): per-key median grows
**linearly** with length (~270 µs per char-position on macOS vs ~16 µs on
Windows), i.e. **O(N²) per full sentence**. Committing a 37-char sentence costs
**~362 ms** on macOS (`m37_metrics` reports `process_key_ns ≈ 384 ms`,
`vocabulary_entries_considered ≈ 9,741`). The same O(N²) scaling law is present
in the Windows numbers; the macOS per-key cost is inflated because the defective
model considers ~58x more vocabulary (above), not because of a hardware constant.

**Ruled out:** allocator (2% leaf), the byte-backed dictionary table (comparable
on both platforms), thermal throttling (tight p95/p99, librime *faster*, short
rows *faster*), and — after the correction above — any hardware/memory-latency
explanation.

**Likely owner and decisive test (for M57):** `TableStorage` delegates
enumeration in `crates/yune-core/src/translator/mod.rs`, while the compact
MARISA traversal lives in
`crates/yune-core/src/dictionary/compiled_table.rs`. The clean isolation
experiment is to run one Yune binary over both platforms'
`luna_pinyin.table.bin` payloads and diff the distinct `all_codes()` count,
the expanded `table_entry_iter()` count against `stored_entries = 498,564`,
the sentence-model owner counts, and the four candidate snapshots.

## Candidate correctness — a real divergence, not an artifact

The reports disclose that Yune matches librime's first candidate page on **both**
abbreviation rows. On macOS this is **false** (reproduced in all 3 runs;
`comment=[]` for both engines, so not a comment artifact; `page_no=0, idx=0`, so
not paging):

- `cszysmsrsd`: librime top `重商主義什麼是認識到`; Yune top `重商主義什麼少女時代`
  (1 shared candidate; Yune's top sits at librime index 2).
- `zybfshmsru`: librime top `自有辦法什麼收入`; Yune top `專業並不是美少女`
  (0 shared — fully disjoint; Yune emits a fixed `並不是美少女` tail across the page).

Yune's abbreviation candidates are not only different but visibly worse
(nonsensical completions), and this shares the root cause with the latency: the
owned abbreviation path explores a large, low-quality sentence space. `ni`/`hao`
text-match librime (comment-only diff), and `n`/`zhongguo`/both long-sentence
tops differ — consistent with the report.

## Bottom line

- The reports' **directional** story reproduces for ~7 of 10 latency dimensions
  and for memory. Yune is slower on sentences, faster on `zhongguo` and
  startup/session.
- The reports' **magnitudes do not hold** on macOS. The long-sentence rows are
  ~18x slower same-config (and ~30-60x by the reported ratio once librime's own
  macOS speedup is included), and **two claimed "win" rows reverse to 5-7x
  losses**.
- **This magnitude gap is a Yune bug, not a platform trait.** macOS builds a
  defective Luna sentence model (a third of the entries, the full `421,966`-row
  abbreviation vocabulary instead of 11); the same-config translator does ~58x
  more work. It is a real portability defect (M57), so the macOS latency
  ratios are diagnostic evidence, not yet a fair contradiction of the Windows
  M55 numbers.
- One correctness disclosure (abbreviation candidate parity) is **falsified** on
  macOS.
- The startup/session "faster than librime" rows the reports already flag as
  run-noisy hold directionally here (and are noisier still — run-3 startup p95 is
  a ~0.68 s tail on 9 samples).

## Caveats

- **Absolute** timings and memory are platform-specific and not comparable across
  Windows/macOS; the verification is about whether the reported **ratios and
  directions** reproduce.
- The **byte-backed opt-in** (`YUNE_POET_BYTE_BACKED=1`) *was* measured on macOS
  in this review: the model is identically broken (`191,984` entries, `421,966`
  abbreviation vocab, same wrong candidates, same ~10 ms/key), so it is not a
  workaround — the defect is in the shared model-construction-from-table path.
- Browser (WASM/Playwright) rows were not re-run; they remain carried evidence.
- The existing WEB-03 TypeDuck byte-backed long-input guard
  (`web03_byte_backed_jyutping_long_input_avoids_candidate_expansion_explosion`)
  could not be used on this checkout without the gitignored local web asset
  directories. Whether the same macOS compact-table issue affects that
  TypeDuck/web lane remains unverified, not disproven.

## Note on the automated synthesis

An adversarial synthesis pass framed the owned path as an accidental slow path
and cited a byte-backed "0.227x, Yune faster" Windows number. That number is a
**pre-corrective, batch-shaped** measurement that the current dashboard
explicitly disavows; and per `schema_install.rs` + the dashboard, **owned is the
shipping default** (byte-backed is the slower opt-in). The corrected reading is
above: owned-vs-owned is the right same-config comparison, and it is ~18x.

## Commands / evidence

- Runs: `run-1/`, `run-2/`, `independent-run-3/` (each with `summary-comparison.csv`,
  `raw_lookup_microbench.csv`, `candidate_snapshots.csv`, `memory-owner-profile.csv`).
- Windows same-config baseline: `../m55-native-match-or-beat/corrective-2026-07-04/run-c-owned-default/`.
- Re-run: `bash scripts/benchmark-native-rime-inprocess-macos.sh --output-root <dir>`
  (with `~/.cargo/bin` on PATH).
