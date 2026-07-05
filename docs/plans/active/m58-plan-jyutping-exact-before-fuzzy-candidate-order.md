# M58 Jyutping Candidate Reachability & Over-Admission Gate Repair Plan

> **For agentic workers:** execute one phase at a time. This plan is written as
> a review packet: before implementing, have another reviewer challenge the
> root-cause chain, the fix, and the oracle gates. The named product target is
> the TypeDuck `jyut6ping3` profile against TypeDuck-HK/librime `v1.1.2`; that
> fork is the correctness oracle for every candidate claim here.

> **Status:** Ready for execution (v4 plus final review tightening, 2026-07-05).
> v4 refinements: the admission divergence is **bidirectional**
> and fixed by two orthogonal mechanisms — Leg A = the *gate* (over-admission);
> derived-form matching = a deferred *admission-depth* lane (under-admission,
> proven present in the flagship `ngohaig` case, not just `nri`). Assertions are
> **directional and per-input** (`beingo` full set+order; `ngohaig` directional),
> order parity is in the model, and capture provenance is recorded. - **Track:**
> Engine behavioral correctness (TypeDuck/Jyutping product lane). - **Created:**
> 2026-07-05. - **Type:** bug-fix milestone. No ABI widening, no new performance
> claim.

> **Amendment note (v3 — the model changed).** v1/v2 framed the bug as "Yune
> emits a same-initial fuzzy flood the oracle never shows; suppress it." An
> adversarial verification pass against the checked-in fixtures **falsifies that
> premise**. A *complete* capture already exists in-repo
> (`jyut6ping3-m21-closeout.json` case[4], input `ngohaigo`, `is_last_page:true`,
> 49 candidates): the oracle emits a **rich, ordered, multi-group** candidate
> list for a fully-parseable input — phrase group → `ngo`-syllable set
> interleaved with `o`-code algebra → the alt-parse word 午安 (`ng5on1`) → the
> full 19-char `ng`-syllable set — **not** "exact matches only." So "stop at the
> exact set" would *over*-suppress oracle-correct groups. The two real,
> provable divergences are:
>
> 1. **Reachability (confirmed core):** a per-fetch cap of 2 truncates each exact
>    toned code so 畀 (third `bei2`) is unreachable, though the oracle emits the
>    full exact set. Two further caps (64 output, 256 pending) can hide more.
> 2. **Admission (reframed):** for `ngohaig` Yune admits `n`+vowel abbreviation
>    codes (你 `nei5`, 能 `nang4`, 男 `naam4`, 女 `neoi5`, 內 `noi6`, 呢 `ne1`)
>    that the complete oracle set does **not** contain; and for `nri`/bare `n` it
>    *drops* vowel-via-`ng`-algebra rows (安 `on1`, 屋 `uk1`, 愛 `oi3` …) the
>    oracle *does* contain. Both stem from `original_code_allows_prefix_fallback`
>    matching the **raw** dictionary code against the single input letter.
>
> The reachability fix (Leg B) is solid and lands first. The admission fix
> (Leg A) is a **hypothesis with a decision matrix**, driven by the full
> captures, and may reduce to "suppress the specific out-of-oracle admissions"
> rather than any blanket flood removal.

**Goal:** Make Yune's TypeDuck/Jyutping candidate output match the
TypeDuck/librime `v1.1.2` oracle for the M58-owned multi-syllable composition
claims: `beingo` reachability/full-capture disposition, the non-syllable
single-letter over-admission gate, and the pinned non-regression rows. The
under-admission-depth lane remains named-deferred unless deliberately pulled
into scope.

