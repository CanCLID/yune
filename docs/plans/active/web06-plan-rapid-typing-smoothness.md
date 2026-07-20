# WEB-06 Rapid-Typing Smoothness And Fair Browser Peer Measurement

> **Milestone:** WEB-06. **Status:** Active — independent specification and
> quality/safety reviews **PASS** (2026-07-18); the user parked M62 and
> explicitly authorized WEB-06 execution on 2026-07-19.
> **Track:** `yune-web` browser input delivery and presentation. **Created:**
> 2026-07-17. **Revised:** 2026-07-19. **Type:** measurement-contract
> correction, attribution-first browser optimization, and real-browser release
> gate.
>
> The user explicitly authorized this reviewed plan for execution on
> 2026-07-19 and parked the separate protected M62 macOS native-memory draft.
> M62 remains intact outside this isolated WEB-06 branch and must not be folded
> into WEB-06 work. Authorization freezes every reviewed threshold, formula,
> cadence, scenario, invalidation rule, and retry limit below unchanged. It
> permits local commits and one source-pinned preview canary. It does **not**
> permit a Git push, production promotion, or any other external deployment
> mutation; after a green preview, execution stops for explicit user approval
> at that exact gate.

## Outcome

WEB-06 has one binding product goal: make rapid classic input in the shipped
`yune-web` public artifact remain visually current without losing, reordering,
or changing the meaning of any RIME key event.

WEB-06 earns the phrase **rapid typing is smooth under the declared browser
lane** only when an exact-source, exact-artifact real-browser run proves all of
the following:

1. the browser observes every declared DOM event exactly once, classifies and
   maps it through the frozen event-to-zero/one/many-action contract, and the
   owning frontend/adapter/native layer completes every resulting action exactly
   once and in order;
2. sustained 60 ms input and the declared burst pattern meet the frozen
   current-state paint, queue, frame-pacing, and recovery ceilings;
3. final composition, candidate, paging, selection, commit, correction, and
   learned-state behavior remain correct under their existing oracle or
   fixture authority;
4. the corrected short Luna peer lane uses the same interaction endpoint for
   Yune and a reproducibly pinned, input-relevant-data-equivalent My RIME
   artifact; the sustained Luna row remains explicitly informational unless
   its independent row-level `packageAlignment=PROVED` too;
5. the source-identical preview/deployment path remains fail-closed; and
6. the existing WEB03-11 gate stays closed and green without rewriting its
   historical contract or evidence.

The milestone is not satisfied by adding a test that accepts the current
experience, by improving native engine timing alone, or by reporting a faster
number from a different endpoint. If the absolute Yune gate passes but the
pinned peer lane cannot be reproduced, the closeout is **partial — Yune
smoothness passed, fair peer verdict unavailable**. If no safe owner clears the
absolute gate, close as a measured no-go with the remaining component and
threshold miss named.

## Why A New WEB Milestone Is Required

