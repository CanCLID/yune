# Phase 04: compiled-dictionary-data - Pattern Map

**Mapped:** 2026-04-29
**Files analyzed:** 16
**Analogs found:** 16 / 16

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/yune-core/src/dictionary/compiled.rs` | utility/model | transform + batch | `crates/yune-core/src/dictionary/compiled.rs` | exact-existing |
| `crates/yune-core/src/dictionary/compiled_table.rs` | utility/parser | file-I/O + transform | `crates/yune-core/src/dictionary/compiled.rs` | role-match |
| `crates/yune-core/src/dictionary/compiled_prism.rs` | utility/parser | file-I/O + transform | `crates/yune-core/src/dictionary/compiled.rs` | role-match |
| `crates/yune-core/src/dictionary/compiled_reverse.rs` | utility/parser | file-I/O + transform | `crates/yune-core/src/dictionary/compiled.rs` | role-match |
| `crates/yune-core/src/dictionary/mod.rs` | config/facade | transform | `crates/yune-core/src/dictionary/mod.rs` | exact-existing |
| `crates/yune-core/src/dictionary/source.rs` | model/parser | transform | `crates/yune-core/src/dictionary/source.rs` | exact-existing |
| `crates/yune-core/src/dictionary/encoder.rs` | utility | transform | `crates/yune-core/src/dictionary/encoder.rs` | exact-existing |
| `crates/yune-core/src/translator/mod.rs` | service | request-response + transform | `crates/yune-core/src/translator/mod.rs` | exact-existing |
| `crates/yune-rime-api/src/schema_install.rs` | service/provider | request-response + file-I/O | `crates/yune-rime-api/src/schema_install.rs` | exact-existing |
| `crates/yune-rime-api/src/runtime.rs` | config/service | file-I/O | `crates/yune-rime-api/src/runtime.rs` | exact-existing |
| `crates/yune-rime-api/src/deployment.rs` | service | batch + file-I/O | `crates/yune-rime-api/src/deployment.rs` | exact-existing |
| `crates/yune-rime-api/src/resource_id.rs` | utility/guard | transform | `crates/yune-rime-api/src/resource_id.rs` | exact-existing |
| `crates/yune-core/src/lib.rs` tests | test | transform | `crates/yune-core/src/lib.rs` test module | exact-existing |
| `crates/yune-rime-api/src/tests/schema_selection.rs` | test | request-response + file-I/O | `crates/yune-rime-api/src/tests/schema_selection.rs` | exact-existing |
| `crates/yune-rime-api/src/tests/deployment.rs` | test | batch + file-I/O | `crates/yune-rime-api/src/tests/deployment.rs` | exact-existing |
| `crates/yune-rime-api/src/tests/resource_id.rs` | test | transform | `crates/yune-rime-api/src/tests/resource_id.rs` | role-match |

## Pattern Assignments

### `crates/yune-core/src/dictionary/compiled.rs` (utility/model, transform + batch)

**Analog:** `crates/yune-core/src/dictionary/compiled.rs`

**Core checksum pattern** (lines 0-39):
```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RimeChecksumComputer {
    remainder: u32,
}

impl RimeChecksumComputer {
    const POLYNOMIAL: u32 = 0xedb8_8320;

    #[must_use]
    pub const fn new(initial_remainder: u32) -> Self {
        Self {
            remainder: initial_remainder,
        }
    }

    pub fn process_bytes(&mut self, bytes: impl AsRef<[u8]>) {
        for byte in bytes.as_ref() {
            self.remainder ^= u32::from(*byte);
            for _ in 0..8 {
                if self.remainder & 1 == 1 {
                    self.remainder = (self.remainder >> 1) ^ Self::POLYNOMIAL;
                } else {
                    self.remainder >>= 1;
                }
            }
        }
    }

