# M55 Phase 3R-0 Fixture Expansion Evidence

Date: 2026-07-04

Verdict: complete with named pre-existing blockers.

## Capture

Command:

```powershell
powershell -ExecutionPolicy Bypass -File scripts\capture-upstream-luna-pinyin.ps1 -OracleRoot target\upstream-oracle\1.17.0 -Output crates\yune-core\tests\fixtures\upstream-1.17.0\luna-pinyin-basic.json -SentenceExpandedOutput crates\yune-core\tests\fixtures\upstream-1.17.0\luna-pinyin-sentence-expanded.json
```

Oracle: `rime/librime 1.17.0`
commit `33e78140250125871856cdc5b42ddc6a5fcd3cd4`.

Output fixture:
`crates/yune-core/tests/fixtures/upstream-1.17.0/luna-pinyin-sentence-expanded.json`.

Fixture shape:

- `11` upstream snapshots.
- `9,644` source dictionary rows for tested syllable/full-input codes.
- `4,999` essay rows for in-scope source/candidate terms.
- Rows include 3-5 syllable phrases, mixed-length sentences, `shijian`,
  `beijing`, and both M55 benchmark inputs:
  `ceshiyixiachangjushuruxingnengzenyang` and
  `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong`.

The default scenario set in `scripts/capture-upstream-luna-pinyin.ps1` writes
the Phase 3R rows into the oracle root's
`target/upstream-oracle/1.17.0/luna-pinyin-scenarios.json` before capture.

## Verification

Focused green rows:

```powershell
cargo test -p yune-core --test upstream_luna_pinyin_parity expanded_sentence
cargo test -p yune-core --test oracle_fixture_provenance upstream_luna_pinyin_fixtures_have_non_circular_source_provenance
```

Result:

- `expanded_sentence_fixture_covers_phase3r_rows` passed.
- `expanded_sentence_green_rows_match_upstream_before_graph_work` passed for
  `sentence_completion_shijian`.
- `upstream_luna_pinyin_fixtures_have_non_circular_source_provenance` passed.

## Named Blockers

The following oracle-captured rows are intentionally recorded as ignored tests
with panic bodies because current Yune does not match them before Phase 3R graph
work:

- `sentence_phrase_zhongguoren`
- `sentence_phrase_beijingshi`
- `sentence_phrase_rengongzhineng`
- `sentence_phrase_bianchengyuyan`
- `sentence_phrase_ceshiyixia`
- `sentence_mixed_woxiangqubeijing`
- `sentence_mixed_jintiantianqihenhao`
- `sentence_completion_beijing`
- `sentence_benchmark_37`
- `sentence_benchmark_59`

Diagnostic mismatch summary from the pre-ignore test run:

- `sentence_phrase_zhongguoren`: Yune keeps five `zhong guo ren`
  combinations after the top row; librime backs off to shorter `zhong guo`
  candidates.
- `sentence_phrase_beijingshi`: Yune keeps five three-syllable combinations;
  librime backs off to `beijing` candidates after the top rows.
- `sentence_phrase_rengongzhineng`: Yune keeps five four-syllable combinations;
  librime backs off to `rengong` candidates after the top row.
- `sentence_phrase_bianchengyuyan`: Yune keeps four-syllable combinations;
  librime backs off to `biancheng` candidates after the top row.
- `sentence_phrase_ceshiyixia`: Yune keeps four-syllable combinations; librime
  backs off to `ceshi` candidates after the top row.
- `sentence_mixed_woxiangqubeijing`: Yune keeps longer combinations; librime
  backs off to `woxiangqu` / `woxiang` candidates after the top row.
- `sentence_mixed_jintiantianqihenhao`: Yune keeps full-length combinations;
  librime backs off to `jintiantianqi` / `jintian` candidates after the top row.
- `sentence_completion_beijing`: fifth candidate differs (`被經` vs `背淨`).
- `sentence_benchmark_37`: Yune misses the librime top sentence
  `測試一下長句輸入性能怎樣` and produces `測試一下長據書如行能怎樣`.
- `sentence_benchmark_59`: Yune misses the librime top sentence
  `這個引擎其實應該支持超長句子輸入才能用` and produces a different
  long segmented sentence.

These blockers are the intended parity net before Phase 3R-1/2 graph surgery;
they are not derived from Yune expected values.
