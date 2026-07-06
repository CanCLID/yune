# M58 Phase 3 Schema/Profile Identity Blast-Radius Audit

Status: audit-only. No schema id split, product rename, dictionary rename,
manifest change, cache-key migration, or userdb migration is implemented here.

Preferred direction remains: canonical `rime-cantonese` owns plain
`jyut6ping3`, and TypeDuck multilingual/profile behavior moves to a future
`jyut6ping3_typeduck` profile only after explicit user sign-off. This audit
records the current blast radius for that decision.

## Inventory

| Surface | Current role | Split risk | Disposition |
| --- | --- | --- | --- |
| `apps/yune-web/public/schema/jyut6ping3.schema.yaml` | Browser-visible `jyut6ping3` schema using TypeDuck-tuned schema assets today. | Plain `jyut6ping3` currently conflates canonical and TypeDuck-derived behavior. | Sign-off required before changing source or semantics. |
| `apps/yune-web/public/schema/jyut6ping3_mobile.schema.yaml` | Shipped/mobile runtime profile id; dictionary remains `jyut6ping3`. | Product path, launch assets, memory probes, and Track B evidence use this id. | Defer rename unless a migration plan and gates are approved. |
| `jyut6ping3_typeduck` | Preferred future TypeDuck id in docs only. | No production schema asset, TypeScript union member, or manifest entry exists today. | Proposal only; not executable without signed code/assets changes. |
| `crates/yune-rime-api/src/schema_install.rs::is_typeduck_jyut6ping3_profile` | Enables TypeDuck heuristics when dictionary is `jyut6ping3` and schema is `jyut6ping3` or `jyut6ping3_*`. | A canonical plain `jyut6ping3` lane would still match TypeDuck heuristics unless predicates are redesigned. | Requires focused predicate tests before any split. |
| `crates/yune-rime-api/src/schema_install.rs::is_yune_web_launch_byte_backed_profile` | Special byte-backed launch path for `jyut6ping3_mobile` and `cangjie5`. | A renamed TypeDuck runtime id would miss the special case. | Requires launch-path tests before any rename. |
| `apps/yune-web/src/worker.ts` schema mapping | UI `jyut6ping3` maps to runtime `jyut6ping3_mobile`, dictionary `jyut6ping3`. | Visible id, runtime id, and dictionary id are aliased. | Requires browser evidence and migration plan. |
| `apps/yune-web/src/types.ts` / `isRimeSchemaId` | TypeScript schema allowlist. | `jyut6ping3_typeduck` is currently rejected. | Add only after sign-off and typecheck/browser gates. |
| `apps/yune-web/public-demo/schema-asset-manifest.json` | WEB-03 public-demo schema assets include `jyut6ping3` and `jyut6ping3_mobile`. | Rename/addition changes public payload contract and hashes. | Requires WEB-03/public-demo guard. |
| `docs/reports/evidence/m55-native-match-or-beat/thresholds/m55-thresholds.csv` | Track B rows use `jyut6ping3_mobile`. | Rename breaks ratchet comparability unless the lane is forked or migrated. | Requires standing M55 ratchet if product-path thresholds change. |
| `scripts/benchmark-native-rime-inprocess.ps1` and `crates/yune-rime-api/benches/native_inprocess_benchmark.rs` | Track B benchmark/status code maps dictionary `jyut6ping3` to prism `jyut6ping3_mobile`. | New id needs explicit benchmark/status mapping. | Requires benchmark gate if changed. |
| Userdb paths in `crates/yune-rime-api/src/userdb/`, `crates/yune-rime-api/src/session.rs`, and `apps/yune-web/src/worker.ts` | Persistence is dictionary-name keyed, commonly `${dictionaryId}.userdb`. | Keeping dictionary `jyut6ping3` shares learning between lanes; renaming dictionary can orphan or migrate user data. | Requires explicit migration/no-migration decision. |
| `crates/yune-rime-api/tests/typeduck_windows_boundary.rs` | TypeDuck-Windows profile smoke uses `schema_id: jyut6ping3`. | Split can break stock TypeDuck-Windows expectations. | Defer to signed TypeDuck/profile Windows plan. |
| `crates/yune-core/tests/fixtures/typeduck-v1.1.2/` | Grandfathered TypeDuck profile regression guards. | These are not canonical `rime-cantonese` evidence and should not be rewritten as canonical fixtures. | Preserve until a profile-lane decision supersedes them. |

## Required Gates Before Any Future Split

- Explicit user sign-off on the id direction.
- Focused predicate tests proving canonical and TypeDuck/profile lanes select
  intended behavior.
- TypeScript typecheck and browser evidence if `apps/yune-web` schema lists,
  worker mapping, or public assets change.
- WEB-03/public-demo guard if public-demo manifests or assets change.
- Standing M55 Track B ratchet if product-path performance thresholds or names
  change.
- Userdb migration/no-migration evidence before dictionary-name changes.
