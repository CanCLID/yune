# M23 — Architecture Hardening (bounded)

> **Status:** Active · **Milestone:** M23 (architecture hardening) · **Updated:** 2026-06-20 · **Type:** execution plan

A small, **finishable** debt-paydown milestone that lands the four reviewable
items from the architecture-hardening register (`decisions.md` D-28) before
Track 2 / M22 multi-schema / native / iOS work deepens the dependency on them.
It is deliberately scoped so it can be declared **green and closed** — it is not
the open-ended core/ABI processor extraction (that stays trigger-gated; see
*Non-goals*).

**Why now.** The M21 closeout introduced a TypeDuck-calibrated tuning constant
into a *shared* engine path (the `21.0` sentence word penalty), which silently
affects default upstream `luna_pinyin` — a direct tension with the upstream-first
standing principle (D-24). The review behind D-28 also confirmed three other
finishable debts: an inert workspace lint policy, an orphaned `yune-schema`
crate, and oversized single-file test modules. None requires a rewrite; all four
are behavior-preserving cleanups.

**Standing rules this milestone obeys.** Behavior-preserving only (D-05 / §6
ownership). Every existing oracle, ABI, and browser gate stays green and
unchanged. No `RimeApi` / `RimeCandidate` change. No classic-input default
change.

---

## Ordering constraints (read before sequencing)

- **WI-1 must land before M19.** M19 onboards new schemas (`double_pinyin`,
  `cangjie5`, `bopomofo`) that flow through the **same** shared
  `sentence_candidate` path that currently reads the TypeDuck `21.0` constant
  unconditionally. Gating the constant first prevents those new upstream schemas
  from inheriting Cantonese tuning the moment they land.
- **WI-2 should land before M18.** M18 adds new dictionary-writer code; enabling
  the lint policy first means that code is governed from birth instead of
  retrofitted.
- WI-3 and WI-4 have no external ordering dependency and can land any time within
  M23.

---

## Work items

### WI-1 — Gate the TypeDuck profile constants out of the shared default path *(highest priority)*

**Problem (verified 2026-06-20).** `crates/yune-core/src/translator/mod.rs`
defines file-level constants consumed in shared code with no profile gate:

- `TYPEDUCK_SENTENCE_WORD_PENALTY: f32 = 21.0` (`mod.rs:16`) is read
  unconditionally by `sentence_piece_quality()` (`mod.rs:75-76`), whose only call
  site is the shared sentence-composition DP loop (`mod.rs:991`). `enable_sentence`
  defaults **true** for every schema (`schema_install.rs:166-167`) and
  `table_translator`/`script_translator`/`r10n_translator` all build the same
  `StaticTableTranslator`, so **upstream `luna_pinyin` inherits the Cantonese-tuned
  penalty whenever it composes a sentence candidate.** This is the confirmed leak.
- `TYPEDUCK_CORRECTION_CREDIBILITY: f32 = -16.118_095` (`mod.rs:13`) and
  `TYPEDUCK_CORRECTION_MAX_DISTANCE: usize = 4` (`mod.rs:14`) are used at
  `mod.rs:69,438-440,468-470,649`. These are *mostly* masked (`enable_correction`
  defaults off; the dynamic-correction lookup is already profile-gated via
  `with_dynamic_correction_lookup(is_typeduck_jyut6ping3_profile)`,
  `schema_install.rs:290`) — but they must be **audited** to confirm no shared
  default path reaches them, then handled the same way.

**The correct pattern already exists in the tree.** The M21-GAP-02 prediction
limit is threaded as a typed translator field (`with_prediction_candidate_limit`,
`schema_install.rs:308-309`) and only enabled for the named profile predicate
`is_typeduck_jyut6ping3_profile()` (`schema_install.rs:317`), config-overridable
and defaulting to none. Mirror that.

**Approach (pick per constant, smallest that holds parity):**
1. Thread the value as a typed translator config field defaulting to the
   upstream-neutral behavior, set to the TypeDuck value only at install time
   behind `is_typeduck_jyut6ping3_profile()`; **or**
2. if the oracle proves the value is genuinely global (upstream `luna_pinyin`
   sentence parity is unchanged by it), keep it shared but **rename it neutrally**
   (drop the `TYPEDUCK_` prefix) and document it as global sentence-composition
   tuning. Decide by experiment, not assertion.

- **Owning module:** `crates/yune-core/src/translator/mod.rs`; install wiring in
  `crates/yune-rime-api/src/schema_install.rs`.
- **Owning tests:** `crates/yune-core/src/tests/translator.rs` for the new
  field/default; the parity gates below are the acceptance oracle.
- **Acceptance:**
  - No `TYPEDUCK_*` constant is read on a code path reachable by a default
    upstream schema unless renamed-and-proven-global per option 2.
  - `cargo test -p yune-core --test upstream_luna_pinyin_parity` green.
  - `cargo test -p yune-core --test cantonese_parity` green **with the M21
    fixtures byte-unchanged** (no observable TypeDuck `jyut6ping3` behavior change).
  - `cargo test --workspace` green.

> A background task chip was already spawned for this slice; it can be executed
> standalone or rolled into M23. Either way it is the first thing to land.

### WI-2 — Make the workspace lint policy actually enforce

**Problem.** `Cargo.toml` declares `[workspace.lints.rust] unsafe_code = "forbid"`
and `[workspace.lints.clippy] all/pedantic = "warn"`, but **no member crate opts
in** with `[lints] workspace = true`, so the policy is inert over a cdylib FFI
surface with thousands of `unsafe` sites.

**Design note / footgun.** `forbid(unsafe_code)` **cannot** be locally overridden
by `#[allow(...)]`, and a crate that sets `[lints] workspace = true` inherits the
*entire* workspace table (you cannot add or subtract a single lint alongside it).
So the FFI crate cannot both inherit the workspace forbid and keep its `unsafe`.
Recommended shape:

