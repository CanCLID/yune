# TypeDuck-Web WI-4 Browser Failures

Date: 2026-06-18

The TypeDuck-Web browser run is no longer blocked by missing WASM/tooling. The
loadable Emscripten artifact initializes in the upstream app, and core
composition through commit works in the real browser. Remaining failures are
behavioral and should be treated as follow-up work, not environment blockers.

Evidence files in this directory:

- `browser-run.log` - PASS/FAIL summary plus final DOM state.
- `browser-console.json` - captured browser console messages from the run.
- `dom-snapshot-candidates.txt` - DOM snapshot showing the candidate panel.
- `persistence-sync.log` - persistence-specific failure evidence.

Current failures:

- Candidate paging: `{Page_Down}` is accepted, but the response remains
  `page: 0`, `isLastPage: true`, with a single `ba echo` candidate and disabled
  paging buttons.
- Candidate deletion: `{Delete}` leaves the same `ba echo` candidate and does
  not mutate composition or delete a candidate.
- Deploy: the app sends `deploy` during the settings path and receives
  `result: false`.
- Persistence sync/reload: browser evidence does not expose
  `syncFromPersistenceBeforeInit` / `syncToPersistenceAfterMutation` markers,
  and persistence survival cannot be proven while deploy fails.
- Dictionary-panel comments: the browser renders the adapter `candidate.comment`
  value (`echo`), but no TypeDuck v1.1.2 oracle dictionary comment bytes appear
  in the browser flow.
- Option toggles: TypeDuck-Web calls `setOption` on load/settings changes; the
  current Yune TypeDuckRuntime wrapper does not implement it, so the app logs
  errors for those calls.
- Upstream DOM shape: React reports invalid nesting in `Candidate.tsx`
  (`tr` inside `button`, `button` inside `tbody`) when candidates render.

Passing browser evidence:

- Initialization reaches the page with `initialized: true`.
- Composition from `b` then `a` returns composing results and visible preedit
  `ba`.
- Candidate list rendering shows `1. ba echo`.
- Candidate selection with `1` commits `ba` into the textarea.
- Backspace mutates composition from `ba` to `b` in the same browser session.
- Customize returns `true` for the settings payload observed during app init.