- **Reachability (confirmed):** for a composition with a complete-syllable
  leading parse (`beingo`→`bei`), emit the **full** exact leading-parse set so
  less-common exact characters (畀 = third `bei2`) are reachable. The oracle
  emits full exact syllable sets (`ng`→19, `hou6`→13, `ngohaigo`'s ng-set→19);
  Yune's per-fetch cap of 2 truncates them. Fix bounded against **all three**
  fallback caps, on the compiled product path.
- **Admission parity (capture-gated):** bring Yune's *set membership* into line
  with the complete oracle captures — suppress the specific codes Yune admits
  that the oracle omits, without dropping the oracle's own multi-group output.
  The exact divergence is defined by diffing Yune's output against the complete
  captures (Phase 0/1), not by an "exact-only" assumption.

This is a comparability/correctness repair, not a reordering-of-fuzzy exercise,
and not a blanket flood removal.

## Problem Statement

Typing a multi-syllable word and picking characters one at a time is broken:
a valid exact-syllable character can be unreachable. Reproduced on this machine
(`jyut6ping3_mobile`, `yune-cli frontend`, page size 6), input `beingo` (畀我),
paging with `=`:

| Page | Candidates (text/code) |
| --- | --- |
| 0 | `俾我`(bei2ngo5) `比`(bei2) `被`(bei6) `備`(bei6) `俾`(bei2) `悲`(bei1) |
| 1 | `秘`(bei3) `臂`(bei3) `卑`(bei1) · `啤`(be1) `唄`(be6) `不`(bat1) |
| 2 | `本`(bun2) `表`(biu2) `部`(bou6) `報`(bou3) `巴`(baa1) `不過`(bat1gwo3) |
| 3 | `波`(bo1) `邊`(bin1) `保`(bou2) `班`(baan1) `變`(bin3) `別`(bit6) |

The wanted 畀 (`bei2`) never appears. Each toned code emits **exactly two**
characters then stops (`bei2`→比,俾; `bei6`→被,備; `bei1`→悲,卑; `bei3`→秘,臂):
畀 is the *third* `bei2` character, dropped by the per-fetch cap. (For bare `bei`,
畀 *does* appear — the full input has an exact multi-tone lookup that flows
through the primary exact/sentence path, not the capped prefix-fallback loop;
Phase 1 confirms this path difference.) So the user must type `bei` alone, commit
畀, then type `ngo` separately.

Second reported case: `諮議局` (typed syllable-by-syllable; not a lexicon word)
— 諮 (`zi1`) is unreachable the same way, plus `enable_completion` injects longer
`zi`-prefixed syllables. The exact ASCII keystrokes must be pinned in Phase 0
(they are not yet stated).

The same-initial characters that appear *around* the exact set are **not**
straightforwardly "the bug" — Oracle Evidence shows the oracle itself emits a
rich same-initial/multi-group list. What is provably wrong is (a) the cap hides
畀 and (b) Yune's admitted *set* differs from the oracle's.

## Oracle Evidence (the real target behavior)

The correctness oracle is TypeDuck-HK/librime `v1.1.2` on `jyut6ping3` (engine
`74cb52b`, schema `1bed1ae`). Verified against the checked-in captures under
`crates/yune-core/tests/fixtures/typeduck-v1.1.2/`:

### A complete capture proves the oracle is multi-group, not exact-only

`jyut6ping3-m21-closeout.json` **case[4]** — input `ngohaigo`, `preedit
"ngo hai go"`, `page_size:50`, **`is_last_page:true`, 49 candidates**. Full
structure, in stable order:

| Ranks | Group | Sample |
| --- | --- | --- |
| 0–2 | phrase | 我係個 `ngo5hai6go3`, 我係 `ngo5hai6`, 我喺 `ngo5hai2` |
| 3–28 | `ngo`-syllable set **interleaved with `o`-code algebra** | 我 `ngo5`, 俄 `ngo4`, **柯 `o1`**, 餓 `ngo6`, **哦 `o4`**, **阿 `o1`** … |
| 29 | alt `ng`+`on` parse word | 午安 `ng5on1` |
| 30–48 | `ng`-syllable set (19 chars) | 五 `ng5`, 午 `ng5`, 誤 `ng6`, 吳 `ng4` … |

So a **fully parseable** input yields four+ groups + `o`-code algebra + an
alternative-parse word — the oracle does **not** "stop at the exact leading
parse." Crucially, this complete set contains **zero** `n`+vowel codes
(`nei`/`nang`/`naam`/`neoi`/`noi`/`ne`); the oracle's same-initial group is
confined to `ng`-syllables. (This capture supersedes the page-0-only
`jyut6ping3-windows-boundary-ngohaig.json`, which shows only the first 4 and is
where v1/v2's "exact only" claim came from.)

### The oracle emits full exact syllable sets (never capped at 2)

`ng`→19 exact `ng`-chars (`is_last_page:true`); `hou`→43–46 exact `hou*`/`ho*`;
the `ngohaigo` ng-set→19. Direct evidence the per-fetch cap of 2 is an
under-count (Reachability fix).

### The oracle floods for an unparseable head — with algebra rows Yune drops

`jyut6ping3-m14-completion-correction.json` — `nri` correction-off
(`correction_default`) and bare `n` (`completion_default`) are the **same**
50-candidate flood. Ranks 0–7 are `n`/`ng`-initial (我 `ngo5`, 你 `nei5`, 外
`ngoi6`, 能 `nang4`, 內 `noi6`, 呢 `ne1`, 男 `naam4`, 女 `neoi5`); ranks **8+
include vowel codes admitted via the `ng`-insertion spelling algebra** — 安
`on1`@8, 屋 `uk1`@9, 愛 `oi3`@10, 呀 `aa4`@13, 亞 `aa3`@16, 歐 `au1`@19 (14 vowel
rows total). An empirical Yune run on `nri` **drops all 14** of these vowel rows
(they fail `starts_with("n")` on the raw code) and diverges from the oracle
starting at rank 8. The `m21_nri` golden is green only because it asserts
`.take(6)` on the Owned/Heap path (`cantonese_parity.rs:1408-1440`); full-depth
`nri` parity is **not** pinned today.

### The genuine `ngohaig` divergence (from the plan's own repro)

Yune (current) emits 我係個/我係/我喺/我, then a same-initial group that **appends
你 `nei5`, 能 `nang4`, 男 `naam4`, 女 `neoi5`, 內 `noi6`, 呢 `ne1`** — codes that
are **absent** from the complete oracle set above. So the divergence is
**admission** (Yune adds out-of-oracle `n`+vowel codes; the oracle's group is
`ng`-only), not "the oracle shows no flood."

## Diagnostic Evidence (root cause)

The candidate list for a bounded lookup is assembled in
[`translator/mod.rs`](../../../crates/yune-core/src/translator/mod.rs):
sentence/exact candidates first, then prefix-fallback candidates appended
(`bounded_candidates_for_lookup_codes` → `prefix_fallback_candidates`).
`jyut6ping3_mobile` sets `prefix_fallback: true`, `enable_completion: true`,
`prediction_never_first: true` (schema YAML line 53 sets `prefix_fallback: true`
**explicitly** — flipping that key off is **not** the fix; it would break
letter-to-tone and other pinned behavior).

### Defect B — reachability: three caps, only the first is per-fetch

Three constants, all gated to the compiled Compact+prism product path
(`bound_expansion = bounds_compact_fallback_expansion()` =
`matches!(self.storage, TableStorage::Compact(_)) && self.prism_payload.is_some()`,
`mod.rs:1029-1031`; otherwise all three are `usize::MAX`):

- `MAX_PREFIX_FALLBACK_CANDIDATES_PER_FETCH_CODE = 2` (`mod.rs:36`), break at
  `mod.rs:2274-2277` — each toned code emits ≤2. **This hides 畀.**
- `MAX_PREFIX_FALLBACK_PENDING_CANDIDATES = 256` (`mod.rs:35`), mid-iteration
  breaks at `mod.rs:2278-2280` (inner) and `mod.rs:2284-2286` (**outer** — once
  256 pending are collected it abandons all remaining `prefix_spec`s in the
  fixed longest-first order, so which fetch codes get dropped is order-dependent).
- `MAX_PREFIX_FALLBACK_CANDIDATES = 64` (`mod.rs:34`), post-sort output break at
  `mod.rs:2317-2320`.

The candidates sort `consumed_input_len`-**descending** (`mod.rs:2288`), so exact
`bei*` already sorts ahead of shorter-prefix expansions — 畀 is dropped by the
cap, **not** positionally buried. **Path caveat:** on Owned `StaticTableTranslator`
(most unit tests) all caps are `usize::MAX` and the bug does **not** reproduce —
Defect B tests must run on the compiled path.

**Track B load-bearing.** Checked-in M57 metrics
(`.../track-b-yune-product/m37_metrics.csv`) show the ratchet input fires
prefix-fallback on 58 of 61 keypresses and produces exactly 3712 candidates =
58 × 64 — i.e. it **saturates the 64 output cap on every firing call**. So
raising caps directly loads the Track B latency gate; sizing must respect all
three caps and the `web03` tripwire (below).

### Defect A — admission: the raw-code single-letter match

`original_code_allows_prefix_fallback(raw_code, lookup)` (`mod.rs:2881-2888`) is
`normalized == lookup || (lookup.len() == 1 && normalized.starts_with(&lookup))`,
where `normalized` comes from the candidate's **raw** dictionary code (field 3 of
the rich comment, via `typeduck_rich_comment_code`, `mod.rs:2911-2926`) — the
*original* spelling (`on1`→`on`), not the `ng`-inserted derived form. Consequences
(both verified):

- **Over-admission** (`ngohaig`): the single-letter `n` prefix admits any code
  whose raw form starts with `n` — including `nei`/`nang`/`naam`, which the
  complete oracle set omits.
- **Under-admission** (`nri`): the oracle reaches 安 `on1`, 屋 `uk1` via the
  `ng`-insertion algebra (`^(?=[aeiou])/ng/`), but Yune matches the raw `on`/`uk`
  against `n` and rejects them. So Yune both adds codes the oracle omits and drops
  codes the oracle keeps.

The single-letter `starts_with` branch is therefore neither "correct as-is" nor
"delete it" — it is **mis-scoped** (raw code vs derived form) in a way that
diverges in both directions.

### No single "leading parse" referent exists

There is no segmentor syllable-span API on this path (segment tags are
whole-input type labels; `SegmentDebug` spans `0..input.len()`), and
`upstream_sentence_model` is installed **only** for luna
(`schema_install.rs:462-486`), not jyut6ping3. `valid_lookup_prefixes`
(`mod.rs:2328`) returns a **Vec of many** prefixes (every char boundary ×
exact + prism aliases): for `ngohaig` **both** `ngo` and `ng` survive. So "the
leading parse" is not a pre-existing singular value. Any rule that needs one must
**define** it (e.g. `prefixes[0]`, longest-first) over that Vec — and, per the
`ngohaigo` capture, the oracle keeps **multiple** parse groups anyway, so the fix
must not assume a single parse.

Relevant symbols: `translator/mod.rs` — `bounded_candidates_for_lookup_codes`,
`prefix_fallback_candidates`, `valid_lookup_prefixes` (`:2328`),
`sentence_lookup_specs` (`:1164`), `original_code_allows_prefix_fallback`
(`:2881`), `typeduck_rich_comment_code` (`:2911`),
`bounds_compact_fallback_expansion` (`:1029`), the three caps (`:34-36`), and the
`consumed_input_len`-descending sort (`:2288`). Spelling algebra:
`spelling_algebra.rs` (`ng`-insertion; `leading_syllable_abbreviations` `:462`).

## Proposed Fix

`prefix_fallback` is Yune-owned (explicit in `jyut6ping3_mobile.schema.yaml:53`,
defaulted-on via `schema_install.rs` `.unwrap_or(is_typeduck_jyut6ping3_profile)`
when the key is absent; the upstream TypeDuck schema at `1bed1ae` has no such
key). So the fix constrains a Yune compatibility shim to the oracle — never
derived from Yune's own output. Two legs of differing confidence; **Leg B lands
first and closes the user-facing bug on its own.**

### Leg B (unconditional, first) — make the exact leading-parse set reachable

Raise the per-fetch cap for the **leading-parse fetch codes** — the fetch codes
of *every* complete-syllable prefix spec (plural: the multi-parse model means
`ngohaig` has both `ngo` and `ng`), each scoped to its own tone — so the full
exact set (incl 畀) emits, matching the oracle. Requirements:

- **Bounded, not `usize::MAX`.** Fixtures bound the exact set at ~13 per single
  toned code (`hou6`; `ng6`=9) and ~26 across multi-syllable prediction (`san1`).
  Pick a bound (~16, up to ~30) the gates hold.
- **Size against all three caps.** Lifting only the per-fetch cap leaves the
  256 pending break and the 64 output cap — a large family (`zi`) can still be
  truncated. Confirm the target character survives all three; if the pending/
  output cap is the binding one, raise it too (bounded) and re-measure.
- **Tone-scope the raise.** A toned input (e.g. `beix` = `bei2`) must not flood
  the wrong-tone `bei1`/`bei3`/`bei6` families after the user disambiguated —
  bound the raise to the leading-parse code's **own tone**, not all tones of the
  shorter untoned prefix. (Phase 0 adds a toned/letter-to-tone capture.)
- **Compiled path only.** The bug and the caps live on Compact+prism; the
  regression test must run there (Owned storage does not trip the caps).
- **Re-prove the gates** (Win Bars): the Track B latency ratchet **and** the
  `web03` `prefix_fallback_views_visited ≤ 5000/6000` tripwire.

### Leg A (capture-gated) — suppress over-admission (the gate)

The admission divergence has **two directions**, and they are fixed by
**different, orthogonal** mechanisms — do not present them as alternatives for
one defect:

- **Over-admission** (Yune emits codes the oracle omits — `ngohaig`'s `你`/`能`/
  `男`/`女`/`內`/`呢`, all admitted via the single-letter `n` prefix). Fixed by
  **the gate**.
- **Under-admission** (Yune omits codes the oracle emits — see the
  admission-depth lane below). Fixed by **derived-form matching**. Deferred.

**Leg A = the gate.** Drop a prefix that is **not itself a complete syllable**
when a **longer complete-syllable prefix already contributes**. Detect
"complete-syllable prefix" via the lookup path — a prefix whose own spelling maps
to an exact syllable code (`valid_lookup_prefixes`/`sentence_lookup_specs`,
`mod.rs:2328`/`:1164`), as opposed to a bare letter that only expands to
same-initial completions. This is consistent with every checked-in capture:

| Input | Complete-syllable prefixes | Gate drops | Result vs oracle |
| --- | --- | --- | --- |
| `ngohaig` | `ngo`, `ng` (both survive) | single-letter `n` | `你`/`能`/`男` gone (oracle has none); `ngo`/`ng` groups kept ✓ |
| `beingo` | `bei`, `be` (capture-decided) | single-letter `b` | `b`-flood gone; `bei` set kept ✓; `be` rows (`啤`/`唄`) are complete-syllable shorter-prefix rows, so the full capture decides whether they stay, need a refined gate, or become a recorded disposition |
| `nri` / bare `n` | *none* (`n`/`nr`/`nri` are not syllables) | nothing (no longer complete-syllable prefix) | flood preserved → `m21_nri` golden safe ✓ |
| `mgoi` / bare `m` | `m` (`m` *is* a syllable) | nothing (`m` is the longest complete-syllable prefix) | `m`-abbreviation rows preserved ✓ |

The gate keys on **is-a-complete-syllable**, not **is-single-letter** — so `m`
(a syllable) is kept while `n`/`b` (not syllables) are dropped only when a longer
syllable contributes.

`beingo` therefore has one capture trap of its own: `be` is a complete syllable
and Yune currently emits `啤`/`唄` through that shorter prefix. If the full oracle
capture omits those rows (or includes an abbreviated-tail word that the current
legs do not own), win bar 2 cannot silently pass. The executor must either fix
that set-level divergence with a scoped refined gate or record an explicit
disposition tied to the capture; do not improvise a broad "drop complete
syllables" rule.

Constraints (verified pinned behaviors — do **not** regress):

- Keep shorter **complete-syllable** prefixes contributing:
  `static_table_translator_prefix_fallback_preserves_full_match`
  (`tests/filter.rs:641-657`) and m24 `jigaajiusihaa` requiring 而家 (`ji4gaa1`)
  at index 1 (`cantonese_parity.rs:1769`) both pin this. The gate only drops the
  non-syllable single-letter prefix, never a complete-syllable shorter prefix.
- Leading-parse/complete-syllable detection reuses the lookup path (`mod.rs:2328`/
  `:1164`) — **not** a segmentor span API (none exists) and **not**
  `complete_syllable_prefix_count` (a prediction-view classifier). The oracle
  keeps multiple parse groups, so this is *admission scoping*, not collapsing to
  one parse.
- The gate alone cannot achieve full `ngohaig` equality — Yune will still miss
  the oracle's 11 `o`-code rows and 午安 (under-admission, below). So `ngohaig`
  uses **directional** assertions, not full equality (Win Bars).

### The admission-depth lane (under-admission — deferred, not `nri`-specific)

Yune's admit rule `original_code_allows_prefix_fallback` (`mod.rs:2881`) matches
the candidate's **raw** dictionary code against the input letter, so it **drops
every code reachable only via the `ng`-insertion / abbreviation algebra**. This
under-admission is **not confined to `nri`** — the adopted `ngohaigo` capture
proves it appears in the flagship case too:

- `nri` / bare `n`: the oracle's vowel rows (安 `on1`, 屋 `uk1`, 愛 `oi3`, …
  ranks 8+) — Yune drops all 14.
