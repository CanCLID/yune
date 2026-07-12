# M59 post-fix macOS build and resource provenance audit

## Result

**PASS:** the frozen post-fix evidence is internally reproducible at the source, binary, and selected deployment-resource levels. This was a read-only audit; it did not run a benchmark, build, deployment, or performance fix.

- Yune is exactly `afb7079b71f7f9353845114ff3e310c0a38b9b87` in a pristine detached worktree.
- Both librime checkouts resolve to pinned upstream `33e78140250125871856cdc5b42ddc6a5fcd3cd4`. The pinned source is pristine. The isolated build checkout is tracked-clean and has one explicitly captured untracked cache file, `deps/boost_1_89_0.tar.gz`; its SHA-256 and the expanded Boost include-tree digest are in `build-inputs.csv`.
- Frozen Yune SHA-256: `3dd5a414c68f7884884c5dc172b3f0b088d1f5ae19cb983eb0eeb2f95bc6c710`. Frozen librime SHA-256: `743acf3e3a0b64f94680a2f822b00ae42d35ce1e2ab3c8994441bc305adaf8f6`.
- The current retained target binaries, all five measured run copies, and all five recorded post-run hashes match those frozen hashes exactly.
- Every selected compiled resource and candidate snapshot is byte-stable across the five runs **except** the Track A `luna_pinyin.prism.bin` raw hash. That prism differs only in its four-byte `schema_file_checksum` field at byte offsets 36–39; after normalizing that metadata word, all five files have the same hash. The dictionary checksum, length, and every other byte are stable. Yune, staged Yune, and librime prism files are byte-identical within each run, so both engines measured the same per-run prism payload and metadata. See `prism-schema-checksum-variance.csv`.
- Logical product-path checksums and storage/mapping status are stable across all five runs for `luna_pinyin`, `jyut6ping3`, and `jyut6ping3_scolar`; see `deployment-stability.csv`.
- Accepted measurement interval: `2026-07-11T21:17:44Z` through `2026-07-11T21:30:27Z`. Exact per-run environment and command manifests are hashed in `accepted-evidence-hashes.csv`.

## Build controls and remaining confounds

The artifacts were not built under equivalent compiler pipelines. Yune used Rust release settings `opt-level=3`, LTO enabled, one codegen unit, `panic=abort`, and stripping. librime used Apple Clang C++17 `-O3 -DNDEBUG`, arm64, macOS deployment target 10.15, and the retained link command shows no LTO flag. librime statically links the captured glog, yaml-cpp, LevelDB, MARISA, and OpenCC archives while dynamically depending on libc++/libSystem. These are real build-shape differences and should remain labeled as a possible confound; this audit does not establish that they explain any measured latency.

The accepted Cargo logs record a cached release build (`Finished ... in 0.07s`) rather than the raw rustc/link invocation. `Cargo.toml`, `Cargo.lock`, Cargo's retained crate fingerprint, final Mach-O identity, and dylib hash are captured, but the exact historical rustc command line cannot be reconstructed from the retained logs. librime's exact CMake cache, `compile_commands.json`, link command, static archives, source/submodule commits, and Mach-O identity are retained and hashed.

Yune's dylib embeds an absolute build-tree install name; librime uses `@rpath/librime.1.dylib`. Both were loaded from explicit copied paths by the accepted harness, and all copied hashes match their frozen originals. This is a packaging difference, not evidence of a performance cause.

## Files

- `validation.json` — machine-readable load-bearing checks; all must pass.
- `source-status.csv`, `submodules.csv` — exact commits and clean/cache-dirty classification.
- `binary-manifest.csv`, `macho-metadata.txt` — SHA-256, size, architecture, UUID, load commands, code-sign identity, and platform build metadata.
- `build-inputs.csv`, `cmake-cache-selected.csv`, `librime-link-command.txt`, `yune-release-settings.csv`, `toolchain-current.txt` — compiler, cache, link, and build-input identity.
- `resource-hashes.csv`, `resource-stability.csv`, `prism-schema-checksum-variance.csv` — five-run deployment artifact and source-tree checks, including the isolated four-byte Track A prism metadata variance.
- `deployment-checksums.csv`, `deployment-stability.csv` — normalized product path, logical checksum, storage, and mmap status.
- `accepted-evidence-hashes.csv` — hashes for retained commands/environments/manifests and full per-run evidence-tree digests.
- `output-manifest.csv` — SHA-256 and size of every audit output except itself.

## Interpretation boundary

This control audit rules out variable engine binaries, different pinned commits, and semantic payload drift in the selected compiled resources as explanations for variation within the accepted five-run packet. The raw Track A prism checksum word varies because it records the per-run deployed schema checksum, but the remaining prism bytes and the within-run Yune/librime pair are identical. The audit does **not** distinguish engine implementation cost from compiler/linker effects, thermal scheduling, cache behavior, or macOS platform behavior. That attribution requires the separately designed diagnostic controls; no performance threshold or signed Windows authority is changed here.