    #[must_use]
    pub const fn checksum(&self) -> u32 {
        self.remainder ^ 0xffff_ffff
    }
}
```

**Bounded metadata parse pattern** (lines 97-115):
```rust
pub fn parse_rime_table_bin_metadata(
    bytes: impl AsRef<[u8]>,
) -> Result<RimeTableBinMetadata, RimeCompiledMetadataError> {
    let bytes = bytes.as_ref();
    ensure_len(bytes, 68)?;
    let version = parse_rime_format_version(bytes, b"Rime::Table/")?;
    if version < 4.0 - f64::EPSILON {
        return Err(RimeCompiledMetadataError::UnsupportedVersion);
    }
    if read_u32_le(bytes, 44)? == 0 || read_u32_le(bytes, 48)? == 0 {
        return Err(RimeCompiledMetadataError::MissingRequiredSection);
    }

    Ok(RimeTableBinMetadata {
        dict_file_checksum: read_u32_le(bytes, 32)?,
        num_syllables: read_u32_le(bytes, 36)?,
        num_entries: read_u32_le(bytes, 40)?,
        string_table_size: read_u32_le(bytes, 64)?,
    })
}
```

**Error handling and byte bounds pattern** (lines 157-187):
```rust
fn ensure_len(bytes: &[u8], len: usize) -> Result<(), RimeCompiledMetadataError> {
    if bytes.len() < len {
        return Err(RimeCompiledMetadataError::TooShort);
    }
    Ok(())
}

fn read_u32_le(bytes: &[u8], offset: usize) -> Result<u32, RimeCompiledMetadataError> {
    let Some(value) = bytes.get(offset..offset + 4) else {
        return Err(RimeCompiledMetadataError::TooShort);
    };
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}
```

**Rebuild planning pattern** (lines 219-255):
```rust
pub fn rime_dict_rebuild_plan(
    input: RimeDictRebuildInput,
) -> Result<RimeDictRebuildPlan, RimeDictRebuildError> {
    let mut dict_file_checksum = input.source_dict_file_checksum;
    let mut rebuild_table = match input.table_dict_file_checksum {
        Some(table_checksum) if input.source_available => table_checksum != dict_file_checksum,
        Some(table_checksum) => {
            dict_file_checksum = table_checksum;
            false
        }
        None if input.source_available => true,
        None => return Err(RimeDictRebuildError::MissingSourceAndTable),
    };

    let mut rebuild_prism = match input.prism {
        Some(prism) => {
            prism.dict_file_checksum != dict_file_checksum
                || prism.schema_file_checksum != input.schema_file_checksum
        }
        None => true,
    };

    if input.reverse_dict_file_checksum != Some(dict_file_checksum) {
        rebuild_table = true;
    }
    if input.source_available && input.force_rebuild_table {
        rebuild_table = true;
    }
    if input.force_rebuild_prism {
        rebuild_prism = true;
    }

    Ok(RimeDictRebuildPlan {
        dict_file_checksum,
        rebuild_table,
        rebuild_prism,
    })
}
```

**Apply to Phase 4:** Extend this file with public result/error types, checksum/freshness decisions, and parse facade functions. Keep low-level byte helpers private or `pub(crate)` and return typed errors instead of panicking.

---

### `crates/yune-core/src/dictionary/compiled_table.rs` (utility/parser, file-I/O + transform)

**Analog:** `crates/yune-core/src/dictionary/compiled.rs`

**Imports pattern:** New focused parser modules should require no filesystem imports; take `impl AsRef<[u8]>`/`&[u8]` and materialize core dictionary data. If they need shared types, import from sibling modules like `source.rs` imports `super::TableEncoder` (line 0):
```rust
use super::TableEncoder;
```

**Core parser pattern:** Copy the `parse_rime_table_bin_metadata` style above (lines 97-115), then add checked relative-offset/list/array helpers adjacent to `ensure_len`/`read_u32_le` (lines 157-169). Do not transmute mapped structs.

**Error pattern:** Use a `#[derive(Clone, Copy, Debug, Eq, PartialEq)]` error enum like `RimeCompiledMetadataError` (lines 89-95), expanded for table payload cases such as out-of-bounds offsets, unsupported string table/trie sections, invalid counts, and unsupported versions.

**Materialization target:** Return or convert into `TableDictionary`, because runtime candidate ordering already flows through `StaticTableTranslator::from_dictionary` (translator lines 79-106).

---

### `crates/yune-core/src/dictionary/compiled_prism.rs` (utility/parser, file-I/O + transform)

**Analog:** `crates/yune-core/src/dictionary/compiled.rs`

**Core parser pattern** (lines 118-138):
```rust
pub fn parse_rime_prism_bin_metadata(
    bytes: impl AsRef<[u8]>,
) -> Result<RimePrismBinMetadata, RimeCompiledMetadataError> {
    let bytes = bytes.as_ref();
    ensure_len(bytes, 320)?;
    let version = parse_rime_format_version(bytes, b"Rime::Prism/")?;
    if version < 4.0 - f64::EPSILON {
        return Err(RimeCompiledMetadataError::UnsupportedVersion);
    }
    if read_u32_le(bytes, 52)? == 0 {
        return Err(RimeCompiledMetadataError::MissingRequiredSection);
    }

    Ok(RimePrismBinMetadata {
        dict_file_checksum: read_u32_le(bytes, 32)?,
        schema_file_checksum: read_u32_le(bytes, 36)?,
        num_syllables: read_u32_le(bytes, 40)?,
        num_spellings: read_u32_le(bytes, 44)?,
        double_array_size: read_u32_le(bytes, 48)?,
    })
}
```

