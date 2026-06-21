# M19 Reference — TypeDuck Profile ABI Surface

> **Status:** Reference · **Created:** 2026-06-21 · **Scope:** named, opt-in
> TypeDuck-profile ABI surface only.

Yune's default `rime_get_api()` remains the upstream `rime/librime 1.17.0`
`RimeApi` table. M19 does not add fork-only fields to that default table and
does not widen `RimeCandidate`.

## Accessor

The TypeDuck-profile surface is exposed through:

```c
rime_get_typeduck_profile_api()
```

This accessor returns an opt-in table whose first bytes are the upstream Yune
`RimeApi` prefix and whose `data_size` advertises the larger profile table. Code
that calls plain `rime_get_api()` continues to see the upstream-sized table.

## Profile Delta

M19 exposes the fork-only list-append slots implemented in
`crates/yune-rime-api/src/config_api.rs`:

- `config_list_append_bool(RimeConfig*, const char* key, Bool value)`
- `config_list_append_int(RimeConfig*, const char* key, int value)`
- `config_list_append_double(RimeConfig*, const char* key, double value)`
- `config_list_append_string(RimeConfig*, const char* key, const char* value)`

The slot order follows the TypeDuck fork header order for the delta:
`bool`, `int`, `double`, `string`. The implementations create a missing list,
append to an existing list, and reject invalid/non-list targets.

## Out Of Surface

`start_quick` is not exposed by M19. The parked TypeDuck-Windows deployer
requirement that blocked M10 was the list-append function-table access used by
`WeaselDeployer/TypeDuckSettings.cpp`; no current M19 evidence requires
`start_quick`.

M19 also does not resume TypeDuck-Windows packaging, does not produce native
release archives, and does not claim a real TypeDuck-Windows frontend E2E pass.

## Contract Test

`crates/yune-rime-api/tests/typeduck_profile_abi_surface.rs` verifies:

- default `rime_get_api()` remains exactly the upstream-sized `RimeApi`;
- `rime_get_typeduck_profile_api()` is larger and exposes the append slots;
- `config_list_append_string` works through the profile table by creating and
  extending a list.
