# Yune Web Runtime Guide

This package wraps the Emscripten `yune_web_*` API for yune-web.

## Rules

- Keep the wrapper a thin transport layer. It must not implement candidate ranking, AI provider logic, or schema semantics in TypeScript.
- Pair every owned native response with `yune_web_free_response`.
- Preserve key mapping tests when changing keyboard behavior.
- Preserve lifecycle safety: operations after cleanup must fail in TypeScript before reusing a native pointer.
- Do not widen `RimeCandidate`, reorder `RimeApi`, or add exports for UI-only convenience.
- AI provider work belongs only behind `stageAi`; `processKeyboardEvent` must remain classic-first.