**Apply to Phase 4:** Keep prism parsing as metadata plus bounded spelling-map/correction/tips descriptors. Treat full Darts traversal as a structured unsupported finding unless fixture-driven support is added. Preserve the same `Result<..., Error>` shape.

---

### `crates/yune-core/src/dictionary/compiled_reverse.rs` (utility/parser, file-I/O + transform)

**Analog:** `crates/yune-core/src/dictionary/compiled.rs` and `crates/yune-core/src/dictionary/source.rs`

**Reverse metadata pattern** (compiled.rs lines 140-155):
```rust
pub fn parse_rime_reverse_bin_metadata(
    bytes: impl AsRef<[u8]>,
) -> Result<RimeReverseBinMetadata, RimeCompiledMetadataError> {
    let bytes = bytes.as_ref();
    ensure_len(bytes, 64)?;
    let version = parse_rime_format_version(bytes, b"Rime::Reverse/")?;
    if !(3.0 - f64::EPSILON..=4.0 + f64::EPSILON).contains(&version) {
        return Err(RimeCompiledMetadataError::UnsupportedVersion);
    }

    Ok(RimeReverseBinMetadata {
        dict_file_checksum: read_u32_le(bytes, 32)?,
        key_trie_size: read_u32_le(bytes, 52)?,
        value_trie_size: read_u32_le(bytes, 60)?,
    })
}
```

**Stem collection target pattern** (source.rs lines 231-249):
```rust
fn collect_rime_table_stems(entries: &[RimeParsedTableEntry]) -> HashMap<String, Vec<String>> {
    let mut stems: HashMap<String, BTreeSet<String>> = HashMap::new();
    for entry in entries {
        let Some(stem) = entry.raw_stem.as_deref().filter(|stem| !stem.is_empty()) else {
            continue;
        };
        if entry.entry.code.is_empty() {
            continue;
        }
        stems
            .entry(entry.entry.text.clone())
            .or_default()
            .insert(stem.to_owned());
    }
    stems
        .into_iter()
        .map(|(text, stems)| (text, stems.into_iter().collect()))
        .collect()
}
```

**Apply to Phase 4:** Expose reverse entries, stem keys, and optional `dict_settings` in a model that can populate `TableDictionary::stems` or reverse lookup comments. Keep MARISA/key-value decoding bounded and return structured unsupported findings when unreadable.

---

### `crates/yune-core/src/dictionary/source.rs` (model/parser, transform)

**Analog:** `crates/yune-core/src/dictionary/source.rs`

**Imports and model pattern** (lines 0-26):
```rust
use super::TableEncoder;
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Clone, Debug, PartialEq)]
pub struct TableEntry {
    pub code: String,
    pub text: String,
    pub weight: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TableDictionary {
    pub(crate) entries: Vec<TableEntry>,
    pub(crate) stems: HashMap<String, Vec<String>>,
    pub(crate) encoder: TableEncoder,
}
```

**Source fallback entry point** (lines 65-113):
```rust
pub fn parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
    input: &str,
    packs: impl IntoIterator<Item = impl AsRef<str>>,
    mut import_loader: impl FnMut(&str) -> Option<String>,
    mut vocabulary_loader: impl FnMut(&str) -> Option<String>,
) -> Result<Self, TableDictionaryParseError> {
    let (metadata, mut entries) = parse_rime_dict_yaml_parts(input)?;
    append_rime_import_table_entries(&metadata, &mut entries, &mut import_loader)?;
    let vocabulary =
        apply_rime_preset_vocabulary_weights(&metadata, &mut entries, &mut vocabulary_loader);
    apply_rime_table_encoder_phrase_entries(&metadata, &mut entries, vocabulary.as_deref());
    let mut dictionary = finalize_rime_table_entries(&metadata, entries);

    for pack in packs {
        let pack = pack.as_ref();
        let Some(pack_yaml) = import_loader(pack) else {
            continue;
        };
        let Ok((pack_metadata, mut pack_entries)) = parse_rime_dict_yaml_parts(&pack_yaml)
        else {
            continue;
        };
        if append_rime_import_table_entries(
            &pack_metadata,
            &mut pack_entries,
            &mut import_loader,
        )
        .is_err()
        {
            continue;
        }
        let vocabulary = apply_rime_preset_vocabulary_weights(
            &pack_metadata,
            &mut pack_entries,
            &mut vocabulary_loader,
        );
        apply_rime_table_encoder_phrase_entries(
            &pack_metadata,
            &mut pack_entries,
            vocabulary.as_deref(),
        );
        let mut pack_dictionary = finalize_rime_table_entries(&pack_metadata, pack_entries);
        dictionary.entries.append(&mut pack_dictionary.entries);
        merge_rime_table_stems(&mut dictionary.stems, pack_dictionary.stems);
    }

    sort_rime_table_entries(&metadata, &mut dictionary.entries);
    Ok(dictionary)
}
```

