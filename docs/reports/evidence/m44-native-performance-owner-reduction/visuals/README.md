# M44 Visual Evidence

These SVGs summarize the final native/profile M44 evidence only. They do not
claim browser, frontend, packaging, deployment, public-demo, or broad
product-delivery speed.

- `m44-native-latency-gates.svg` uses
  `../final-native-benchmark/summary.csv` for final Yune-to-librime ratios and
  row-specific M44 ratio guards.
- `m44-trackb-owner-reduction.svg` uses
  `../phase-0-native-benchmark/summary.csv` and
  `../final-native-benchmark/summary.csv` for Track B phase-0-to-final latency
  reductions; exact lookup counts are the selected counters recorded in the
  final M44 evidence.
- `m44-memory-gates.svg` uses
  `../final-native-benchmark/final-gates.md` and
  `../final-native-benchmark/memory-owner-profile.csv` for the Track A peak
  memory target, phase-0/final peak working-set values, and remaining retained
  owner context.
- `m44-root-cause-gains-and-blockers.svg` uses the M44 phase-0/final summaries,
  final gates, and selected M37 owner counters to show the causal gain map and
  the remaining `ni`/memory blockers.
