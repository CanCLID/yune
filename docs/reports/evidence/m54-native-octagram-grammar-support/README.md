# M54 Native Octagram Grammar Support Evidence

> **Status:** Complete - **Milestone:** M54 (native octagram grammar support) - **Updated:** 2026-07-01 - **Type:** evidence index

This folder records the oracle-first evidence for M54. M54 adds native octagram-compatible grammar support as a Yune `Grammar` provider; it does not add the librime C++ plugin ABI, dynamic plugin loading, Lua/predict/proto, AI ranking, frontend/platform work, or public performance claims.

## Contents

- `task-0-target-selection.md` - source/data pins, license verification, schema patches, vendoring decisions, and no-go review.
- `external-pins.json` - machine-readable source/data pin manifest.
- `phase-0-oracle/` - oracle capture notes and output provenance.
- `synthetic-rear-boundary-oracle.md` - executable follow-up oracle for the
  empty-context rear-boundary regression.
- `clean-room-design.md` - implementation boundary and behavior design.
- `phase-3-yune-core-verification.md` - human-readable Yune-vs-oracle verification.
- `phase-3-yune-core-verification.json` - machine-readable committed verification summary for the lotem and RIME-LMDG lanes.
- `final-gates.md` - closeout test, regression, ABI, and scope gates.