**Error type pattern** (lines 461-480):
```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableDictionaryParseError {
    message: String,
}

impl std::fmt::Display for TableDictionaryParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TableDictionaryParseError {}
```

**Apply to Phase 4:** Source fallback must continue using this exact import/pack/vocabulary/encoder path so compiled-vs-source parity is measured against existing ordering.

---

### `crates/yune-core/src/dictionary/encoder.rs` (utility, transform)

**Analog:** `crates/yune-core/src/dictionary/encoder.rs`

**Imports/model pattern** (lines 0-9):
```rust
use regex::Regex;

#[derive(Clone, Debug, Default)]
pub struct TableEncoder {
    rules: Vec<TableEncodingRule>,
    exclude_pattern_sources: Vec<String>,
    exclude_patterns: Vec<Regex>,
    tail_anchor: String,
    max_phrase_length: usize,
}
```

**Fallible configuration pattern** (lines 43-75):
```rust
pub fn add_length_equal_rule(
    &mut self,
    length: usize,
    formula: &str,
) -> Result<(), TableEncoderFormulaError> {
    let rule = TableEncodingRule::from_formula(length, length, formula)?;
    self.max_phrase_length = self
        .max_phrase_length
        .max(length)
        .min(Self::MAX_PHRASE_LENGTH);
    self.rules.push(rule);
    Ok(())
}
```

**Apply to Phase 4:** UniTE-adjacent compiled `dict_settings` should feed existing `TableEncoder` primitives rather than adding a second formula parser.

---

### `crates/yune-rime-api/src/schema_install.rs` (service/provider, request-response + file-I/O)

**Analog:** `crates/yune-rime-api/src/schema_install.rs`

**Imports pattern** (lines 0-17):
```rust
use std::{collections::HashSet, fs, os::raw::c_int};

use regex::Regex;
use serde_yaml::{Mapping, Value};
use yune_core::{
    CharsetFilter, HistoryTranslator, ReverseLookupFilter, ReverseLookupTranslator,
    SchemaListTranslator, SimplifierFilter, SingleCharFilter, StaticTableTranslator,
    SwitchTranslator, TableDictionary, TaggedFilter, UniquifierFilter,
};

use crate::{
    config_scalar_bool, config_scalar_double, config_scalar_int, config_scalar_string,
    ends_with_ascii_digit, find_config_value, install_schema_punctuation_translator_from_config,
    load_runtime_config_root, resource_id::validate_data_resource_id, schema_folded_switch_options,
    schema_list_translator_entries_for_current, schema_switch_translator_switches,
    selected_runtime_data_path, switch_scalar_field, AffixSegmentor, ConfigOpenKind,
    MatcherPattern, MatcherSegmentor, PunctSegmentor, SessionState,
};
```

**Translator install pattern** (lines 88-153):
```rust
fn install_schema_dictionary_translator_from_config(
    session: &mut SessionState,
    schema_config: &Value,
    component_name: &str,
    name_space: &str,
) {
    let Some(dictionary) = load_schema_table_dictionary(schema_config, name_space) else {
        return;
    };
    let enable_charset_filter = find_config_value(
        schema_config,
        &format!("{name_space}/enable_charset_filter"),
    )
    .and_then(config_scalar_bool)
    .unwrap_or(false);
    // ... option extraction ...
    session.engine.add_translator(
        StaticTableTranslator::from_dictionary(dictionary)
            .with_spelling_algebra(&spelling_algebra)
            .with_completion(enable_completion)
            .with_charset_filter(enable_charset_filter)
            .with_sentence(enable_sentence)
            .with_sentence_over_completion(sentence_over_completion)
            .with_delimiters(delimiters)
            .with_tags(tags)
            .with_initial_quality(initial_quality)
            .with_comment_format(&comment_format)
            .with_dictionary_exclude(dictionary_exclude),
    );
}
```

