# Report source notes

## Reporting job

- Question: Why is pinned upstream librime materially faster on macOS than in the signed Windows M59 Increment-0 packet, and what should Yune learn from it?
- Audience: technical.
- Decision supported: distinguish platform/build acceleration from Yune engine-design work, then prioritize diagnostic follow-up without changing code or thresholds.
- Scope: exact Yune commit `457751824b8944676dc44912b9ce31ff29d78403`, exact librime commit `33e78140250125871856cdc5b42ddc6a5fcd3cd4`, five signed Windows rounds, five fixed-binary macOS rounds, and bounded external controls.
- Success criterion: an evidence-backed causal decomposition that states what is measured, inferred, still confounded, and actionable.

## Technical-report structure mapping

1. Title: artifact title block.
2. Technical summary: `technical_summary`.
3. Key findings with visual evidence: platform scaling and allocator sections with two native charts.
4. Scope, data, and metric definitions: `scope_definitions`.
5. Methodology: `methodology` plus the paired-counter table.
6. Limitations, uncertainty, and robustness: `robustness_heading` plus build comparison.
7. Recommended next steps: `recommendations`.
8. Further questions: `further_questions`.

## Chart map

### Platform component scaling

- Analytical question: Did macOS change Yune and librime absolute latency by similar amounts across all 17 inputs?
- Takeaway: no; librime's median platform improvement is much larger, so the ratio change is denominator-led.
- Family/type: comparison, grouped horizontal bar.
- Rows: 17 inputs with two same-unit series.
- Fields: input label, Yune macOS-versus-Windows percent, librime macOS-versus-Windows percent, absolute component medians for tooltip context.
- Palette policy: hard two-root cap via a two-series identity palette; signed values retain a visible zero reference and labels rather than red/green semantics.
- Delivery: native chart in `artifact.json`, packaged into `report.html`.

### Allocator sensitivity

- Analytical question: Does disabling the macOS Nano zone affect librime and Yune equally on the long-sentence path?
- Takeaway: no; librime slows 21.5-23.6%, while Yune stays within 0.6% of default.
- Family/type: comparison, grouped vertical bar.
- Rows: two long inputs with two same-unit series.
- Fields: input label, engine-specific percent change versus default, five rounds per engine/input.
- Palette policy: same two-series identity palette and zero reference as the platform chart.
- Delivery: native chart in `artifact.json`, packaged into `report.html`.

## Source and transformation inventory

- `artifact.json`: canonical validated report input with all reviewed snapshot
  rows embedded.
- `report.html`: self-contained portable report output.
- `sql/*.sql`: the nine SQLite queries used to load the reviewed report
  datasets.
- The tracked compact CSV/JSON files preserve platform scaling, allocator/API
  controls, counter parity, candidate shape, build comparison, long-sentence
  ownership, profile summaries, and the aggregate analysis result.
- `sources/librime-poet-source-notes.md`: exact pinned upstream source
  locations supporting the sentence-path comparison.
- Raw five-round evidence, unsummarized sample profiles, binaries, drivers, and
  machine-specific analysis scripts remain external and are intentionally not
  copied into this compact report packet.

## Caveats and omissions

- D-24/D-48 already select pinned librime's Luna page and order. References to
  coordination with M59 concern implementation sequencing only, not permission
  to accept the candidate-page divergence. This diagnostic creates no milestone
  and does not redirect the concurrently active Windows work.
- No causal share is assigned to CPU/ISA, compiler, STL/CRT, plugin configuration, or compiled payload because those factors were not independently controlled.
- `MallocNanoZone=0` is a macOS intervention, not a Windows allocator emulation. The 26.5% and 32.1% gap shares are explicitly inferential.
- The stripped Yune release binary prevented symbol-level sample attribution; exact M37 component timers provide the load-bearing Yune ownership evidence.
- Absolute allocator-control times were cache-state sensitive; only within-control relative changes are used causally.
- The report contains all 17 audit rows. The enhanced reader paginates the card view after 15 rows and exposes all rows in subsequent/fullscreen views.
- Portable packaging passed validation, exact-payload checks, and semantic structural verification. No compatible installed headless Chromium was available, so chart SVG extraction, responsive browser QA, and source-dialog interaction were not run.
