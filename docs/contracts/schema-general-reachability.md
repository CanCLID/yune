# Schema-General Single-Character Reachability Contract

Status: Normative from M60. M59 owns the shipped behavior; M60 formalizes its
governance, audit, and onboarding boundary without changing runtime behavior.

## Capability invariant

Every deployed dictionary-translator prescription inherits
`leading_syllable_reachability = true` unless its effective, fully compiled
namespaced setting is explicitly `false`. The namespace comes from the deployed
`engine/translators` prescription: the default is `translator`, while a
prescription such as `script_translator@foo` reads
`foo/leading_syllable_reachability`. A schema does not need, and must not add, a
per-schema `true` flag to receive the default.

The setting applies to the dictionary-translator arms installed by production
schema deployment: `table_translator`, `script_translator`, and
`r10n_translator`. Other translator components do not silently acquire this
contract. The upstream-table lazy bounded arm is explicitly covered as a
leading-single path, but it rejects `prefix_fallback` before entry; it therefore
cannot establish or override TypeDuck prefix ownership. The ordinary bounded,
complete, and byte-backed paths use the request-local ownership rule below.

An effective explicit `false` disables only the supplemental leading-syllable
family for that translator namespace. It does not disable exact dictionary
translation, another translator, TypeDuck `prefix_fallback`, selection, or
recomposition behavior those independent paths already own. Default-on
reachability makes a leading character available for arbitrary non-lexicon
composition; it is not a promise that the character ranks first or appears on a
particular page. Existing selection and partial-recomposition rules continue to
decide the consumed span and residual input.

## Per-input TypeDuck precedence

`prefix_fallback` is TypeDuck-profile compatibility machinery. It is not
inherited by a new schema and is not the schema-general default.

When both mechanisms are enabled, precedence is per input:

1. a deployed proper prefix found for the current request sets
   `prefix_fallback_owned`, and prefix fallback owns that request's reachability;
2. when no deployed proper prefix exists, prefix fallback does not own the
   request and the independent leading-syllable path remains available.

Enabling `prefix_fallback` at schema level therefore never suppresses
leading-syllable reachability for every input. This is the runtime rule already
shipped by M59, not new M60 behavior.

## Explicit-false governance

There is no current opt-out. The live registry at
[`apps/yune-web/schema-acceptance-coverage.json`](../../apps/yune-web/schema-acceptance-coverage.json)
must contain exactly `reachabilityOptOuts: []` until a separate owner decision
approves one.

A future opt-out is valid only when the registry has exactly one complete row
for each affected (`settingAsset`, `configPath`, `schemaAsset`) tuple emitted by
the production-semantic Rust audit, and every row maps back to exactly one such
effective false tuple. Shared settings require one row for every affected
installable schema asset. The row must retain the exact M60 logical shape:
stable unique id; exact schema, setting, and namespaced config paths; schema id;
pinned source repository and 40-hex commit; named owner and specific reason;
nonempty affected surfaces; tracked oracle or owner-spec evidence with a 40-hex
Yune source commit; accepted real-path id; named decision/approver/date; and
nonempty revisit triggers with an unexpired review date.

An opt-out cannot be inferred from a temporary test patch, generated deploy
output, dependency-only row, missing evidence, unsupported/N/A classification,
or absent setting. Approval narrows D-47 and requires a separate owner decision.

## Anti-gaming and fail-closed rules

The following are prohibited:

- schema-id branches, input allowlists, or promotion tables;
- baked or replayed oracle candidates and circular fixtures;
- per-schema `true` flags used as onboarding work;
- silently treating an open schema as unsupported or N/A;
- unregistered or overlapping schema roots;
- a JavaScript reimplementation of RIME include/patch/custom merge semantics;
- an opt-out without the exact explicit-false/registry bijection.

The checker discovers Git-tracked `*.schema.yaml` paths, assigns each to exactly
one registered `product`, `test-fixture`, or `historical-evidence` root, and
requires the Rust audit for production schemas. Rust owns compiled include,
patch, custom-overlay, nested/slash-key, translator-prescription, boolean, source
trace, and asset-hash semantics. JavaScript owns registered-root, safe-path,
current-file, acceptance, expiry, and opt-out-bijection policy. Unresolved
directives in the relevant translator dependency closure fail the audit.

## Schema onboarding

For a repository-owned product schema:

1. register any new product root with the checker before adding its first
   schema; existing test fixtures and historical evidence stay explicitly
   non-product;
2. add the schema asset and regenerate the schema asset manifest;
3. run the production updater, which creates a blocking `status: open` /
   `disposition: unclassified` row and never creates or suggests an opt-out;
4. classify the asset as shipped/selectable, runtime alias or mirror,
   dependency-only, or mandatory nonshipped validation without collapsing those
   categories;
5. attach D-24/D-31-correct oracle provenance, or an explicit owner-spec where
   the oracle does not exhibit the owner-required reachability;
6. add a narrow real deployment-path default-on and explicit-false acceptance
   test; and
7. mark the row accepted and run the mandatory schema-manifest checker.

An open row blocks acceptance. `unsupported`, N/A, a dependency-only
disposition, or an automatically synthesized opt-out cannot close a new product
schema. A future product root remains ungoverned until it is registered and is a
hard checker failure, not implicit coverage.

## Source-bound authority

| Claim | Authority retained after M60 |
| --- | --- |
| Final native reachability behavior and signed performance guard | M59 source `443cc636` |
| Shipped registry, WASM, app, browser, and package closeout | M59 source `5fa986d8` |
| Bidirectional 60-asset manifest/tree exactness | M59 follow-up `07845e02` |
| WEB03-11 measured product behavior | measured product source `ef485b10` |
| Capability formalism, static audit, and onboarding governance | M60 closeout tree; no new behavior or performance claim |

M60 does not recapture an oracle, rerun a benchmark, rewrite M59 or WEB03
evidence, change candidate ordering, or project one evidence lane onto another.
The oracle pins and D-24/D-31/D-48 lane boundaries remain unchanged.

## Scope boundary

This contract adds no runtime behavior, candidate-order or ranking change,
selection or recomposition change, schema-install change, C ABI/API table/export,
schema or profile id, browser or Windows product surface, oracle fixture, signed
threshold, benchmark, memory optimization, or M61 implementation.