**Current source loader to replace/extend** (lines 389-414):
```rust
fn load_schema_table_dictionary(
    schema_config: &Value,
    name_space: &str,
) -> Option<TableDictionary> {
    let dictionary_name = find_config_value(schema_config, &format!("{name_space}/dictionary"))
        .and_then(config_scalar_string)
        .and_then(|dictionary_name| validate_data_resource_id(&dictionary_name))?;
    let dictionary_path = selected_runtime_data_path(&format!("{dictionary_name}.dict.yaml"))?;
    let dictionary_yaml = fs::read_to_string(dictionary_path).ok()?;
    let packs = schema_dictionary_packs(schema_config, name_space);
    TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
        &dictionary_yaml,
        packs,
        |import_table| {
            let import_table = validate_data_resource_id(import_table)?;
            selected_runtime_data_path(&format!("{import_table}.dict.yaml"))
                .and_then(|path| fs::read_to_string(path).ok())
        },
        |vocabulary| {
            let vocabulary = validate_data_resource_id(vocabulary)?;
            selected_runtime_data_path(&format!("{vocabulary}.txt"))
                .and_then(|path| fs::read_to_string(path).ok())
        },
    )
    .ok()
}
```

**Apply to Phase 4:** Keep all schema scalar extraction and `StaticTableTranslator::from_dictionary` wiring. Replace the raw `Option<TableDictionary>` internally with a structured compiled/source/no-usable-path outcome, but preserve `validate_data_resource_id` before every file-name construction.

---

### `crates/yune-rime-api/src/runtime.rs` (config/service, file-I/O)

**Analog:** `crates/yune-rime-api/src/runtime.rs`

**Runtime path model pattern** (lines 16-43):
```rust
pub(crate) struct RuntimePaths {
    pub(crate) shared_data_dir: CString,
    pub(crate) user_data_dir: CString,
    pub(crate) prebuilt_data_dir: CString,
    pub(crate) staging_dir: CString,
    pub(crate) sync_dir: CString,
    pub(crate) user_id: CString,
    pub(crate) user_data_sync_dir: CString,
    pub(crate) distribution_name: CString,
    pub(crate) distribution_code_name: CString,
    pub(crate) distribution_version: CString,
    pub(crate) app_name: CString,
    pub(crate) log_dir: CString,
    pub(crate) backup_config_files: bool,
}
```

**Trait/default path pattern** (lines 108-116):
```rust
let shared_data_dir = provided_string(unsafe { ptr::addr_of!((*traits).shared_data_dir) })
    .unwrap_or_else(|| ".".to_owned());
let user_data_dir = provided_string(unsafe { ptr::addr_of!((*traits).user_data_dir) })
    .unwrap_or_else(|| ".".to_owned());
let prebuilt_data_dir =
    provided_string(unsafe { ptr::addr_of!((*traits).prebuilt_data_dir) })
        .unwrap_or_else(|| path_join(&shared_data_dir, "build"));
let staging_dir = provided_string(unsafe { ptr::addr_of!((*traits).staging_dir) })
    .unwrap_or_else(|| path_join(&user_data_dir, "build"));
```

**Apply to Phase 4:** Do not put parser logic here. Add path helper only if compiled artifact resolution needs shared staging/prebuilt root behavior; otherwise use `selected_runtime_data_path` from `lib.rs`.

---

### `crates/yune-rime-api/src/deployment.rs` (service, batch + file-I/O)

**Analog:** `crates/yune-rime-api/src/deployment.rs`

**Imports pattern** (lines 0-19):
```rust
use std::{
    collections::HashSet,
    fs,
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    sync::atomic::Ordering,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_yaml::{Mapping, Number, Value};

use crate::{
    apply_config_directives, apply_custom_patch, apply_legacy_preset_config_plugins, bool_from,
    cstring_from_lossless_str, find_config_value, load_runtime_config_root,
    normalize_config_resource_id, optional_c_string, path_join,
    resource_id::validate_data_resource_id, runtime_paths, runtime_user_data_sync_dir,
    service_started, set_build_info, set_config_value, source_modified_secs,
    source_uses_auto_custom_patch, sync_all_user_dicts, user_dict_upgrade, Bool, ConfigOpenKind,
    RimeCleanupAllSessions, RimeSetup, RimeTraits, FALSE, RIME_VERSION_BYTES, TRUE,
};
```