Native Track A results do not include browser event delivery, the JavaScript
action queue, worker messaging, response decoding, React reconciliation, layout,
or frame presentation. The repository explicitly keeps native and browser
performance evidence separate in [conventions](../../conventions.md#9-key-risks--concerns-current).

The current browser hard stop is real but does not prove the product property
in this plan. Its normal lane types one 47-key Jyutping input at 100 ms/key and
allows p95 `150 ms`, max `250 ms`, and queue wait `100 ms`; its stress lane
types at 250 ms/key and allows p95 `750 ms` and max `1000 ms`. See the
[WEB03-11 procedure](../../../apps/yune-web/e2e/yune-browser-smoke.md#automated-playwright-entry-points).
A recurring `72 ms` delay is therefore green under the existing release
contract even though it spans more than four nominal 60 Hz frame intervals.

The current path also contains two diagnosed risks that require a dedicated
owner rather than another loose integration test:

- [`rime.ts`](../../../apps/yune-web/src/rime.ts) uses one stateful FIFO and one
  in-flight action for key, schema, option, candidate, persistence, userdb, and
  AI work. A preserved fast-input receipt contains a `74 ms` final-key result
  with `55 ms` of queue wait and only `1 ms` of worker processing
  ([receipt](../../../apps/yune-web/e2e/results/web03-latency-regression-fix/local-browser-latency/summary.csv)).
- The main thread parses and serializes rolling diagnostic arrays into DOM
  datasets on each key, maps rich candidate comments, updates several parent
  states, and can reconcile the complete userdb table. These are candidate
  owners, not pre-approved conclusions.

The browser peer result also needs correction before it can guide an
optimization. The existing
[comparator](../../../apps/yune-web/e2e/yune-web-comparator-benchmark.spec.ts)
stops Yune after any new candidate-bearing diagnostic, while the My RIME side
waits for the complete input and candidate. Retained
[`ni` samples](../../../apps/yune-web/e2e/results/yune-web-vs-my-rime-baseline/current-dashboard/samples.json)
timed Yune's earlier `n` response (`那`) while the final DOM showed `你`. The
historical packet remains evidence of the old method, but
its `74 ms` versus `95 ms` / `0.779x` input-to-candidate reading is not valid
fair-peer evidence. The downstream `0.899x` commit reading is tainted too,
because Yune begins commit timing after that premature candidate endpoint while
My RIME begins only after the complete-input endpoint.

## Authority, Sequence, And Source Boundary

### Milestone identity

- WEB-06 is a new browser track after completed WEB-05. It does not reopen
  WEB-03 or renumber M62.
- The current [roadmap](../../roadmap.md) records WEB-06 as active and M62 as
  parked by the user's explicit 2026-07-19 authorization.
- No existing M61/M62 planning edits may be folded into a WEB-06 commit.

### Clean source and artifact identity

Execution is authorized. Phase 0 first records
one clean pre-instrumentation `WEB06_PRODUCT_SHA`. The reviewed measurement-only
commit based on that source becomes `WEB06_BASE_SHA`; Phase 1 RED and the
final candidate use the same metric event semantics, collector/observer
configuration, parser, and verdict arithmetic. The instrumentation-only and
final candidates, plus the unchanged PRODUCT/no-probe control, must each be
built once through the pinned public-demo entrypoint, sealed, and identified by:

- source commit and clean-tree state;
- Rust, Emscripten, Node, npm, Playwright, and Chromium versions;
- OS, architecture, CPU, logical-core count, memory, power state, browser
  viewport, and display-refresh calibration;
- `build-info.json`;
- app bundle, worker, WASM, schema manifest, split-asset, and complete public
  artifact hashes; and
- exact command, environment, cadence, scenario registry, and metric-contract
  version.

The metric surface has three sealed modes. **PRODUCT/no-probe** is the unchanged
`WEB06_PRODUCT_SHA`. A bounded **BASE/minimal paint probe** records event/action
identity and current-state/terminal double-rAF timestamps and remains invariant
across baseline, final, and the shipped production-default build. A **BASE/full
diagnostic collector** adds raw component spans and retained receipts for the
binding test. One external Playwright DOM sentinel and parser is common to all
three modes; it records atomic DOM state/terminal tuples stable through
double-rAF, driver/page timing, continuous frames/Long Tasks, and exact declared
event/visible-sample counts without using the app's internal diagnostic result.
Minimal/full collectors use bounded in-memory rings and out-of-band worker
metadata, perform no per-action DOM JSON serialization or console logging, and
export once after the timed window. The external sentinel records callback
self-time; each callback must be `<0.5 ms` and total sentinel self-time per
event `<1.0 ms`.

Before the binding RED is accepted, exactly five valid counterbalanced
fresh-context triplets within at most seven retained attempts
over the pre-optimization Rapid long Jyutping row compare (A)
PRODUCT/no-probe versus BASE/minimal and (B) BASE/minimal versus BASE/full. Mode
is fixed before page load and never toggled mid-round. A valid mode red is never
replaced; failure to obtain five valid triplets is setup no-go. For each
comparison, the absolute pooled-median delta must be `<=1.0 ms`, absolute
pooled-p95 delta `<=2.0 ms`, and absolute pooled-max delta `<=4.0 ms`. The compared modes may not
disagree on any common-surface paint/terminal/frame/Long-Task/count verdict.
Minimal/full additionally compare every internal action/queue verdict and exact
count. Instrumentation may add no in-window `>=50 ms` Long Task and no
individual main- or worker-side bookkeeping callback `>=5.0 ms`.

The review freezes the three modes, common parser, counterbalancing, and ceilings
before measurement. If the minimal probe creates a product-visible RED, the
full collector changes a verdict, a parser manufactures the improvement, or the
final build omits the reviewed minimal production probe, the measurement cannot
support a verdict.

The baseline and final use the same host, browser, viewport, toolchain,
prebuilt schema assets, cache regime, scenario order, and evidence parser. Only
the reviewed WEB-06 implementation, owning tests, required release plumbing,
and closeout documentation/evidence may differ. A dirty planning worktree, an
ambient dev server, or a live deployment whose source cannot be matched is
diagnostic only.

### Behavior and ABI authority

- Candidate behavior continues to follow the target/oracle precedence in
  [conventions](../../conventions.md#1-overview--architecture) and the
  [engine support contract](../../contracts/engine-support-contract.md).
- A before/after Yune comparison proves regression or improvement, not oracle
  parity. Expected candidate or commit bytes may not be derived from Yune.
- WEB-06 may change `apps/yune-web`, its Playwright gates, and, only if Phase 1
  attribution proves ownership, optimize the TypeScript-only
  `packages/yune-web-runtime` request/response transport path. Phase 0 has one
  narrower exception: its instrumentation-only commit may add behavior-neutral
  raw timing spans inside
  [`runtime.ts`](../../../packages/yune-web-runtime/src/runtime.ts) and
  [`response.ts`](../../../packages/yune-web-runtime/src/response.ts) for the
  required ABI, byte-extraction, and JSON-parse raw spans.
  That exception must stay within the observer-overhead ceilings and preserve
  request/response shape, pointer/free ownership, JSON bytes, ordering, and
  error semantics; it does not authorize a Branch C optimization. Release
  integration may make
  narrowly scoped changes to `.github/workflows/deploy-yune-web.yml`,
  `apps/yune-web/public-demo/certify-public-release.sh`, e2e package commands,
  and their policy tests. The source classifier, secret-free certification,
  archive sealing, preview/production credentials, identical-byte promotion,
  and production hash interlocks remain frozen.
- Public runtime `Action` inputs and native/runtime response shapes remain
  frozen. Phase 0 may introduce a versioned private `yune-web` main-to-worker
  control/metadata envelope for event/action sequence IDs, timing metadata, and
  out-of-band clock ping/echo. The worker must strip that envelope before the
  public runtime call and attach private receipt metadata outside the public
  response payload; private `engineRaw` capture occurs before adapter projection.
  No consumer-visible field or native JSON byte may change.
- The default C ABI, TypeDuck profile ABI, Rust engine behavior, schema assets,
  candidate page size, and the `yune_web_*` export allowlist are frozen.
  Attribution that requires a Rust change or a new/changed export stops WEB-06
  and requires a separately reviewed engine proposal.
- AI remains default-off and is not part of the binding classic-input result.

## Planned Requirements

- [ ] **WEB06-CORRECT-01:** Correct the comparator so both Yune and My RIME
  start and stop on the same declared input/candidate endpoint; prove for each
  app that the timed observation, rendered composition, visible candidate
  collection, and action sequence belong to one coherent render of the
  complete input. A body-`innerText` or candidate-bearing-line heuristic is not
  a binding observer. Retain the old packet and add a
  dated correction to every current interaction-latency claim derived from the
  flawed packet, including `0.779x` candidate and `0.899x` commit, before
  forming a replacement ratio.
- [ ] **WEB06-MEASURE-01:** Add a focused real-browser smoothness harness with
  raw event-delivery, action queue, worker, roundtrip, mapping, React commit,
  current-state double-rAF proxy, continuous frame-gap, long-task, supersession,
  and queue-depth timestamps; publish a versioned metric contract and prove its
  clocks, ordering, exact expected sample counts, minimal/full observer
  overhead, and recomputation from raw timestamps.
- [ ] **WEB06-BASELINE-01:** Capture exactly five valid source-current baseline
  rounds over the exact normal, rapid, burst, correction, commit, selection,
  paging, and peer scenarios below; preserve every valid red with no retry;
  freeze the scenario registry, record the already-authorized thresholds
  unchanged, and freeze the environment rules and one attributed owner before
  production code changes.
- [ ] **WEB06-REDUCE-01:** Land one production-default browser-path reduction
  against the frozen owner without benchmark-only shortcuts, feature removal,
  smaller candidate pages, slower input cadence, hidden resource reuse, stale
  result substitution, metric clamping, or behavior changes; final Yune bytes
  must pass every absolute smoothness and compatibility gate.
- [ ] **WEB06-SCHED-01:** If and only if Phase 1 attributes a binding miss to
  queueing, preserve every event's frozen zero/one/many-action mapping and every
  mapped action/stateful barrier exactly once and in order while allowing only
  explicitly recorded stale presentation snapshots to be superseded; commits,
  digit selection, paging, Backspace, modifier releases, option/schema/deploy
  barriers, and persistence may never be discarded or reordered. If queueing
  is not the selected owner, independent evidence must
  disposition this requirement as `N/A — precondition false`; it is never
  silently skipped.
- [ ] **WEB06-COMPAT-01:** Preserve the exact externally owned candidate/commit
  guards where they exist, record each claim as `oracle-exact`,
  `contract-exact`, or `latency-and-page-shape-only`, preserve six-row page
  shape where already required, userdb learning and reload, schema switching,
  classic default behavior, and all existing WEB03-11 assertions; prove every
  rapid DOM event has the frozen action cardinality, every mapped action is
  completed by its owning layer, and every printable result is either painted
  or linked within the same deadline to a later monotonic prefix-extension in
  the same deletion-free supersession sub-run. Every barrier action has exactly
  one non-superseded terminal outcome. A shape-only row cannot support a
  candidate-parity claim.
- [ ] **WEB06-PEER-01:** Build or otherwise reproduce one locally served,
  hash-pinned My RIME artifact from an explicitly reviewed source commit and
  schema package; prove identical logical schema/dictionary/essay plus any
  optional sentence/poet or `.gram` model (explicit `none` for plain Luna) and
  effective-configuration inputs while recording each engine's compiled table,
  prism, reverse, data/model, and runtime hashes; and run same-browser,
  same-viewport, same-cadence, same-cache, same-input, same-endpoint Luna
  samples. The short `ni` row is the candidate binding peer row only with
  `packageAlignment=PROVED` and a coherent render observer. The sustained Luna
  row is informational unless its independent row-level alignment is also
  `PROVED`. Live `my-rime.vercel.app`, an `innerText` heuristic, and
  dictionary-confounded Jyutping remain informational and cannot satisfy this
  requirement.
- [ ] **WEB06-RELEASE-01:** Run the final focused gate over the exact sealed
  public artifact built/certified once through the normal release path, retain
  WEB03-11 unchanged, and exercise a byte-identical preview canary once. Stop
  after the green preview and request explicit user approval; only if that
  approval is received may the identical archive be promoted and production
  identity verified, without a second build or replacing the binding local
  measurement with a noisy shared-host rerun. Git push is likewise outside the
  current authorization.
- [ ] **WEB06-EVIDENCE-01:** Preserve full JSON, traces, screenshots, and logs
  outside Git; curate only compact source-bound summaries, threshold verdicts,
  hashes, commands, and a manifest; pass link/diff/growth/privacy checks and two
  independent reviews before closeout.

## Metric Contract

### Required raw timestamps

The total event map classifies every timed DOM event as `mapped-action(s)`,
`frontend-consumed(reason)`, or `browser-pass-through`; every mapped or
in-window background action is classified `native-key`, `adapter-only`,
`read-only`, or `stateful-barrier`. Only `native-key` may support an “engine
processed the key” claim. Every `keyof Actions` used during an interaction
window must appear in the frozen map; an unclassified event/action is a behavior
RED.

Each declared DOM event receives a monotonic `eventSequenceId`; each mapped
interactive action receives a monotonic `sequenceId`, an explicit
`compositionEpochId`, and a `supersessionSubRunId`. The raw receipt must retain,
when applicable:

1. requested driver dispatch, actual driver dispatch, DOM event `timeStamp`,
   and normalized event time;
2. DOM event type/code/key/modifiers, `eventSequenceId`, mapped action
   cardinality and ordered action IDs/kinds/arguments/classifications, event
   classification, and handler entry (`eventDeliveredAt`);
3. action enqueue, main-queue depth, and worker-dispatch depth;
4. worker send; first-statement worker message receive; worker action start
   after any initialization/dispatch wait; worker finish; and first-statement
   main-thread response receive;
5. worker-internal spans for ABI call, response-byte extraction, JSON parse,
   adapter translation, and persistence when those stages run;
6. main-thread response mapping start and finish;
7. React state-update scheduling and an exact-sequence layout-effect commit;
8. first rAF and second-rAF current-state or exact terminal observation;
9. linked `engineRaw`, `presentationExpected`, and independently observed
   `domObserved` state fingerprints;
10. continuous rAF frame timestamps covering the complete timed window;
11. every `>=50 ms` main-thread Long Task entry available in the binding
    Chromium runtime, with its overlap against each continuous interaction
    window; and
12. outcome: `painted`, `superseded`, `committed`,
    `processed-no-visual-change`, `barrier-completed`, or explicit failure.

`engineRaw` retains the raw logical composition input/context, action kind,
epoch/sub-run, raw candidate/status bytes, and page data before adapter/UI
projection. `presentationExpected` is the deterministic adapter/view projection
for the active comments/options/page posture. `domObserved` independently reads
the rendered input/preedit, page number, highlighted index, ordered visible
candidate text/label/comment digest, status bits, and action sequence token.
Exact presentation means `presentationExpected == domObserved`; raw engine and
DOM bytes are not required to equal when the reviewed UI projection normalizes
or omits fields. The raw logical-input action sequence remains authoritative for
supersession prefix proof.

The originating `sequenceId` must travel with the pending React state through
update scheduling, layout-effect commit, and double-rAF observation; matching a
pending diagnostic by input/candidate text, splicing it away, or silently
dropping it is forbidden. Commit fingerprints additionally contain exact commit
text plus textarea value, selection/caret, and visible composition state after
the action. Human-interactive non-supersedable actions record
`terminalObservedAt` after a reviewed second-rAF observation of the exact
changed or cleared DOM/commit state.

Binding receipts use unrounded, per-realm `performance.now()`/event timestamps
plus the declared clock conversion. Existing rounded `queueWaitMs`/`totalMs`
fields and epoch-summed DOM-dataset values remain legacy diagnostics only; they
may be displayed but never feed a WEB-06 verdict or recomputation.

The binding Chromium lane freezes one event-clock rule:
`normalizedEventAt = event.timeStamp` directly in the page
`performance.now()` domain. A synthetic untimed keyboard event before each
round must prove the event timestamp shares that origin and is monotonic; no
epoch-magnitude heuristic, `Date.now()`, time-origin addition, or fallback mode
is allowed. A nonfinite, negative, decreasing, or wrong-origin event timestamp
is a metric setup failure, not a value to clamp. Timed event delivery may still
be arbitrarily late and remains measured by `eventDeliveryMs`.

The Playwright driver clock is calibrated into the page main-thread clock with
nine ping/echo exchanges immediately before and after each round. For driver
send/receive `d0/d3` and page receive/send `m1/m2`, compute
`driverPageOffset = ((m1 - d0) + (m2 - d3)) / 2` (page minus driver) and
`netRtt = (d3 - d0) - (m2 - m1)` and `uncertainty = netRtt / 2`. Every raw
value and `netRtt` must be finite and `netRtt >=0`; otherwise calibration is
setup-invalid. Select and linearly interpolate the minimum nonnegative
uncertainty pre/post exchanges. Selected and interpolated uncertainties and the
absolute pre/post offset drift must be in `0..2.0 ms`. Convert the timestamp
captured immediately before each Playwright keyboard call as
`correctedDriverDispatchAt = driverDispatchAt + interpolatedDriverPageOffset`.
Missing calibration or boundary data is setup-invalid.

Each phrase repetition records one continuous interaction window from the
last boundary rAF at or before its first `correctedDriverDispatchAt` through the
boundary rAF after its final exact covering paint or terminal observation.
Frame and Long Task gates apply to that complete window, including dispatch to
DOM delivery, gaps between keys, and every commit/barrier interval; a task
cannot escape because it falls before the DOM event timestamp or between two
per-key intervals.

Worker-local timestamps are not subtracted from main-thread timestamps until
they are converted into the main-thread clock domain. Each round performs nine
ping/echo exchanges immediately before and nine immediately after typing. For
main send/receive `m0/m3` and worker receive/send `w1/w2`, each exchange
computes `offset = ((w1 - m0) + (w2 - m3)) / 2` (worker minus main) and
`netRtt = (m3 - m0) - (w2 - w1)` and `uncertainty = netRtt / 2`. Every raw
value and `netRtt` must be finite and `netRtt >=0`; otherwise calibration is
setup-invalid. Each set selects the minimum nonnegative-uncertainty exchange.
Worker timestamps convert as
`correctedWorkerAt = workerAt - interpolatedOffset`, with offset and uncertainty
linearly interpolated between the selected pre/post exchange midpoints. Both
selected and interpolated uncertainties must be in `0..2.0 ms`, and the
absolute pre/post offset difference must be `<=2.0 ms`; otherwise the round is
setup-invalid. The receipt
retains every exchange, selected offsets, uncertainties, drift, interpolation,
and corrected worker timestamps. Queue verdicts use the conservative upper
bound after adding interpolated uncertainty, not only the point estimate.
Both clock calibrations run only while the action queue is idle, are
carried out-of-band, and are excluded from event/action/sample counts.

The harness must verify that conversion and every ordering invariant before
accepting a run. A calibration-derived negative/cross-context duration or
cross-context ordering whose uncertainty cannot prove the required order is
`SETUP_INVALID_CLOCK_CALIBRATION` and is never clamped into an app pass or app
RED. A same-realm impossible ordering, missing/duplicate/reordered event/action
ID, superseded barrier, invalid/non-prefix supersession link, or orphaned
supersession is a non-retryable behavior/metric RED rather than a missing
sample. The Long Task observer must install and prove support before timing, and
every timed page must remain foreground, focused, and
`visibilityState === "visible"`; an unavailable observer cannot be reported as
zero Long Tasks.

### Derived metrics

- `eventDeliveryMs = eventDeliveredAt - normalizedEventAt`;
- `eventHandlerEnqueueMs = actionEnqueuedAt - eventDeliveredAt`;
- `mainQueueWaitMs = workerSentAt - actionEnqueuedAt`;
- `workerMessageDeliveryMs = correctedWorkerMessageReceivedAt - workerSentAt`;
- `workerPreActionWaitMs = correctedWorkerActionStartedAt - correctedWorkerMessageReceivedAt`;
- `workerDispatchWaitMs = correctedWorkerActionStartedAt - workerSentAt`;
- `preServiceWaitMs = correctedWorkerActionStartedAt - actionEnqueuedAt`;
- `preServiceWaitUpperBoundMs = preServiceWaitMs + interpolatedWorkerClockUncertaintyMs`;
- `workerProcessMs = correctedWorkerFinishedAt - correctedWorkerActionStartedAt`;
- `workerAbiMs`, `workerResponseExtractMs`, `workerJsonParseMs`,
  `workerAdapterTranslateMs`, and `workerPersistenceMs` from their raw spans;
- `workerRoundtripMs = mainResponseReceivedAt - workerSentAt`;
- `mainResponseDispatchMs = responseMappingStartedAt - mainResponseReceivedAt`;
- `responseMappingMs = responseMappingFinishedAt - responseMappingStartedAt`;
- `stateScheduleMs = stateUpdateScheduledAt - responseMappingFinishedAt`;
- `reactCommitMs = stateCommittedAt - stateUpdateScheduledAt`;
- `paintProxyMs = paintObservedAt - stateCommittedAt`;
- `eventToCurrentPaintMs = paintObservedAt - normalizedEventAt`;
- `handlerToCurrentPaintMs = paintObservedAt - eventDeliveredAt`;
- `eventToCoveringPaintMs = coveringPaintAt - normalizedEventAt` for either an
  exact painted state or the earliest exact later state that validly supersedes
  it;
- `eventToTerminalObservationMs = terminalObservedAt - normalizedEventAt` for
  a non-supersedable Backspace/Delete, selection, paging, commit-producing,
  mapped modifier tap/release, or visible cancel action;
- `driverDispatchToCoveringPaintUpperBoundMs = coveringPaintAt - correctedDriverDispatchAt + interpolatedDriverClockUncertaintyMs`;
- `driverDispatchToTerminalUpperBoundMs = terminalObservedAt - correctedDriverDispatchAt + interpolatedDriverClockUncertaintyMs`; and
- `supersessionSequenceLag` plus `supersessionTimeMs` from the original action
  to that covering paint.

For each exact, non-superseded painted action, the harness also computes
`timelineResidualMs` between `eventToCurrentPaintMs` and the sum of event
delivery, handler/enqueue, main queue, worker roundtrip, main response dispatch,
response mapping, state scheduling, React commit, and paint-proxy spans. Its
absolute value must be `<=0.1 ms`; a larger residual is a metric-contract
failure, not unattributed app time.

The binding queue guardrail is conservative
`preServiceWaitUpperBoundMs`, not merely main-queue wait. Its point estimate may
be near zero at the declared 60 ms cadence; that does not weaken its role as the
check that a design did not post every action
immediately and move the same backlog into worker dispatch. The unconditional
FIFO-pressure correctness row below separately proves ordering while a backlog
exists, independent of Rust cost.

`paintObservedAt` remains a double-`requestAnimationFrame` proxy. It is not
described as compositor presentation, photon latency, INP, or physical
keyboard-to-display latency. Continuous frame gaps and Long Tasks are reported
separately; they are not folded into an invented compositor number.

### Current-state and supersession semantics

The existing latency test expects every prefix to paint. That contract would
forbid a potentially valid presentation optimization, so WEB-06 separates
engine processing from presentation:

- every declared event must have exactly its frozen zero/one/many-action
  mapping; every mapped action must have exactly one ordered completion;
- a result may paint only if its rendered input and visible candidate state
  fingerprints match its exact sequence;
- an intermediate printable-insertion result may be marked `superseded` only
  by a later processed sequence in the same composition epoch and the same
  deletion-free `supersessionSubRunId`;
- the receipt must link `supersededBySequenceId` and prove the later sequence
  painted the earliest exact covering state. A covering state means that the
  later state's ordered raw composition-input action sequence is a strict
  append-only extension of the superseded state's sequence, all intervening
  actions are supersedable printable insertions, its logical/rendered input is
  the corresponding monotonic prefix-extension, and the later candidate page
  belongs to that same later render. Rendered/preedit normalization alone is
  not a substitute for the raw-action-prefix proof;
- the harness rejects a supersession link that crosses a sub-run boundary, is
  not a prefix-extension, points backward, skips an uncompleted action, or
  resolves through a barrier;
- every painted or superseded key must resolve to its exact covering paint
  within the binding `eventToCoveringPaintMs` deadline, with
  `supersessionSequenceLag <=2`; a chain cannot postpone visual resolution
  until the end of a sustained phrase;
- commits, digit selection, paging, deletion, Backspace/correction boundaries,
  modifier releases, schema/option/deploy actions, persistence effects, and
  errors are never supersedable;
- every deletion or Backspace terminates the current supersession sub-run after
  its own exact outcome; one that changes composition must paint its exact DOM
  fingerprint. No supersession edge may originate at, target, or span the
  correction;
- digit selection, paging, punctuation/commit, modifier release, focus loss,
  Escape/cancel, schema or option change, deploy, persistence/userdb barriers,
  and injected or recovered errors also terminate the supersession sub-run;
  focus loss, Escape/cancel, commit, schema or option change, deploy,
  persistence barrier, and error recovery additionally terminate the current
  composition epoch; and
- the latest result at every declared burst pause and at scenario end must
  paint within the recovery ceiling.

No diagnostic may silently disappear merely because React skipped an
intermediate render.

`processed-no-visual-change` is valid only for a non-barrier action whose frozen
contract does not change visible composition/candidates and whose before/after
DOM digest is identical. Every barrier-class action must produce exactly one of
`painted`, `committed`, `barrier-completed`, or explicit failure and may never
be tagged `superseded`. `barrier-completed` requires the barrier's declared
action receipt plus its listener, unchanged-state confirmation, status,
persistence, or lifecycle effect as applicable. These outcomes cannot be used
for printable keys or other actions expected to change the visible state.

## Exact Browser Workload

### Common environment

- Chromium/Playwright, one test worker, zero retries, one foreground page.
- Desktop viewport `1365x900`, locale `zh-HK`, AI disabled, inspector/debug UI
  at the production default, six-row candidate page, and the exact public-demo
  production build.
- Exactly five valid fresh-profile rounds per binding scenario for both
  baseline and final. At most seven total attempts may be made to obtain those
  five rounds. Every attempt is retained; if five valid rounds are not obtained
  by attempt seven, the scenario closes
  `SETUP_NO_GO_INSUFFICIENT_VALID_ROUNDS` and cannot pass on a smaller sample.
- Assets are fully ready before typing; no schema, manifest, split-part, model,
  or fallback request is permitted during a timed window.
- The binding host is a declared 60 Hz lane. Each round records at least 120
  idle rAF intervals before typing; the idle median must be in `15..18 ms` or
  the setup is invalid. A different refresh lane requires a reviewed plan
  amendment before baseline and cannot dynamically loosen the frame threshold.
- The first declared key is timed and counted. There is no discarded warm-up
  key, phrase, or round.
- Each new rapid or burst valid round contains three exact repetitions of its
  declared phrase in the same fresh profile. After repetitions one and two, the
  harness waits for the exact final paint, sends an exact Escape/cancel reset,
  proves empty composition and queue depth zero, and then begins the next
  repetition; the reset is an action outcome but not a paint-latency sample.
  All three repetitions, including every first key, contribute to the
  per-round distribution. This yields at least 141 samples per rapid/burst
  round, so nearest-rank p99 is no longer the max.
- Browser crash, source/hash mismatch, missing artifact, missing observer, loss
  of foreground visibility/focus, clock-calibration failure, or metric-contract
  failure is setup-invalid. A delivered cadence outside its frozen range is
  never silently accepted. Setup-invalid attempts consume the seven-attempt
  cap. A valid latency, queue, frame, behavior, or peer red is never replaced or
  retried, and the cadence precedence below prevents a co-occurring red from
  being laundered as `NO_VERDICT`.

### Scenario registry

The plan freezes these rows for initial review. `Expected` is the exact number
of binding covering-latency or terminal samples per valid round; a printable
sample is present whether its outcome is `painted` or validly `superseded`. The
receipt separately requires one terminal action outcome for every declared
action. Every Phase-0 placeholder must become an exact integer in the versioned
registry before the RED; a row with `TBD` or a mismatched count is setup-invalid.

| Lane | Schema | Input / interaction | Cadence | Expected | Authority class | Purpose |
| --- | --- | --- | --- | ---: | --- | --- |
| Existing normal guard | `jyut6ping3` | `ngodeigungsijigaahaidoumaaigangeihaaijansougeoi` | 100 ms/key | 47 | `latency-and-page-shape-only` | Preserve WEB03-11 unchanged; not the new smoothness verdict. |
| Rapid Jyutping | `jyut6ping3` | three repetitions of the same 47-key input with two exact resets | 60 ms/key | 141 covering + 2 reset-terminal | `latency-and-page-shape-only` | Primary reported fast-typing reproduction. |
| Rapid long Jyutping | `jyut6ping3` | three repetitions of `taihaajyugwodaahoucoenggegeoizigosingnangwuidimjoeng` with two exact resets | 60 ms/key | 156 covering + 2 reset-terminal | `latency-and-page-shape-only` | Historical long-input/queue owner. |
| Rapid Luna | `luna_pinyin` | three repetitions of `zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong` with two exact resets | 60 ms/key | 177 covering + 2 reset-terminal | `latency-and-page-shape-only` | Cross-schema absolute Yune smoothness row; no peer-fairness claim. |
| Burst Jyutping | `jyut6ping3` | three repetitions of the primary 47-key input with two exact resets | repeating 40/40/40/120 ms gaps | 141 covering + 2 reset-terminal | `latency-and-page-shape-only` | Four-key bursts plus a recovery pause. |
| Burst Luna | `luna_pinyin` | three repetitions of the rapid Luna input with two exact resets | repeating 40/40/40/120 ms gaps | 177 covering + 2 reset-terminal | `latency-and-page-shape-only` | Cross-schema burst/recovery guard. |
| Correction | `jyut6ping3` | Phase 0 freezes an oracle-backed printable sequence, two Backspaces, resumed input, and commit | 60 ms printable gaps | Phase-0 exact | `oracle-exact` or blocked | Prove correction ends a supersession sub-run and commit is never coalesced. |
| Selection/paging | `luna_pinyin` and `jyut6ping3` | Phase 0 freezes fixture-backed page, digit-select, and page-turn inputs | 60 ms printable gaps; immediate action after ready state | Phase-0 exact | `oracle-exact` or blocked | Prove stateful barriers and exact final state. |
| Burst action map | owning schemas | Phase 0 freezes printable input interleaved with Backspace, arrow/page action, and modifier tap/release | repeating 40/40/40/120 ms gaps | Phase-0 exact | `latency-and-page-shape-only` plus exact action map | Unconditional fast non-printable/action-mapping guard. |
| FIFO pressure barriers | owning schemas | Same-task batches guarantee an earlier action is in flight: commit-then-type, digit-select-then-type, page-then-type, and userdb/persistence completion followed by typing | same-task pressure batch; no latency-ratio claim | Phase-0 exact | `contract-exact` or blocked | Unconditional ordering proof independent of Rust cost; interactive terminal outcomes retain `67 ms`, while only userdb/persistence stress completion may use `250 ms`. |
| Learned row | current TypeDuck profile lane | Existing UI-learned row, reload, then rapid composition | 60 ms/key | Phase-0 exact | `contract-exact` or blocked | Preserve userdb ordering, persistence, and queue behavior. |
| Fair peer short | `luna_pinyin` | `ni`, then commit | identical 60 ms Yune/My RIME schedule | 2 covering + 1 commit-terminal; ratio uses exact final `ni` only | `latency-and-page-shape-only`; ratio requires `packageAlignment=PROVED` | Candidate binding peer row after logical-data and observer proof. |
| Peer sustained | `luna_pinyin` | rapid Luna input, then commit | identical 60 ms Yune/My RIME schedule | 59 covering + 1 commit-terminal; exact-prefix peer observations separately counted | `latency-and-page-shape-only`; informational unless its row has `packageAlignment=PROVED` | Sustained product snapshot; unproved logical-data alignment must not be hidden. |
| Extended scheduler barriers (conditional) | owning schemas | Phase 0 freezes option, schema, deploy, persistence, and injected-error barriers beyond the unconditional rows | target action is dispatched while earlier declared work is in flight | Phase-0 exact | `contract-exact` or blocked | Required only if Branch B is selected; proves wider scheduler safety under actual pressure. |

The correction and selection/paging concrete inputs must be bound to existing
external fixtures or freshly captured oracle bytes and reviewed before Phase 1
baseline execution. They may not be chosen after implementation. If no valid
fixture exists, the row remains explicitly blocked rather than using Yune's
baseline output as expected behavior.

Phase 0 resolves every conditional authority cell and exact count. The rapid,
burst, and peer rows remain `latency-and-page-shape-only`; they can prove
latency, state coherence, and frozen page membership/shape, but not equality of
candidate values between engines. A row becomes `oracle-exact` only after its
expected bytes are captured from the correct upstream/TypeDuck authority with
complete provenance; no Yune-generated baseline is promoted into an oracle.
`contract-exact` is limited to externally reviewed event/action cardinality,
ordering, lifecycle, persistence, or UI side-effect predicates and cannot
support a candidate-order/parity claim.
The DOM-event-to-action map for printable keys, digits, Backspace, arrows/page,
punctuation, modifiers, Escape, and every declared barrier is frozen and tested
unconditionally in Phase 0, regardless of which optimization branch is later
selected.

The FIFO-pressure row is valid only when each intended overlap proves
`earlier.workerSentAt <= later.actionEnqueuedAt < earlier.mainResponseReceivedAt`.
The pair is dispatched from one page task with no await between production
events, later queue depth is `>=1`, the earlier engine/barrier effect completes
before the later owning action starts, and the earlier visible terminal effect
is observed before the later visible state applies. Its Phase-0 barrier manifest
names the originating DOM event, ordered mapped action IDs, expected terminal
outcome/effect, and deadline for every subcase. Engine commit/persistence
completion and the subsequent UI userdb refresh are separate actions/outcomes;
one cannot stand in for or hide the other.

### Cadence validity

- Sustained 60 ms lane: every actual driver-dispatch gap must be in
  `48..75 ms`.
- Burst lane: every nominal 40 ms gap must be in `32..50 ms`, and every
  nominal 120 ms recovery gap must be in `96..150 ms`, measured at actual
  driver dispatch.
- The driver preserves phase during normal scheduling and rebases after a late
  host timer; it never emits a short catch-up gap.
- Requested, actual-driver, normalized-event, and delivered-handler gaps are
  all retained. If the driver gap is valid but the DOM/handler delivery is late,
  that delay belongs to the measured browser path and cannot invalidate the
  round.
- A too-short driver gap is `NO_VERDICT_INVALID_CADENCE` for latency/frame
  arithmetic, consumes the attempt cap, and never excuses a behavior/order red.
  A driver gap above the maximum makes the workload easier: if the attempt also
  contains any `>67 ms` latency/terminal sample, frame/Long Task red, or behavior
  red, RED takes precedence and the attempt is not replaceable. Only a
  cadence-invalid attempt with no hard red may be replaced under the cap.
- Every invalid attempt remains in the receipt and is not removed, relabeled as
  engine time, or allowed to drain the queue in a passing workload.

### Statistics

Report each scenario and component per round and across the five exact repeats:

- sample count, median, p95, p99, and max;
- cadence in-range count and delayed-host count;
- processed, painted, superseded, committed, and failed sequence counts;
- maximum and end-of-burst queue depth;
- time from each burst's last key to the exact latest painted state;
- frame-interval median, p95, p99, max, and count at or above `50 ms`; and
- Long Task count, duration, and overlap with continuous interaction windows.

Before any percentile is computed, the round's declared action count and
expected covering-latency/terminal sample counts must exactly match the frozen
registry. A missing first key, extra warm-up, missing barrier outcome, or absent
covering sample is a hard RED, not a smaller denominator.

Verdicts use unrounded high-resolution durations (or exact integer
microseconds). Sort ascending and select index `ceil((n - 1) * p)` without
interpolation; round only display values after the verdict. Every valid round
and the pooled same-scenario distribution must independently pass p95, p99, and
max. The three repetitions produce 141, 156, or 177 samples per rapid/burst
round, so p99 is distinct from max; max remains separately binding by design.
There is no cross-scenario pooling, and a within-one-phrase percentile is not
mislabeled as between-session variance.

## Binding Thresholds

These thresholds are approved and frozen unchanged by the user's 2026-07-19
execution authorization. Baseline results may demonstrate that a threshold is
impossible or badly defined, but the implementation may not begin while any
threshold amendment is under consideration.

### Global current-state max

Every Yune painted or validly superseded printable sample in every new binding
WEB-06 row has both `eventToCoveringPaintMs <=67 ms` and
`driverDispatchToCoveringPaintUpperBoundMs <=67 ms`. This includes rapid,
burst, correction, the printable portions of selection/action-map and FIFO
pressure, learned-state, and the binding short-peer row. Only the unchanged
WEB03 normal guard and rows explicitly classified informational are excluded.
A `72 ms` Yune `ni` sample therefore cannot pass merely because My RIME is
slower or because another rapid row passed. Rapid/burst rows additionally carry
the distribution gates below; human-interactive terminal actions carry their
own global max section. Every new binding row also rejects any `>=50 ms` frame
interval or Long Task intersecting its continuous interaction window.

### Sustained 60 ms lanes

- `eventToCoveringPaintMs`: every valid round and the five-round pool have p95
  `<=50 ms`, p99 `<=67 ms`, and max `<=67 ms`. The larger per-round sample
  count keeps p99 statistically distinct from max, while the separate strict
  max preserves the product rule that any valid `72 ms` current-state sample is
  RED. A host-caused valid hitch remains user-visible and is not retried away.
- Every `driverDispatchToCoveringPaintUpperBoundMs` sample is also `<=67 ms`.
  This is a delivery-integrity max guard: it prevents automation dispatch-to-DOM
  delay from occurring before the event timestamp and escaping the verdict. It
  is not represented as physical-keyboard latency.
- `preServiceWaitUpperBoundMs`: p95 `<=10 ms`, max `<=30 ms`; point-estimate
  `preServiceWaitMs`, main-queue, worker-dispatch, offset, and uncertainty remain
  separately reported. This is an ownership/backlog guardrail, not the
  user-visible latency endpoint.
- `supersessionSequenceLag <=2` for every superseded key; the covering-paint
  deadline above remains binding regardless of sequence lag.
- no frame interval intersecting a continuous interaction window may reach
  `50 ms`; frame p99 must be `<=35.4 ms` on the fixed 60 Hz binding lane.
- zero Long Task entries with duration `>=50 ms` may intersect a continuous
  interaction window. Entries wholly outside those windows are retained and
  compared with the same-duration idle control; they are not silently counted
  as input jank or silently discarded.
- zero missing, duplicate, reordered, or silently dropped event/action
  sequences.

### Human-interactive barrier lanes

- Every Backspace/Delete that changes composition, digit selection, page turn,
  commit-producing key, mapped modifier tap/release, and visible Escape/cancel
  state has `eventToTerminalObservationMs <=67 ms` per action; this is a
  max-only rule and cannot be hidden by a percentile.
- Every corresponding `driverDispatchToTerminalUpperBoundMs` is also
  `<=67 ms`.
- The FIFO-pressure row's `250 ms` ceiling applies only to non-interactive
  userdb/persistence stress completion. Its commit, selection, paging, and
  correction actions retain the `67 ms` terminal-observation deadline.
- Every such action has one exact non-superseded terminal outcome and its
  interaction interval participates in the continuous frame/Long Task gate.

### Burst lanes

- every key, painted or superseded, meets the same per-round and pooled
  p95/p99/max ceilings as sustained input;
- every latest state at a declared 120 ms pause paints within `67 ms` of the
  burst's final key;
- the queue returns to depth zero before the next burst begins;
- `preServiceWaitUpperBoundMs` retains the sustained p95/max ceilings;
- every intermediate result is either painted exactly or linked to a later
  exact painted sequence; and
- final scenario state paints within `67 ms` of the final key.

### Existing and peer guards

- WEB03-11 retains its existing thresholds and exact 8-scenario/186-key plus
  47-key contracts. WEB-06 does not weaken or rewrite them.
- Final normal-cadence, startup, WASM memory, resource payload, and behavior
  values must not regress outside their existing gates; WEB-06 claims no win
  in those dimensions unless separately measured and reviewed.
- The short `ni` Luna peer lane is binding for method validity and exact ratio
  publication only after its observer and input-relevant-data equivalence gates
  pass. Yune must first pass the absolute thresholds above. A claim that Yune
  matches or beats My RIME additionally requires the short-row Yune/peer p95
  ratio to be `<=1.00x`; a value above `1.00x` remains visible even if the
  absolute Yune gate passes. The sustained row has no binding threshold; while
  `packageAlignment=DATA_CONFOUNDED`, it remains informational and cannot
  support a fair/matches/beats claim. If independently `PROVED`, its ratio may
  be published only as non-binding evidence.

### Fair peer endpoint

The peer timer is not whole-string wall time from the first key; that would
mostly measure the fixed 60 ms typing schedule. A page-level observer installed
before typing records the same external endpoint for both apps. Each app must
emit or expose a coherent per-render observation containing composition,
candidate collection, and commit sequence/version; reading unrelated matches
from `document.body.innerText` is not coherent enough for a binding row. The
same external observer algorithm constructs an atomic DOM tuple in both apps;
app-specific selectors are frozen in a reviewed selector manifest. A Yune
internal sequence token may cross-check that tuple but cannot define an endpoint
that My RIME lacks. The observer records a revision immediately before each
event; an accepted candidate tuple must be a strictly later revision, contain
exactly one visible candidate surface, and remain byte-identical through both
rAF checks.

1. timer start is the normalized DOM event time of the declared measured key;
   the binding short ratio measures only the final `i -> ni` state, while a
   sustained informational summary may use per-prefix observations only when
   every compared prefix has an exact coherent state in both apps—covering a
   missing prefix with a later state is not endpoint-equivalent;
2. timer stop is the earliest observed rendered state whose composition equals
   the exact prefix and whose visible candidate collection belongs to that same
   render commit, followed by the reviewed double-rAF observation; the harness
   re-reads and byte-compares the same composition/candidate DOM digest after
   the second rAF before accepting the stop;
3. the final-prefix row additionally requires the exact complete composition
   and the frozen candidate membership/page-shape predicate for that app; it
   does not require Yune's candidate value to equal My RIME's candidate value;
4. the commit event is dispatched only after that coherent final-prefix state;
   commit timing starts at that commit DOM event's normalized timestamp and
   stops when the exact committed text is present at the declared target and the
   visible composition surface is absent. Yune additionally records exact
   textarea value/caret and cleared composition. For pinned My RIME, the
   source-seeded editable selector is `#container textarea` (the `container` id
   belongs to the Naive UI wrapper, not the textarea); the reviewed selector
   manifest must revalidate that selector against the built DOM, then prove the
   exact committed value/selection plus no visible composition/candidate
   popover. Hidden Vue preedit/menu refs need not be empty and cannot satisfy or
   defeat the visible completion predicate; and
5. page size, options, comments posture, schema and dictionary package bytes,
   readiness, viewport, cache policy, and cadence are frozen and recorded.

Each peer row carries `packageAlignment: PROVED | DATA_CONFOUNDED`. `PROVED`
requires a reviewed peer-data manifest showing identical logical Luna inputs:
fully resolved schema/includes/patches; `luna_pinyin.dict.yaml` and every import;
the preset-vocabulary/`essay.txt` corpus; any sentence/poet or `.gram` model,
recording explicit `none` when absent; speller algebra; filters/options; page
size/comments posture; and fresh empty userdb state. Record repository, commit,
and SHA-256 for every logical input plus both engines' compiled table, prism,
reverse, data/model, and runtime hashes and reproducible build commands.
Engine-specific compiled bytes need not be identical, but their logical source
inputs and effective configuration must be. A shared schema id/package family,
matching candidate, filename, file size, or network-resource list does not prove
alignment. A negative control that changes or omits the `essay.txt` identity
must make the verdict parser refuse a binding ratio.

The retained network receipt demonstrates different transport/package shapes,
but it does not by itself prove that My RIME omitted essay semantics: the
[pinned My RIME build script](https://github.com/LibreService/my_rime/blob/c73ea172d28f07031ba87a1d71c4d2e1c8ba82a3/scripts/install_schemas.ts#L92)
installs `essay` before deployment, and plain Luna uses
`essay.txt` preset vocabulary on the null-grammar path under
[D-30](../../decisions.md#upstream-sentence--language-model-poet-project-wide-d-30).
Because the exact logical-input identity remains unproved, the 59-key sustained
row starts `DATA_CONFOUNDED` and informational. The short `ni` row is not
exempt—`ni` selection uses essay weights under
[UPSTREAM-BEHAVIOR-02](../../requirements.md#m12-upstream-oracle-and-behavioral-parity-requirements)—but
it remains the intended binding row once its own alignment and same-render
observer are `PROVED`.

A 2026-07-18 read-only audit of a clean local checkout verified exact My RIME
HEAD `c73ea172d28f07031ba87a1d71c4d2e1c8ba82a3`, package `0.10.9`, license
`AGPL-3.0-or-later`, its recorded submodule pins, and the `essay` install call.
That closes only the source-text assumption. The checkout contains no generated
schema manifests, `public/ime` packages, runtime assets, or `dist` artifact;
those paths and `pnpm-lock.yaml` are ignored. The prerequisite/schema downloader
calls also do not state immutable repository refs at their callsites, while the
historical CI installs an unversioned pnpm and activates Emscripten `latest`.
Therefore a commit pin alone is not an artifact-reproducibility proof. Before a
peer build can be binding, Phase 0 must freeze and hash the Node/pnpm dependency
lock and package integrities, exact compiler/Emscripten toolchain, every recipe's
resolved repository commit and logical bytes, all generated manifests, and the
complete served artifact. A command that can re-resolve a branch head or
`latest` is not a reproducible build command.

If My RIME cannot expose that coherent state, or any logical input is missing,
unknown, or different, that row remains a product-level informational snapshot
and no fair/matches/beats ratio may be formed. Alignment may build a separately
labelled pinned My RIME artifact from the same logical inputs; WEB-06 may not
strip, disable, replace, or otherwise weaken Yune's frozen schema assets to
manufacture alignment.

## Phase 0 — Review And Measurement Correction

No performance implementation starts in this phase.

- [x] Obtain explicit user authorization and record whether M62 is parked or
  WEB-06 is queued. The user parked M62 and authorized WEB-06 on 2026-07-19.
- [x] Review the scenario registry, supersession semantics, frozen
  thresholds, host/environment rules, and evidence-retention budget.
- [ ] Record `WEB06_PRODUCT_SHA`, land/review the instrumentation-only commit,
  record `WEB06_BASE_SHA`, and freeze the minimal-probe/full-collector split,
  numeric observer-overhead ceilings, exact registry counts, clock-calibration
  algorithm, and metric semantics before accepting a RED. Behavior-neutral
  runtime request/response spans are allowed only under the Phase-0 carve-out.
- [ ] Obtain exactly five valid counterbalanced PRODUCT/no-probe, BASE/minimal,
  and BASE/full triplets within seven attempts over the pre-optimization Rapid
  long Jyutping row with the common
  external sentinel and no mid-round toggle; prove exact counts, both absolute
  `<=1.0/2.0/4.0 ms` comparison ceilings, no common-surface or minimal/full
  internal verdict disagreement, no in-window `>=50 ms` instrumentation Long
  Task, no `>=5 ms` main/worker collector callback, and the external sentinel's
  `<0.5 ms` callback / `<1.0 ms` per-event self-time ceilings.
- [ ] Prove collection-off/minimal/full equivalence for public action/result and
  error shapes, native binding call count/order, decoded JSON, and pointer-free
  traces. Every nonzero response pointer is freed exactly once on every
  success/failure path, zero is never freed, spans do not overlap, and a
  collector exception fails measurement without altering the app result/error.
- [ ] Freeze and test the complete DOM-event-to-action map unconditionally,
  including digits versus `processKey`, Backspace/Delete, arrow/page,
  punctuation/commit, modifier release, Escape/cancel, all barriers, every
  `keyof Actions`, event/action classifications, and in-window background
  actions.
- [ ] Add supersession contract tests: append-only `n -> ni` may link; `ni ->
  na` across Backspace and any printable-to-Backspace link hard-fail; and the
  real correction row proves exact pre-delete, each deletion, resumed-input,
  and commit fingerprints.
- [ ] Correct the Yune comparator wait condition to require the exact complete
  input's diagnostic and exact rendered candidate state, and replace My RIME's
  body-`innerText` heuristic with a coherent same-render composition/candidate
  observer.
- [ ] Add a negative regression proving `ni` cannot stop on the earlier `n`
  diagnostic; verify every per-prefix and final timed engine/DOM fingerprint
  agrees, and prove commit timing cannot start from a premature candidate.
- [ ] Preserve the existing comparator packet unchanged and write a dated
  correction for the dashboard/history claim.
- [ ] Build and seal a newly labelled pinned My RIME peer artifact locally. The
  clean planning checkout already verifies source commit
  `c73ea172d28f07031ba87a1d71c4d2e1c8ba82a3`, package `0.10.9`, and source
  license, but it has no generated artifact or committed dependency lock. Phase
  0 must freeze the exact selected dependency/toolchain resolution, recipe
  commits, logical schema inputs, build command, generated manifests, runtime
  bytes, and complete artifact hash. Do not call it a reproduction of the
  historical live Vercel bytes unless byte identity is independently proved.
- [ ] Produce and independently verify the row-level peer-data manifest and
  `packageAlignment` verdict. A negative control that changes/omits the essay
  identity must refuse a binding ratio. Build alignment on the pinned peer;
  never alter Yune's frozen schema/data assets.
- [ ] Freeze the fixture-backed correction, selection, paging, commit, and
  learned-row inputs, plus every Phase-0 exact action/sample count, before
  measurement. Explicitly assert that the first key is included and no warm-up
  exclusion exists.

Phase 0 is green only when the endpoint regression fails on the old method and
passes on the corrected method for both apps, My RIME identity and row-level
alignment are reproducible or explicitly blocked, observer overhead is inside
its numeric ceiling, supersession/action-map negatives pass, and the
metric/threshold contract has independent approval.

## Phase 1 — Source-Current RED Baseline And Attribution

- [ ] Start from the clean instrumentation baseline `WEB06_BASE_SHA`; build and
  seal the exact public artifact once.
- [ ] Capture exactly five valid rounds per binding scenario without
  production-path changes and within the seven-attempt cap; exhaustion is a
  setup no-go, never a pass over fewer rounds.
- [ ] Recompute every derived value from raw timestamps and byte-compare the
  independent summary with the test runner's summary.
- [ ] Prove Long Task support/overlap, observer installation,
  foreground/focus state, 60 Hz idle-frame bounds, main/worker clock offset and
  residual, driver/page clock offset and delivery guard, composition
  epochs/supersession sub-runs, exact action/sample
  counts, engine/DOM fingerprints, per-key covering-paint resolution, and
  main/worker/combined queue spans before accepting any row.
- [ ] Run the unconditional Burst action map and FIFO pressure-barrier rows,
  proving exact action order and one allowed non-superseded terminal outcome per
  barrier before owner selection.
- [ ] Capture one focused performance trace for each distinct binding owner;
  traces are diagnostic and do not replace the repeated browser gate.
- [ ] Run production-default UI plus focused negative controls that isolate,
  one at a time, diagnostic dataset writes, userdb/inspector rendering, rich
  candidate mapping, React parent updates, and FIFO queue wait.
- [ ] Quantify each control against the same exact scenario and artifact. Do
  not combine controls before attribution.
- [ ] Freeze one owner and expected movement before Phase 2.

Owner selection uses this precedence:

1. main-thread presentation/diagnostic owner if it explains a binding
   current-state paint or frame miss;
2. JavaScript action scheduling if queue wait remains a binding miss after the
   presentation control;
3. TypeScript response decoding/mapping if it is independently dominant; or
4. measured no-go / separate engine proposal if the owner lies in Rust, schema
   assets, or a new web export.

The plan does not pre-authorize all plausible levers merely because they are
listed.

## Phase 2 — Bounded Owner Reduction

### Branch A: presentation/diagnostic owner

Allowed only when Phase 1 selects this owner. Candidate levers include:

- replace per-key DOM dataset parse/stringify round-trips with an in-memory
  bounded ring exposed through the existing debug seam, retaining only minimal
  production DOM markers and the invariant minimal paint probe;
- avoid inspector/candidate-debug state updates when the owning surface is not
  active;
- isolate, memoize, or virtualize the userdb and inspector/control surfaces
  without removing data or changing their visible behavior;
- keep composition/candidate state local enough that unrelated panels do not
  reconcile on every key;
- cache or defer rich comment parsing only when candidate bytes and visible
  output stay exact; and
- change the visible `Input latency` label to its truthful endpoint or display
  the reviewed current-state metric.

The browser gate must run with the same diagnostics needed for release
verification. A benchmark-only fast path that is absent from production is not
an accepted reduction.

### Branch B: action-queue owner

Allowed only when Phase 1 proves queue ownership after Branch A controls. The
action map, base supersession rules, Burst action-map row, and FIFO-pressure row
are already unconditional; this branch adds the wider scheduler design and
conditional barrier matrix.

- Add explicit sequence IDs and stateful barrier classification.
- Preserve every event's frozen action cardinality and route/complete every
  mapped action at its owning layer exactly once and in order.
- Preserve worker/session ordering across key, Backspace, selection, paging,
  option, schema, deploy, persistence, and error boundaries.
- Coalesce only stale presentation application after the engine action has
  completed; never drop raw key events.
- Keep commits and side effects observable exactly once.
- Run every extended conditional scheduler-barrier scenario with the target
  action queued or dispatched while earlier declared work remains in flight,
  including commit/persistence/userdb refresh followed immediately by new
  typing.
- Keep AI default-off; if enabled in a diagnostic lane, it is lower priority
  and cancellable without delaying classic input.

A worker batch or multi-in-flight design is not assumed safe. It must prove
the exact order/barrier invariants above before it can be selected.

### Branch C: TypeScript runtime mapping owner

Allowed only when decoding/mapping is independently dominant and can be fixed
without a Rust or export change. Update the runtime's owning tests and preserve
pointer/free, request/response/JSON shape, ordering, and error semantics. Phase
0 timing fields do not by themselves prove this owner or authorize this branch.
Discovery of a Rust/export owner stops this branch and WEB-06 implementation
pending separate approval.

## Phase 3 — Compatibility And Final Measurement

- [ ] Run focused unit/type/build checks for every touched TypeScript owner.
- [ ] Run the exact correction, commit, selection, paging, learned-row/reload,
  Burst action-map, FIFO-pressure, supersession-negative, and schema behavior
  guards.
- [ ] Build and certify the final public archive exactly once through the normal
  release entrypoint, seal it, record its full archive/file manifest and hash,
  and serve the extracted sealed bytes for every remaining local gate. The
  timestamped `build-info.json` is part of identity; source/toolchain equivalence
  cannot substitute for byte identity.
- [ ] Run exactly five valid final rounds per binding scenario within the
  seven-attempt cap, using the same host, toolchain, artifact procedure, cache
  regime, scenario order, metric configuration, and parser as baseline.
- [ ] Prove every threshold and behavior requirement from raw receipts.
- [ ] Run unchanged WEB03-11 against the exact final artifact.
- [ ] If `packages/yune-web-runtime` changed, run its tests and build.
- [ ] Confirm `crates/`, schema assets, ABI tables, and
  `scripts/yune-web-exports.txt` are unchanged.

Do not run broad Rust gates for a browser-only diff. Any Rust diff is a scope
violation, not a reason to expand the WEB-06 gate list after implementation.

## Phase 4 — Pinned Peer And Deployment Proof

- [ ] Reuse—do not rebuild—the exact Phase-3 measured/certified Yune archive and
  serve it with the pinned My RIME artifact under the same browser/host
  conditions.
- [ ] Run exactly five valid fresh-profile Luna short and sustained peer rounds,
  within seven attempts per row, with identical inputs, cadence, readiness,
  endpoint, viewport, and cache policy.
- [ ] Record absolute values and each row's `packageAlignment` verdict. Publish
  an exact Yune/peer ratio only for a `PROVED` row; the short row is the binding
  peer verdict, while a non-`PROVED` sustained row remains an explicitly
  data-confounded product snapshot. Jyutping remains separately informational.
- [ ] Upload the exact Phase-3 archive hash to one preview, run the source-pinned
  normal/rapid canary once, then stop and request explicit user approval at the
  production-promotion gate. Promote those identical bytes only after green and
  only if that approval is received. If the deployment path cannot accept the
  measured archive without rebuilding, deployment identity is blocked and the
  milestone cannot claim Full success.
- [ ] After explicit production-promotion approval, verify production
  source/toolchain/app/worker/WASM/schema hashes. Do not rerun a timer-sensitive
  binding benchmark on a shared production alias.

## Anti-Gaming And Non-Goals

WEB-06 may not win by:

- slowing or smoothing the test's requested input schedule;
- reducing candidate page size, comments, dictionaries, schemas, userdb data,
  or visible product controls;
- precomputing benchmark candidates or special-casing inputs;
- dropping, reordering, or merging raw key events;
- applying an old candidate page to a newer composition;
- moving backlog from the main queue into worker dispatch while reporting only
  the now-smaller main-queue wait;
- omitting inconvenient valid samples, retrying measured reds, or averaging a
  spike out through unrelated easy scenarios;
- relabelling a co-occurring latency/frame/Long-Task/behavior red as invalid
  cadence, passing fewer than five valid rounds, or dropping the first key;
- changing timestamps, rounding before verdict calculation, clamping metrics,
  or describing double-rAF as compositor presentation;
- enabling a benchmark-only fast path that production does not use;
- rebuilding after the binding matrix or treating a same-source build with a
  different timestamp/file hash as the measured artifact;
- using live My RIME bytes as if they were pinned to an inspected source,
  calling matching output/package names data alignment, using an `innerText`
  heuristic as a coherent endpoint, or weakening Yune data to match the peer;
- projecting native, WASM-memory, startup, payload, or peer results across
  evidence lanes.

Explicit non-goals:

- native engine latency or memory optimization;
- WASM heap or encoded-resource reduction;
- candidate ranking changes;
- a new C ABI or `yune_web_*` export;
- AI-provider/product work;
- mobile Safari, Firefox, physical-device, or physical keyboard-to-display
  claims; and
- proving actual compositor/GPU presentation timing.

## Evidence Layout

Raw JSON, traces, browser profiles, videos, screenshots, and full logs remain
outside Git under a source/run-attempt-named root. The compact tracked packet
belongs under:

```text
docs/reports/evidence/web06-rapid-typing-smoothness/
  README.md
  baseline-summary.csv
  final-summary.csv
  peer-summary.csv
  peer-data-manifest.json
  observer-overhead.csv
  clock-calibration-summary.csv
  threshold-verdict.csv
  behavior-verdict.csv
  source-artifact-manifest.json
  evidence-manifest.json
  review-spec.md
  review-quality.md
```

Each compact row identifies source, artifact manifest hash, metric-contract
version, scenario-registry version, round/sample count, environment ID, and raw
packet checksum. Raw paths and logs must not leak local absolute paths, secrets,
browser profiles, or user data.

## Success, Partial, And No-Go Dispositions

### Full success

All nine WEB06 requirements have a reviewed disposition: the eight
unconditional requirements pass, and WEB06-SCHED-01 either passes its Branch B
gate or is `N/A — precondition false` with Phase 1 evidence. Every absolute
threshold is green, exact behavior remains green, the pinned short Luna peer
row has `packageAlignment=PROVED` and a coherent observer, WEB03-11 stays green,
and identical-byte preview/production verification completes. By explicit
scope choice, the sustained peer row remains a labelled informational snapshot
unless its independent alignment becomes `PROVED`; it is not allowed to support
a fair ratio otherwise and does not block full success. Any published peer ratio
above `1.00x` remains visible and prevents a “matches/beats My RIME” claim even
though the absolute Yune smoothness result may pass.

### Partial

Allowed only with explicit user disposition:

- absolute Yune smoothness and compatibility pass, but the short pinned My RIME
  row cannot be reproduced, observed coherently, or marked
  `packageAlignment=PROVED`;
- a material owner reduction lands but one frozen smoothness ceiling remains
  red; or
- local exact-artifact gates pass but deployment identity/canary is blocked.

The closeout names the remaining red and does not use “smoothness fixed” for a
threshold miss.

### Setup no-go

If any binding scenario fails to produce exactly five valid rounds within seven
attempts, or the required observer/clock/60 Hz environment cannot be validated,
close that lane `SETUP_NO_GO` with no product pass/fail verdict. If only the peer
lane is setup-blocked after absolute Yune gates pass, use the Partial disposition
above; never infer a ratio from fewer rounds or an unproved package.

### Measured no-go

Close no-go when the valid RED is reproduced but no safe in-scope owner clears
it, or when the only causal owner requires Rust/ABI/schema changes outside this
plan. Retain independently useful behavior-neutral instrumentation only after
review; do not retain test-only product overhead without a named ongoing owner.

## Review Findings And Authorization Gates

The initial 2026-07-18 independent specification review returned **approve after
specified changes** with no P0, two P1 findings, and bounded P2/P3 precision
items. This revision accepts and dispositions them as follows. The 2026-07-18
closure re-review then returned **PASS — no remaining plan-level blocker**. That
PASS closes the specification gate. The user separately supplied execution
authorization on 2026-07-19.

| Review finding | Disposition in this revision |
| --- | --- |
| P1 covering state may cross deletion | Added raw-action append-only supersession sub-runs, complete boundary list, hard crossing negatives, and exact correction-state proof. |
| P1 peer data alignment undefined | Added row-level `PROVED/DATA_CONFOUNDED` logical-input manifests, compiled hashes, negative control, frozen-Yune rule, and conditional ratios. The review's stronger “My RIME has no essay semantics” premise was corrected against the pinned build script and D-30. |
| Phase-0 runtime instrumentation boundary | Defined a conditional, post-user-authorization Phase-0 carve-out for behavior-neutral request/response raw spans and a private worker metadata envelope while freezing public/native shapes. |
| Observer overhead undefined | Added sealed PRODUCT/no-probe, BASE/minimal, and BASE/full modes with one external sentinel; froze counterbalanced triplets, absolute `1/2/4 ms` median/p95/max deltas, identical verdicts, callback ceiling, and Long Task guard. |
| Cross-context clock skew | Added explicit driver/page and main/worker pre/post ping/echo formulas, interpolation, uncertainty bounds, conservative queue verdict, dispatch-to-paint/terminal guard, and fail-closed ordering. |
| Per-round p99 equals max | Each rapid/burst round now has three exact repetitions (`n=141/156/177`); p99 is distinct and strict per-round/pooled max `<=67 ms` keeps every valid `72 ms` sample RED. |
| Barrier/supersession outcome gaps | Added forbidden superseded-barrier invariant, exact terminal outcomes, `eventToTerminalObservationMs`, and unconditional interactive deadlines. |
| Barrier pressure/action-map coverage conditional | Added unconditional Burst action-map and FIFO-pressure rows; only the extended lifecycle matrix remains Branch-B-conditional. |
| My RIME endpoint incoherent | Required the same atomic external DOM tuple algorithm, reviewed selectors, strict post-event double-rAF digest stability, and exact visible-surface commit predicate. |
| Invalid-round exhaustion | Requires exactly five valid rounds within seven retained attempts; exhaustion is setup no-go. |
| Frame/Long Task, authority, warm-up, and counts | Fixed the 60 Hz calibration bounds, continuous interaction windows, authority column, first-key rule, and exact sample/action counts. |
| Follow-up instrumentation audit | Added total event/action classification, private wire identity, raw-to-presentation-to-DOM fingerprints, React sequence propagation, first-statement transport spans, pointer/free equivalence, strict post-event peer revisions, visible My RIME commit semantics, and out-of-window collector export. |

The same-day local My RIME source audit independently confirmed the pinned
`essay` install call and corrected the source-seeded editable selector to
`#container textarea`. Because generated schema/runtime assets and the dependency
lock are absent, this audit does not complete the Phase-0 peer-artifact task or
change any row from `DATA_CONFOUNDED` to `PROVED`.

The original open questions are now dispositioned:

1. The `50/67 ms` paint thresholds are frozen execution targets under the
   user's explicit authorization; the strict-max rule and larger per-round
   sample are frozen before production.
   Queue `10/30 ms` applies to the uncertainty upper bound. If Phase 1 proves a
   threshold impossible, implementation stays stopped for a reviewed amendment
   or measured no-go; it is not tuned after seeing a candidate.
2. The binding burst pattern remains `40/40/40/120 ms`. A captured human cadence
   may be retained only as clearly non-binding evidence.
3. Phase 0 first attempts pinned My RIME `c73ea17...`. A newer commit may replace
   it only through an explicit reviewed pin and full row-level data manifest;
   otherwise the short peer requirement is Partial, never satisfied by live
   bytes.
4. Supersession-run boundaries now include Delete/Backspace, selection/digit,
   paging, punctuation/commit, modifier release, focus loss, Escape/cancel,
   persistence/userdb, schema, option, deploy, and error boundaries. Review may
   add a boundary but may not remove one after the RED.
5. A TypeScript-only runtime mapping fix remains inside WEB-06 with owning tests
   and frozen public/native shapes. Any Rust/export/ABI owner triggers a separate
   proposal.
6. The source-pinned preview runs one exact normal/rapid canary; the complete
   timing matrix remains binding locally to avoid shared-host noise.

Two independent reviews are the authorization standard. Current status:

1. **Specification closure confirmation — PASS (2026-07-18):** endpoint
   equivalence, scenario completeness, threshold meaning, oracle
   non-circularity, peer reproducibility, source/artifact identity, and no
   loophole that lets a valid `72 ms` hitch pass.
2. **Quality/safety review — PASS (2026-07-18):** event/action ordering,
   supersession/barrier safety, React/diagnostic ownership, TypeScript runtime
   boundary, ABI/export isolation, evidence retention, deployment integration,
   test flake risk, corrected My RIME selectors, and peer-artifact
   reproducibility gates.

The user explicitly parked M62, chose WEB-06 as the next active milestone, and
authorized execution on 2026-07-19.
