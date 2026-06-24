# M27 Control Classification Before

> **Status:** Captured from pre-M27 source inspection - **Milestone:** M27 (TypeDuck-Web startup/runtime init) - **Updated:** 2026-06-22 - **Type:** evidence

## Pre-M27 Behavior

| Control Class | Examples | Pre-M27 Route |
|---|---|---|
| Deploy-time engine preferences | Auto-completion, Auto-correction, Auto-composition, Input Memory, Combine same-text candidates, Prediction never first, Prediction threshold, Dictionary exclude, schema switch, Cangjie version | React effect used the global async loading wrapper, then `customize`; most engine settings then called `deploy` and schema reselect. |
| Live session options | ASCII mode, Full shape, Simplification, Traditionalization, Extended charset, Disabled, Yune inspector | React effect used the global async loading wrapper, then `setOption`. |
| Browser-only display controls | Display languages, Candidate Menu Layout, Chinese Typeface, Candidate Jyutping, Reverse code display | Browser state/local storage only. |
| AI Candidates | AI Candidates | React effect used the global async loading wrapper, then `customize({ enableAI })`; CandidatePanel separately called `stageAi` for the second-pass local candidate row. |

## M27 Target

`AI Candidates` must not show the page-wide loading indicator and must not deploy or reinitialize the runtime. Deploy-time controls may remain deploy-backed, but the marker sequence must make that boundary explicit.