**Workspace update orchestration pattern** (lines 560-584):
```rust
pub(crate) fn workspace_update() -> bool {
    if !deploy_config_file("default.yaml", "config_version") {
        return false;
    }
    let _ = symlink_prebuilt_dictionaries();

    let default_config = load_runtime_config_root("default", ConfigOpenKind::Deployed);
    let Some(schema_ids) = workspace_schema_ids(&default_config) else {
        return false;
    };

    let mut built = HashSet::new();
    let mut success = true;
    for schema_id in schema_ids {
        if !workspace_update_schema(&schema_id, false, &mut built) {
            success = false;
        }
    }

    write_last_build_time() && success
}

pub(crate) fn run_workspace_maintenance_tasks() -> bool {
    workspace_update() && user_dict_upgrade() && cleanup_trash()
}
```

**Resource-validation pattern inside batch selection** (lines 632-643):
```rust
fn schema_dependencies(schema_config: &Value) -> Vec<String> {
    let Some(Value::Sequence(dependencies)) =
        find_config_value(schema_config, "schema/dependencies")
    else {
        return Vec::new();
    };
    dependencies
        .iter()
        .filter_map(Value::as_str)
        .filter_map(validate_data_resource_id)
        .collect()
}
```

**Apply to Phase 4:** Add dictionary rebuild execution/freshness as deterministic workspace-update work after schema config deployment, with per-artifact success booleans/actions. Follow the same `bool` API surface but keep internal result structs richer.

---

### `crates/yune-rime-api/src/resource_id.rs` (utility/guard, transform)

**Analog:** `crates/yune-rime-api/src/resource_id.rs`

**Validation pattern** (lines 22-47):
```rust
pub(crate) fn validate_data_resource_id(id: &str) -> Option<String> {
    validate_logical_id(id)
}

fn validate_logical_id(id: &str) -> Option<String> {
    if id.is_empty()
        || id == "."
        || id == ".."
        || id.starts_with('~')
        || id.contains('\0')
        || id.contains('/')
        || id.contains('\\')
        || has_windows_drive_prefix(id)
    {
        return None;
    }

    Some(id.to_owned())
}
```

**Apply to Phase 4:** Every schema-provided dictionary, prism, reverse, pack, import, and vocabulary ID must pass this validator before path construction. Do not add path separators or absolute path support.

---

### `crates/yune-core/src/dictionary/mod.rs` (config/facade, transform)

**Analog:** `crates/yune-core/src/dictionary/mod.rs`

**Module/export pattern** (lines 0-12):
```rust
mod compiled;
mod encoder;
mod source;

pub use compiled::{
    parse_rime_prism_bin_metadata, parse_rime_reverse_bin_metadata, parse_rime_table_bin_metadata,
    rime_checksum_bytes, rime_dict_rebuild_plan, rime_dict_source_checksum, RimeChecksumComputer,
    RimeCompiledMetadataError, RimeDictRebuildError, RimeDictRebuildInput, RimeDictRebuildPlan,
    RimePrismBinMetadata, RimePrismChecksumMetadata, RimeReverseBinMetadata, RimeTableBinMetadata,
};
pub use encoder::{CodeCoords, TableEncoder, TableEncoderFormulaError, TableEncodingRule};
pub(crate) use source::normalize_table_code;
pub use source::{TableDictionary, TableDictionaryParseError, TableEntry};
```

**Apply to Phase 4:** Add `mod compiled_table; mod compiled_prism; mod compiled_reverse;` here if files are split. Export only stable planner-needed types/functions; keep parser internals private.

---

## Test Pattern Assignments

### `crates/yune-core/src/lib.rs` tests (test, transform)

**Analog:** `crates/yune-core/src/lib.rs` test module

**Compiled parser fixture pattern** (lines 2335-2386):
```rust
#[test]
fn parses_librime_compiled_table_prism_and_reverse_metadata() {
    let mut table = vec![0; 68];
    put_c_string(&mut table, 0, b"Rime::Table/4.0");
    put_u32_le(&mut table, 32, 0x1111_1111);
    put_u32_le(&mut table, 36, 7);
    put_u32_le(&mut table, 40, 11);
    put_u32_le(&mut table, 44, 0x40);
    put_u32_le(&mut table, 48, 0x44);
    put_u32_le(&mut table, 64, 13);
    assert_eq!(
        parse_rime_table_bin_metadata(&table),
        Ok(RimeTableBinMetadata {
            dict_file_checksum: 0x1111_1111,
            num_syllables: 7,
            num_entries: 11,
            string_table_size: 13,
        })
    );
}
```

