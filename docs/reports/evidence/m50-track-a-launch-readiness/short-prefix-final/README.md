# M50 Short-Prefix Final

Scope: native Track A `luna_pinyin` only. This run is not evidence for browser,
frontend, product package, deployment, public demo, TypeDuck keyboard-profile
memory, or iOS-device claims.

## Command

`powershell -ExecutionPolicy Bypass -File scripts\benchmark-native-rime-inprocess.ps1 -OutputRoot docs\reports\evidence\m50-track-a-launch-readiness\short-prefix-final -Iterations 9 -SessionIterations 60 -KeyIterations 80 -TrackAInputs n,ni,hao,zhongguo,ceshiyixiachangjushuruxingnengzenyang,zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong,cszysmsrsd,zybfshmsru -TrackBInputs neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung -DeployProductBeforeBenchmark`

## Result

The bounded short-key path keeps M44's first-page surplus and underfill fallback
behavior. It now avoids raw-comment cloning during bounded lookup-view
materialization when the visible comment is empty or can be derived from the
entry-code suffix, and it skips empty dictionary-exclude and spelling-abbreviation
set checks for Track A.

| Input | Yune median | librime median | Ratio | Verdict |
| --- | ---: | ---: | ---: | --- |
| `n` | `60.000 us` | `21.600 us` | `2.778x` | pass |
| `ni` | `44.500 us` | `14.600 us` | `3.048x` | measured blocker |

`ni` remains above the `<=3.0x` same-run gate. The implementation does not add
a retained heap prefix/vocabulary index.

## Blocker Attribution

M37 medians for `ni` in this run:

- `process_key_ns`: `86,750`
- `translator_ns`: `82,200`
- `lookup_views_visited`: `196`
- `exact_lookup_candidates`: `7`
- `prefix_lookup_candidates`: `7`
- `short_key_candidate_rows_scanned`: `14`
- `short_key_candidates_materialized`: `14`
- `short_key_filter_ns`: `26,550`
- `short_key_first_page_materialize_ns`: `5,200`

The remaining measured blocker is the exact-row scan needed to find enough
charset-allowed `ni` candidates without a retained prefix/exact acceptance
index. Reintroducing a retained vocabulary/prefix index was intentionally
avoided because the rejected M49 index added about `35 MB`.

## Focused Checks

- `cargo test -p yune-core bounded_static_table_request_matches_eager_top_candidates`
- `cargo test -p yune-core short_luna_key_refresh_uses_first_page_bound_and_completes_on_page_turn`
- `cargo test -p yune-core short_luna_key_refresh_falls_back_when_filter_surplus_underfills_first_page`
- `cargo test -p yune-core bounded_compact_translator_uses_prism_abbreviation_spans_for_sentence_model`