- `ngohaig`(`o`): the oracle's **11 `o`-code rows** (柯 屙 哦 阿 痾 啊 珂 疴 軻 嚄
  婀, interleaved in the `ngo`-set) — raw code `o` ≠ lookup `ngo`, multi-char
  branch is exact-equal only → Yune emits **zero**. Plus 午安 (`ng5on1`), admitted
  by the oracle via ng-full + on-abbreviated — a **third** admission mechanism no
  leg touches.

Disposition: this whole lane (both `nri` vowel rows and `ngohaig` `o`-codes/午安)
is **deferred** with a single named root cause — the raw-vs-derived admit rule —
**or** deliberately pulled into scope as a Leg C (derived-form matching) if the
executor chooses. Either way:

- **Pin what is true:** keep the `m21_nri` golden (first-6, Owned) and state it
  covers only the first page-worth, not full-depth parity.
- **Do not silently claim parity** for `nri` or `ngohaig` full-depth.
- **Do not regress it:** the gate must not drop any oracle-present row; if
  derived-form matching is pulled in, capture and pin the newly-admitted rows.

## Decided Calls

- **No ABI widening.** No `RimeApi`/profile-slot/`yune_web_*` change.
- **Oracle-first, full-page.** Expected candidates come from TypeDuck-HK/librime
  `v1.1.2`, captured to `is_last_page:true`, never derived from Yune. Adopt the
  existing complete `ngohaigo` capture (`m21-closeout` case[4]) as ground truth
  rather than re-deriving it.