- Keep `unsafe_code = "forbid"` + clippy pedantic in `[workspace.lints]`.
- `yune-core`, `yune-schema`, `yune-cli` → add `[lints] workspace = true`
  (they are, or should be, unsafe-free; this makes the forbid real where it
  matters).
- `yune-rime-api` → define its **own** `[lints]` table (not `workspace = true`):
  apply clippy `all`/`pedantic = "warn"` and set `unsafe_code = "allow"` (or
  `"deny"` with documented module-level `#[allow]`s) as the **explicit, documented
  FFI exception**. Reference this exception in §4/§9 of `CONVENTIONS.md`.

- **Owning file:** root `Cargo.toml` + each `crates/*/Cargo.toml`.
- **Acceptance:** `cargo clippy --workspace --all-targets -- -D warnings` is clean;
  introducing `unsafe` in `yune-core`/`yune-schema`/`yune-cli` fails the build;
  the `yune-rime-api` exception is documented, not implicit. `cargo test --workspace`
  green.

### WI-3 — Resolve `yune-schema` (promote or park/delete)

**Problem.** `yune-schema` is a workspace member but an **orphan**: no crate
depends on it, no `use yune_schema` exists anywhere, and production schema parsing
lives in `yune-rime-api` (`config.rs`, `schema_install.rs`, `schema_selection.rs`).
The architecture diagram implies it is "the schema layer," which is currently
misleading.

**Decision required (product call):**
- **Promote** — make `yune-rime-api` parse/install schemas *through* `yune-schema`
  so the crate becomes the real typed schema model; or
- **Park/Delete** — remove it from the workspace (or clearly mark it
  experimental/parked in its own `Cargo.toml` + a one-line README) so no doc or
  diagram implies it is load-bearing.

Recommended default: **delete it from the workspace** unless there is a concrete
near-term plan to route production parsing through it — a parked-but-present crate
is exactly the ambiguity that caused the confusion. Re-add later if M18/M19 wants a
shared typed schema model.

- **Owning files:** `crates/yune-schema/`, root `Cargo.toml` members list, the
  `CONVENTIONS.md` architecture diagram + §2 crate list.
- **Acceptance:** the repo no longer contains an orphan-but-implied-live crate;
  whichever choice is made is reflected in the architecture diagram and §2.
  `cargo build --workspace` / `cargo test --workspace` green.

### WI-4 — Test-module hygiene (behavior-preserving)

**Problem.** The largest files in the tree are test modules, and behavior
ownership is hard to scan:

- `crates/yune-core/src/lib.rs` is ~95% inline tests: `mod facade_tests`
  (`lib.rs:157` → EOF, ~3.2k lines) sits in what should be a thin facade.
- `crates/yune-rime-api/src/tests/schema_selection.rs` (~8.2k),
  `crates/yune-rime-api/src/tests/schema_processors.rs` (~7.0k), and
  `crates/yune-rime-api/tests/frontend_client.rs` (~4.3k) are oversized
  single-file modules.

**Approach.** Move `yune-core/src/lib.rs`'s `facade_tests` into behavior-owned
modules under `crates/yune-core/src/tests/` per §6; split the three large ABI test
modules **only along behavior ownership lines** (e.g. by processor / by schema-
install concern / by frontend-client scenario). Pure mechanical moves — **no
assertion added, removed, or changed.**

- **Owning files:** the test modules above + their `mod` wiring.
- **Acceptance:** no facade file is dominated by inline tests; no single test
  module is the largest file in its crate by a wide margin; identical test count
  and assertions before/after (`cargo test --workspace` lists the same tests and
  is green). Lowest-urgency item; can land last or incrementally.

---

## Non-goals (explicitly out of M23)

- **Core/ABI processor extraction.** Moving the RIME processor pipeline from
  `yune-rime-api/src/processors/` into a core-owned Rust API is the large,
  **trigger-gated** D-28 item — it lands when a real non-ABI consumer (iOS
  package, Yune-native frontend) needs the full input pipeline, not speculatively.
  M23 does **not** touch it.
- **Process-global → multi-instance refactor.** Designing the instance boundary is
  future work tied to product/native expansion (D-28 / §9), not M23.
- Any behavior change, ABI change, or new feature.

---

## Sequencing within M23

1. **WI-1** (gate constants) — first; unblocks M19 safely.
2. **WI-2** (lints) — before M18 starts adding writer code.
3. **WI-3** (yune-schema) — independent; quick decision.
4. **WI-4** (test hygiene) — last / incremental; lowest urgency.

WI-1 and WI-2 are the gating pair for the rest of the roadmap; WI-3 and WI-4 are
opportunistic within the milestone.

## Milestone close (acceptance gate)

M23 is **done** when: WI-1–WI-3 are complete and WI-4 is complete or explicitly
deferred-with-a-reason; `cargo test --workspace`,
`cargo clippy --workspace --all-targets -- -D warnings`,
`upstream_luna_pinyin_parity`, `cantonese_parity`, and the TypeDuck-Web gates are
all green; and the M21 fixtures are byte-unchanged. Flip this banner to
`Finished`, move the doc to `plans/archive/`, and update `roadmap.md` +
`decisions.md` D-28 outcome.

## Links

- `decisions.md` D-28 (architecture-hardening register) and the standing
  upstream-first / profile-isolation principles (D-24, D-25).
- `fork-parity-ledger.md` → "Yune-vs-librime architecture crib sheet" (profile
  tuning + core/input split rows).
- `CONVENTIONS.md` §4 (lint gate), §6 (module/test ownership), §9 (known risks).
- `roadmap.md` → *Planned / Next up* → execution order.