**Rejection test pattern** (lines 2389-2437):
```rust
#[test]
fn compiled_metadata_parser_matches_librime_load_rejection_cases() {
    let mut table = vec![0; 68];
    put_c_string(&mut table, 0, b"Rime::Table/3.0");
    assert_eq!(
        parse_rime_table_bin_metadata(&table),
        Err(RimeCompiledMetadataError::UnsupportedVersion)
    );
    // ... invalid section and TooShort checks ...
}
```

**Apply to Phase 4:** Add malformed offset/count/length/version tests beside these helpers. Use local `Vec<u8>` fixture construction and assert typed errors.

---

### `crates/yune-rime-api/src/tests/schema_selection.rs` (test, request-response + file-I/O)

**Analog:** `crates/yune-rime-api/src/tests/schema_selection.rs`

**Schema-loaded dictionary fixture pattern** (lines 1128-1227):
```rust
#[test]
fn select_schema_loads_librime_dictionary_packs() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("schema-dictionary-packs");
    let shared = root.join("shared");
    let user = root.join("user");
    let staging = user.join("build");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::create_dir_all(&staging).expect("staging dir should be created");
    fs::write(
        staging.join("luna.schema.yaml"),
        "\
schema:
  schema_id: luna
  name: Luna
engine:
  translators:
    - table_translator
translator:
  dictionary: luna
  packs:
    - luna_pack
    - missing_pack
",
    )
    .expect("schema config should be written");
    // ... write shared dictionaries, RimeSetup, RimeSelectSchema, RimeProcessKey ...
    assert_eq!(texts, ["爸", "吧", "八", "ba"]);
}
```

**Preset vocabulary/encoder behavior pattern** (lines 1235-1435): Use `select_schema_uses_preset_vocabulary_for_dictionary_weights` and `select_schema_encodes_rule_based_dictionary_and_preset_phrases` for compiled-vs-source parity fixtures; they already assert user-visible candidate order.

**Apply to Phase 4:** Add compiled-first/source-fallback tests here or in a split `dictionary_data.rs` test module using the same `test_guard`, temp shared/user/staging dirs, `RimeSetup`, `RimeSelectSchema`, `RimeProcessKey`, `RimeGetContext`, `RimeFreeContext`, cleanup pattern.

---

### `crates/yune-rime-api/src/tests/deployment.rs` (test, batch + file-I/O)

**Analog:** `crates/yune-rime-api/src/tests/deployment.rs`

**Runtime path trait test pattern** (lines 2-93):
```rust
#[test]
fn setup_and_initialize_expose_runtime_metadata_paths() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let shared = CString::new("/tmp/yune-shared").expect("path should be valid");
    let user = CString::new("/tmp/yune-user").expect("path should be valid");
    let staging = CString::new("/tmp/yune-stage").expect("path should be valid");
    let mut traits = empty_traits();
    traits.shared_data_dir = shared.as_ptr();
    traits.user_data_dir = user.as_ptr();
    traits.staging_dir = staging.as_ptr();
    // SAFETY: traits points to a valid RimeTraits object with valid strings.
    unsafe { RimeSetup(&traits) };
    // ... assert staging/prebuilt/user/shared getters ...
}
```

**Workspace update/rebuild orchestration test pattern** (lines 1747-1864):
```rust
#[test]
fn workspace_update_deploys_default_schemas_and_dependencies() {
    let _guard = test_guard();
    RimeCleanupAllSessions();
    let root = unique_temp_dir("workspace-update");
    let shared = root.join("shared");
    let user = root.join("user");
    fs::create_dir_all(&shared).expect("shared dir should be created");
    fs::write(shared.join("default.yaml"), "...")
        .expect("default config should be written");
    // ... setup traits ...
    unsafe { RimeDeployerInitialize(&traits) };
    assert_eq!(RimeRunTask(workspace_task.as_ptr()), TRUE);
    // ... assert build outputs and user.yaml last_build_time ...
}
```

**Apply to Phase 4:** Rebuild/freshness tests should follow deployment tests, asserting staged/prebuilt/source file behavior and `workspace_update`/`prebuild_all_schemas` task outcomes without invoking external librime compilers.

---

## Shared Patterns

### Resource-ID validation before path joins