- **The oracle is multi-group, not exact-only.** For a parseable input it emits
  phrase + syllable-set(s) + algebra + alt-parse words in a stable order. The fix
  targets reachability + admission set membership, never "trim to exact."
- **Leg B is the confirmed core; Leg A (the gate) is capture-gated.** "No Leg A"
  requires *both* the `beingo` and `ngohaig` diffs clean — and `ngohaig`'s
  over-admission is already proven, so a `beingo`-only match cannot close Leg A
  globally.
- **Over- and under-admission are orthogonal.** The gate fixes over-admission
  (你/能/男); derived-form matching fixes under-admission (o-codes, `nri` vowel
  rows). Not alternatives for one defect.
- **Leg B raise is bounded and tone-scoped (per-tone), sized against all three
  caps.**
- **Defect B is compiled-path-only.** Reproduce/test on Compact+prism.
- **The admission-depth lane is deferred** (both `nri` vowel rows and `ngohaig`
  `o`-codes/午安), pinned narrow (`m21_nri` golden first-6); may be pulled in as
  an explicit Leg C.
- **Order parity is in the model.** The oracle interleaves by frequency; Yune
  sorts `consumed_input_len`-desc then quality — assert order for `beingo`,
  record it elsewhere.
- **Product lane only.** Upstream `luna_pinyin` Track A is out of scope unless a
  fixture shows the same defect there.

