# Lane A re-validation — 2026-07-08 (durable baseline before the diagnosis increment)

Increment result: **requirement 1 (shim/landmine disposition) RESOLVED**;
**requirements 2/3 (Yune canonical runner + classified diff) BLOCKED** on a Yune
rime-cantonese deploy failure. Owner-verified (landmine correct, blocker
reproduces, one diagnosis branch eliminated). Recorded now because these findings
otherwise live only in chat — the pre-revert Yune-runner artifacts died precisely
because findings weren't committed while fresh.

## 1. Landmine disposition — DISARMED (requirement 1)

The M58 audit flagged that a canonical schema with `schema_id: jyut6ping3` + dict
`jyut6ping3` might silently install TypeDuck-profile shims. **The current typed-config
predicate (from `5d3dba2a`) closes this.** `is_typeduck_jyut6ping3_profile`
(`crates/yune-rime-api/src/schema_install.rs:860-864`) requires **both**
`dictionary == "jyut6ping3"` **and** `schema_behavior_profile_from_config == TypeduckJyutping`,
which comes only from a `yune/profile: typeduck_jyutping` key
(`schema_yune_profile`, `:867-871`). The staged upstream rime-cantonese schema
(`target/upstream-oracle/1.17.0/m58-rime-cantonese-shared/jyut6ping3.schema.yaml`)
carries **zero `yune` keys** and none of the shim configs → `SchemaBehaviorProfile::Standard`.

Resolved canonical lane configuration (pin before any diff row):

| setting | value | source |
|---|---|---|
| behavior profile | **Standard** | no `yune/profile` key (`:172-175`) |
| prefix_fallback | **false** | `unwrap_or(is_typeduck)` → false (`:330-339`) |
| prediction_candidate_limit | **none** | predicate false (`:329`) |
| dynamic_correction_lookup | **false** | `with_dynamic_correction_lookup(is_typeduck)` (`:457`) |
| sentence_word_penalty | **default** (no 21.0) | `if is_typeduck` false (`:505-506`) |
| leading_syllable_reachability | **ON** | M59 flip default (`:352`); librime exhibits it natively, so it belongs in the parity surface |
| storage | **heap / source** | `prefer_compact_storage` false for this lane (`:389-391`) |

No opt-out marker needed — the lane runs shim-free naturally. The pre-revert
`bei`→碑悲卑… numbers had unknown shim state and are **not** trusted; the re-run
must reproduce or correct them.

## 2. Capture integrity — VERIFIED (content), with a byte-reproducibility caveat

Regenerated `phase-1/canonical-rime-cantonese-capture.json` from the oracle via
`scripts/capture-upstream-rime-cantonese.ps1 -Inputs be,bei,bein,being,beingo,beix,beixngoxx,ngohaig,ngohaigo,n,nri,mgoi,zijiguk -ReportedCaseInput zijiguk -Output <tmp>`:
- **Content-identical**: 0 structural diffs across all 13 inputs (every candidate +
  provenance field equal; 77,280 lines each).
- **Provenance re-pinned**: the oracle `rime.dll` SHA-256 self-check inside the
  script passed (committed capture file SHA-256 `d924ec77586391f88cc19e915461bfc97d4cdd85ad6e7cbe78c175486f281bf1`).
- **Byte-diff is serializer cosmetics**: the only difference is JSON indentation
  depth (a `ConvertTo-Json` version difference), not data. **Follow-up caveat:** unlike
  the luna capture, this capture is **content-reproducible but not byte-reproducible**;
  content + provenance are what's verified.

## 3. Deploy blocker — Yune cannot load the staged rime-cantonese (blocks req. 2/3)

```
cargo run -p yune-cli -- frontend \
  --shared-data-dir target/upstream-oracle/1.17.0/m58-rime-cantonese-shared \
  --user-data-dir <tmp> --schema jyut6ping3 --sequence "bei"
# -> error: schema deployment failed. next: verify shared_data_dir contains deployable RIME schema files.
```
`deploy()` returns non-TRUE (`crates/yune-cli/src/rime_frontend.rs:188-191`); the
bool ABI swallows the real cause. **Eliminated causes:**
- NOT the multi-file `import_tables` — Yune supports it (`schema_install.rs:783`,
  `source.rs:748` `append_rime_import_table_entries`); the 5 component dicts
  (chars/words/phrase/lettered/maps) + `jyut6ping3.dict.yaml` are all present.
- NOT missing assets — all schema/dict/vocab files present, incl.
  `vocabulary: essay-cantonese` (`essay-cantonese.txt` staged) and the oracle's
  pre-built `build/jyut6ping3.table.bin`.
- **NOT artifact staleness** (owner-verified): a clean staging copy containing
  ONLY the YAML/txt sources fails **identically**, so it is not the M56
  fail-closed policy rejecting librime-format compiled artifacts. The failure is
  in Yune's **source-path deploy** of the rime-cantonese schema set (schema parse,
  multi-dict compile, or a referenced resource — check `vocabulary: essay-cantonese`
  early).

There is **no committed Yune canonical runner** — the pre-revert `yune-canonical-*`
artifacts the m59-canonical README cited were lost in the revert. So the runner
must be **re-created**, not re-run. **No classified diff can be computed until Yune
loads rime-cantonese.**

## 4. Next increment — diagnosis (fresh)

Surface the swallowed deploy `Result` via a small integration test/harness in
`tests/` that calls the schema-install/deploy path directly (the no-engine-code
rule binds the engine, not diagnostics). Check the `vocabulary: essay-cantonese`
load path early. If the root cause is a missing Yune capability that
rime-cantonese's schema uses, that becomes a **named Lane A work item with its own
oracle discipline** — the surprise that sequencing Lane A first was meant to surface.