**Source:** `crates/yune-rime-api/src/resource_id.rs` lines 22-47 and `schema_install.rs` lines 393-411
**Apply to:** `schema_install.rs`, `deployment.rs`, any compiled artifact resolver

```rust
let dictionary_name = find_config_value(schema_config, &format!("{name_space}/dictionary"))
    .and_then(config_scalar_string)
    .and_then(|dictionary_name| validate_data_resource_id(&dictionary_name))?;
let dictionary_path = selected_runtime_data_path(&format!("{dictionary_name}.dict.yaml"))?;
```

### Runtime data root order

**Source:** `crates/yune-rime-api/src/lib.rs` lines 1596-1609
**Apply to:** compiled `.table.bin`, `.prism.bin`, `.reverse.bin`, source `.dict.yaml`, vocabulary `.txt`

```rust
pub(crate) fn selected_runtime_data_path(file_name: &str) -> Option<PathBuf> {
    let file_name = validate_data_resource_id(file_name)?;
    let paths = runtime_paths()
        .lock()
        .expect("runtime paths should not be poisoned");
    [
        paths.staging_dir.to_string_lossy().into_owned(),
        paths.prebuilt_data_dir.to_string_lossy().into_owned(),
        paths.shared_data_dir.to_string_lossy().into_owned(),
    ]
    .into_iter()
    .map(|root| Path::new(&root).join(&file_name))
    .find(|path| path.is_file())
}
```

### Candidate-order parity through existing translator

**Source:** `crates/yune-core/src/translator/mod.rs` lines 79-106 and 238-272
**Apply to:** compiled table materialization and source fallback

```rust
pub fn from_dictionary(dictionary: TableDictionary) -> Self {
    let entries = dictionary
        .entries
        .into_iter()
        .map(|entry| {
            let candidate = Candidate {
                text: entry.text,
                comment: entry.code.clone(),
                source: CandidateSource::Table,
                quality: entry.weight,
            };
            (entry.code, candidate)
        })
        .collect();
    Self { /* existing defaults */ }
}
```

### Safe parser errors instead of panics

**Source:** `crates/yune-core/src/dictionary/compiled.rs` lines 89-95 and 157-169
**Apply to:** all compiled payload readers

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RimeCompiledMetadataError {
    TooShort,
    InvalidFormat,
    UnsupportedVersion,
    MissingRequiredSection,
}
```

### Test fixture lifecycle

**Source:** `crates/yune-rime-api/src/tests/mod.rs` lines 94-104 and 330-339
**Apply to:** all schema-loaded compiled-data tests

```rust
fn test_guard() -> MutexGuard<'static, ()> {
    static TEST_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    let guard = TEST_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("test lock should not be poisoned");
    let traits = empty_traits();
    // SAFETY: empty traits points to valid storage for the duration of the call.
    unsafe { RimeInitialize(&traits) };
    guard
}

fn unique_temp_dir(name: &str) -> std::path::PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    env::temp_dir().join(format!(
        "yune-rime-api-{name}-{}-{nonce}",
        std::process::id()
    ))
}
```

## No Exact Analog Found

Files with no exact same-role/same-format analog in the codebase. Planner should combine the partial analogs above with RESEARCH.md byte-layout guidance.

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `crates/yune-core/src/dictionary/compiled_table.rs` | utility/parser | file-I/O + transform | Existing compiled parser reads metadata only, not bounded table payload sections or MARISA string data. |
| `crates/yune-core/src/dictionary/compiled_prism.rs` | utility/parser | file-I/O + transform | Existing compiled parser reads prism metadata only, not Darts/spelling-map descriptors. |
| `crates/yune-core/src/dictionary/compiled_reverse.rs` | utility/parser | file-I/O + transform | Existing compiled parser reads reverse metadata only, not key/value tries, stems, or `dict_settings`. |
| `crates/yune-rime-api/src/tests/distribution_schema_comparison.rs` | test | request-response + file-I/O | File is referenced by planning context but is not present in the current worktree; use `schema_selection.rs` schema-loaded patterns instead or create a focused split test module. |

## Metadata

**Analog search scope:** `crates/yune-core/src`, `crates/yune-rime-api/src`, `crates/yune-rime-api/src/tests`
**Files scanned:** Rust source files under core and rime-api crates; focused analogs read: 10
**Project instructions:** No `CLAUDE.md` found in the worktree. No `.claude/skills/` or `.agents/skills/` directory found in the worktree.
**Pattern extraction date:** 2026-04-29