## Win Bars

Assertions are **directional and per-input** — full set+order equality is
targeted **only for `beingo`**, and even there the capture decides whether any
complete-syllable shorter-prefix or abbreviated-tail rows need a refined gate or
an explicit disposition. M58 closes when:

1. **(Leg B) 畀 reachable for `beingo`** — the full exact `bei*` set (incl the
   third `bei2` = 畀) emits, on the compiled Compact+prism product path (assert 畀
   absent pre-fix, present post-fix). Cap raise bounded and tone-scoped; target
   survives all three caps.
2. **`beingo` full equality target (set + order)** — Yune matches the full
   `beingo` capture (`is_last_page:true`, recorded product options), or any
   remaining set/order divergence is fixed or **recorded with an explicit
   disposition**. Pre-named risks: `be` rows (`啤`/`唄`) may survive the gate as
   complete-syllable shorter-prefix rows, and abbreviation-tail rows may appear
   in the oracle. Neither case may silently pass or trigger an unscoped gate
   change.
3. **`ngohaig` directional** (its complete admission model is the adopted
   `ngohaigo` capture, case[4] `default_combined`; `ngohaig` itself is captured
   full too or the proxy limit is stated): (i) **over-admission gone** — none of
   `你`/`能`/`男`/`女`/`內`/`呢` appear (Leg A gate); (ii) **no oracle row lost** —
   every oracle-present row Yune previously emitted is still present. The 11
   `o`-codes + 午安 remain missing → named in the deferred admission-depth lane,
   **not** asserted present.
4. **Toned-scope regression pinned** — adopt `fork-parity-06` (`neix`→0, checked
   in) as the tone-scoping oracle; `neix` stays empty post-Leg-B is a named row,
   and the toned input does not flood wrong-tone families.
5. **`zi`/`諮議局` diff-first** (keystrokes stated in Phase 0) — Yune matches the
   full capture where achievable; any completion admission-depth surprise is given
   the same directional/deferred treatment, not pre-committed full equality.
6. **`m21_nri` stays green**, stated to cover only first-6 (Owned); full-depth
   `nri`/`ngohaig` admission-depth parity documented deferred; the gate does not
   regress `nri`.
7. Oracle-driven tests assert candidate set **and order** from the full captured
   TypeDuck bytes (not from Yune); Defect-B coverage runs on the compiled product
   path; a named non-Cantonese control input is included.
8. **No Track B latency regression and no `web03` tripwire break**, both
   re-proven: the `benchmark-native-rime-inprocess` command in
   [roadmap.md §Current Guardrails](../../roadmap.md) with
   `-TrackBInputs neigojangingkeisatjinggoiziwunciucoenggeoizisyujapsinhojijung`,
   `-TrackAThresholds …/m55-thresholds.csv -FailOnRegression`
   **`-DeployProductBeforeBenchmark`** (macOS:
   `scripts/benchmark-native-rime-inprocess-macos.sh`); and
   `web03_byte_backed_jyutping_long_input_avoids_candidate_expansion_explosion`
   (`prefix_fallback_views_visited ≤ 5000/6000`) still passing.
9. `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   and the focused tests pass.

**Dispositions are per-input and per-direction.** "No Leg A needed" requires
*both* the `beingo` **and** `ngohaig` diffs clean — and the `ngohaig`
over-admission is already proven from checked-in bytes, so a `beingo`-only match
cannot close Leg A globally. Leg B alone closing the **headline 畀 bug** is a
valid milestone close only if (2)/(4)/(8) hold **and** `ngohaig`'s remaining
divergence is explicitly named+deferred, never read as "no divergence anywhere."
Close partial/no-go only if 畀 cannot be made reachable within the Track B
ratchet + `web03` tripwire, or if a fix would regress a pinned view (leading
commit, filter.rs/m24 shorter-prefix, completion, or the `nri` first-6 golden).

## Scope

In scope: the three fallback caps for leading-parse codes (Leg B, unconditional,
tone-scoped); `original_code_allows_prefix_fallback` admission scoping (Leg A,
capture-gated); full paginated oracle captures + tests; re-proving the Track B
ratchet and `web03` tripwire; a completion-ordering decision for `zi`.

Out of scope: ABI changes; `luna_pinyin` Track A; performance rebaselining
(beyond re-proving the gates); broad translator refactors; the **admission-depth
lane** (under-admission — `nri` vowel rows *and* `ngohaig` `o`-codes/午安 —
deferred, pinned narrow, unless deliberately pulled in as Leg C); flipping the
`prefix_fallback` YAML key.

## Phases

### Phase 0: Freeze the ground truth (captures are the design input)
- [ ] Record the failing Yune candidate order (paged, compiled Compact+prism
      product path) for `beingo`, `諮議局`, `zi`, a **letter-to-tone** input
      (adopt `fork-parity-06`'s `neix`→0 as the oracle; add a `bei`-tone analog
      if the schema defines one), and the `be`/`bein`/`being` intermediates, under
      `docs/reports/evidence/m58-jyutping-exact-before-fuzzy/phase-0/`. Pin the
      `beingo` fingerprint (exactly two per `bei*`, 畀 absent) and confirm 畀
      *is* present for bare `bei`.
- [ ] **Adopt existing complete oracle captures; do not re-derive them.** The
      complete admission model is `jyut6ping3-m21-closeout.json` case[4]
      (input `ngohaigo`, `is_last_page:true`, 49 cands, multi-group); the
      `nri`/bare-`n` flood is `jyut6ping3-m14-completion-correction.json`; the
      tone-scoping oracle is `jyut6ping3-fork-parity-06-letter-to-tone.json`
      (`neix`→0 already checked in). Reference these directly.
- [ ] **Record adopted-capture provenance and mirror it on the Yune side.** The
      corpus contains **same-input contradictions** by option/variant — e.g. bare
      `m` is 2 rows in `fork-parity-01` but 50 rows (flood) in `m21-closeout`
      case[2]; `ngohaigo` case[4] is `default_combined` while case[7] is
      `simplification_on`. So for every adopted capture, record its variant and
      options and configure the Yune diff to the **same** options; otherwise the
      diff compares apples to oranges.
- [ ] **Capture the still-missing full oracle lists** (`is_last_page:true`,
      shipping `jyut6ping3_mobile` options — page-fill matters, a page-0-only
      capture cannot conclude anything) for `beingo`, **full `ngohaig`** (note:
      `ngohaig` ≠ `ngohaigo` — the dangling `g` changes the phrase group and tail,
      so either capture it or state the `ngohaigo`-proxy assumption and its
      limits), `zi`, the toned input, and the exact ASCII keystrokes for `諮議局`
      (state the literal keystrokes in the fixture). **Reject any row where
      `captured_all_pages != true` or `pagination_error` is present.** Pagination
      groundwork is implemented in `scripts/oracle-rime-probe.cs`
      (`CaptureWithIdentity` loops on `Page_Down` `0xff56`, emitting
      `pages`/`all_candidates`/`captured_all_pages`) — the Windows session must
      compile-verify it and the executor must add a reusable capture preset (there
      is no `-Fixture M58` mode) or record the exact invocation. If capture is
      unavailable, block — do not proceed on Yune-defined expectations.
- [ ] **Define the divergence by diff, per direction.** For `beingo` and
      `ngohaig`, diff Yune's full output (at the recorded options) against the
      complete oracle: (i) **over-admission** — codes Yune admits the oracle omits
      (→ Leg A gate); (ii) **under-admission** — codes the oracle admits Yune omits
      (→ admission-depth lane, deferred); (iii) **order** — where the two disagree
      on ranking. This diff, not an "exact only" assumption, is the spec.
- [ ] Pick a **named non-Cantonese control** input with a known oracle behavior
      and record it (Win Bar 7).

### Phase 1: Instrument prefix selection and admission
- [ ] Add a dev-only diagnostic that reports, per input, the specs
      `valid_lookup_prefixes` emits, each spec's fetch code, and which candidates
      `original_code_allows_prefix_fallback` admits (raw code vs derived form),
      so the over/under-admission is visible against the capture diff.

### Phase 2: Fix reachability (Leg B) then admission (Leg A, gated)
- [ ] **Leg B (unconditional).** Raise the per-fetch cap for leading-parse fetch
      codes, bounded to the oracle-observed max and tone-scoped; if the 256/64
      caps are the binding truncation for the target character, raise those
      (bounded) too. Keep caps on non-leading-parse codes unless a win-bar
      capture-match requires a scoped exception.
- [ ] Add a compiled-path test: `beingo` emits 畀 (absent pre-fix, present
      post-fix) and the toned input does not flood wrong-tone families.
- [ ] **Leg A = the gate (per the Phase 0 over-admission diff).** Drop the
      non-syllable single-letter prefix when a longer complete-syllable prefix
      contributes (kills `ngohaig`'s `你`/`能`/`男`, `beingo`'s `b`-flood;
      preserves `nri` flood and `mgoi`/`m` abbreviations). Complete-syllable
      detection via the lookup path (`mod.rs:2328`/`:1164`). Do not touch
      complete-syllable shorter prefixes (filter.rs/m24). Leave the
      under-admission (o-codes, `nri` vowel rows) to the deferred admission-depth
      lane unless deliberately pulling in derived-form matching as Leg C.
- [ ] Add real-path tests: `beingo` **full set+order** equality vs its capture;
      `ngohaig` **directional** (no over-admission rows; no oracle row lost); the
      control input; `neix` stays empty (tone-scope); `m21_nri` stays green.

### Phase 3: Completion ordering decision (`zi`)
- [ ] With the `zi` capture, decide whether completion candidates
      (`zi`→`zing`/`zik`) precede or follow exact-syllable characters; implement
      to match, with a test tied to a win bar.

### Phase 4: Re-verify and close
- [ ] `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D
      warnings`, focused tests, the `web03` tripwire, and **re-prove the Track B
      latency ratchet** (with `-DeployProductBeforeBenchmark`; macOS uses
      `scripts/benchmark-native-rime-inprocess-macos.sh`). Mandatory — Leg B
      changes the fallback set size, and Track B already saturates the 64 cap.
- [ ] Write `docs/reports/evidence/m58-jyutping-exact-before-fuzzy/` with
      before/after captures, the oracle diff, and the Leg A disposition.
- [ ] Update roadmap/requirements/milestone-history on closeout; move this plan
      to `plans/completed/`.

## Review Checklist For Claude

- Does the plan treat the oracle as **multi-group** (per `ngohaigo` case[4]),
  never "exact only"? Any "stop at the exact set" language is a v1/v2 relapse.
- Is Leg B (the confirmed core) tested on the **compiled Compact+prism path**,
  bounded, tone-scoped, and sized against **all three** caps (2 / 256 / 64)?
- Are **over- and under-admission** kept orthogonal — Leg A is the **gate**
  (fixes over-admission `你`/`能`/`男`), and derived-form matching is the deferred
  admission-depth lane (fixes `o`-codes, `nri` vowel rows), **not** two options
  for one defect?
- Are assertions **directional and per-input** — `beingo` full set+order,
  `ngohaig` directional (no over-admission, no oracle row lost, the 11 `o`-codes/
  午安 named-deferred not asserted-present)?
- Does Leg A leave complete-syllable shorter prefixes (filter.rs/m24) intact and
  key on **is-a-complete-syllable**, not is-single-letter (so `m` survives)?
- Is "complete-syllable prefix" detected via the **lookup path**
  (`valid_lookup_prefixes`/`sentence_lookup_specs`), never a segmentor span or the
  luna-only sentence model?
- Is capture **provenance** recorded (variant/options — `ngohaigo` case[4] is
  `default_combined`) and mirrored on the Yune side of the diff?
- Is **order parity** handled — asserted for `beingo`, recorded elsewhere (the
  oracle interleaves by frequency; Yune sorts by consumed-length-then-quality)?
- Are the Track B ratchet (with `-DeployProductBeforeBenchmark`) and the `web03`
  `prefix_fallback_views_visited` tripwire both re-run, not assumed?
- Are all expectations from `is_last_page:true` captures, not from Yune?

## Non-Goals

- Do not change `luna_pinyin` Track A behavior.
- Do not "trim to the exact set" — the oracle emits a rich multi-group list.
- Do not drop complete-syllable shorter prefixes (filter.rs / m24 pin them).
- Do not claim full-depth admission parity (`nri` vowel rows or `ngohaig`
  `o`-codes/午安); that lane is deferred and unpinned.
- Do not assert full set+order equality for `ngohaig` (unachievable by the gate
  alone — under-admission remains); use directional assertions.
- Do not flip the `prefix_fallback` YAML key as a shortcut.
- Do not assert candidate order from Yune; assert it from the TypeDuck oracle.
