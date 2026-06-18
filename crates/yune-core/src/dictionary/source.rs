use super::TableEncoder;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[derive(Clone, Debug, PartialEq)]
pub struct TableEntry {
    pub code: String,
    pub text: String,
    pub weight: f32,
}

impl TableEntry {
    #[must_use]
    pub fn new(code: impl Into<String>, text: impl Into<String>, weight: f32) -> Self {
        Self {
            code: normalize_table_code(&code.into()),
            text: text.into(),
            weight,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DictionaryLookupRecord {
    pub code: String,
    pub fields: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RimeCorrectionEntry {
    pub observed_input: String,
    pub canonical_code: String,
}

impl RimeCorrectionEntry {
    #[must_use]
    pub fn new(observed_input: impl Into<String>, canonical_code: impl Into<String>) -> Self {
        Self {
            observed_input: normalize_table_code(&observed_input.into()),
            canonical_code: normalize_table_code(&canonical_code.into()),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RimeToleranceRule {
    pub near_code: String,
    pub candidate_codes: Vec<String>,
}

impl RimeToleranceRule {
    #[must_use]
    pub fn new(
        near_code: impl Into<String>,
        candidate_codes: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            near_code: normalize_table_code(&near_code.into()),
            candidate_codes: candidate_codes
                .into_iter()
                .map(Into::into)
                .map(|code| normalize_table_code(&code))
                .filter(|code| !code.is_empty())
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TableDictionary {
    pub(crate) entries: Vec<TableEntry>,
    pub(crate) stems: HashMap<String, Vec<String>>,
    pub(crate) dict_settings: BTreeMap<String, String>,
    pub(crate) encoder: TableEncoder,
    pub(crate) corrections: Vec<RimeCorrectionEntry>,
    pub(crate) tolerance_rules: Vec<RimeToleranceRule>,
    pub(crate) lookup_records: HashMap<String, Vec<DictionaryLookupRecord>>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TableDictionaryAdvancedData {
    pub stems: HashMap<String, Vec<String>>,
    pub dict_settings: BTreeMap<String, String>,
    pub encoder: TableEncoder,
    pub corrections: Vec<RimeCorrectionEntry>,
    pub tolerance_rules: Vec<RimeToleranceRule>,
}

impl TableDictionary {
    #[must_use]
    pub fn new(entries: impl IntoIterator<Item = TableEntry>) -> Self {
        Self::with_advanced_data(entries, TableDictionaryAdvancedData::default())
    }

    #[must_use]
    pub fn with_advanced_data(
        entries: impl IntoIterator<Item = TableEntry>,
        advanced: TableDictionaryAdvancedData,
    ) -> Self {
        Self {
            entries: entries.into_iter().collect(),
            stems: advanced.stems,
            dict_settings: advanced.dict_settings,
            encoder: advanced.encoder,
            corrections: advanced.corrections,
            tolerance_rules: advanced.tolerance_rules,
            lookup_records: HashMap::new(),
        }
    }

    #[must_use]
    pub fn with_merged_advanced_data_from(mut self, other: &Self) -> Self {
        merge_rime_table_stems(&mut self.stems, other.stems.clone());
        self.dict_settings.extend(other.dict_settings.clone());
        if !self.encoder.loaded() && other.encoder.loaded() {
            self.encoder = other.encoder.clone();
        }
        self.corrections.extend(other.corrections.clone());
        self.tolerance_rules.extend(other.tolerance_rules.clone());
        merge_dictionary_lookup_records(&mut self.lookup_records, other.lookup_records.clone());
        self
    }

    pub fn parse_rime_dict_yaml(input: &str) -> Result<Self, TableDictionaryParseError> {
        let (metadata, entries) = parse_rime_dict_yaml_parts(input)?;
        Ok(finalize_rime_table_entries(&metadata, entries))
    }

    pub fn parse_rime_dict_yaml_with_imports(
        input: &str,
        mut import_loader: impl FnMut(&str) -> Option<String>,
    ) -> Result<Self, TableDictionaryParseError> {
        let (metadata, mut entries) = parse_rime_dict_yaml_parts(input)?;
        append_rime_import_table_entries(&metadata, &mut entries, &mut import_loader)?;
        Ok(finalize_rime_table_entries(&metadata, entries))
    }

    pub fn parse_rime_dict_yaml_with_imports_and_packs(
        input: &str,
        packs: impl IntoIterator<Item = impl AsRef<str>>,
        mut import_loader: impl FnMut(&str) -> Option<String>,
    ) -> Result<Self, TableDictionaryParseError> {
        Self::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
            input,
            packs,
            &mut import_loader,
            |_| None,
        )
    }

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
            merge_dictionary_lookup_records(
                &mut dictionary.lookup_records,
                pack_dictionary.lookup_records,
            );
            dictionary
                .dict_settings
                .extend(pack_dictionary.dict_settings);
        }

        sort_rime_table_entries(&metadata, &mut dictionary.entries);
        Ok(dictionary)
    }

    #[must_use]
    pub fn entries(&self) -> &[TableEntry] {
        &self.entries
    }

    #[must_use]
    pub fn stems(&self) -> &HashMap<String, Vec<String>> {
        &self.stems
    }

    #[must_use]
    pub fn stems_for(&self, text: &str) -> Option<&[String]> {
        self.stems.get(text).map(Vec::as_slice)
    }

    #[must_use]
    pub fn dict_settings(&self) -> &BTreeMap<String, String> {
        &self.dict_settings
    }

    #[must_use]
    pub fn encoder(&self) -> &TableEncoder {
        &self.encoder
    }

    #[must_use]
    pub fn corrections(&self) -> &[RimeCorrectionEntry] {
        &self.corrections
    }

    #[must_use]
    pub fn tolerance_rules(&self) -> &[RimeToleranceRule] {
        &self.tolerance_rules
    }

    #[must_use]
    pub fn lookup_records_for(&self, text: &str) -> Option<&[DictionaryLookupRecord]> {
        self.lookup_records.get(text).map(Vec::as_slice)
    }
}

fn parse_rime_dict_yaml_parts(
    input: &str,
) -> Result<(RimeTableMetadata, Vec<RimeParsedTableEntry>), TableDictionaryParseError> {
    let mut metadata = RimeTableMetadata::default();
    let mut in_header = false;
    let mut body_start = None;

    for (line_index, line) in input.lines().enumerate() {
        let line = strip_utf8_bom(line);
        let trimmed = line.trim();
        if !in_header {
            if trimmed == "---" {
                in_header = true;
                continue;
            }

            if trimmed.is_empty() {
                continue;
            }

            in_header = true;
        }

        if trimmed == "..." {
            body_start = Some(line_index + 1);
            break;
        }
        metadata.read_header_line(line);
    }
    metadata.finish_header();

    let body_start = body_start.ok_or_else(|| {
        TableDictionaryParseError::new("RIME dictionary header is missing terminating '...'")
    })?;
    if !metadata.is_complete() {
        return Err(TableDictionaryParseError::new(
            "RIME dictionary header is missing required name or version",
        ));
    }
    let mut entries = Vec::new();
    let mut comments_enabled = true;

    for line in input.lines().skip(body_start) {
        if line.trim().is_empty() {
            continue;
        }
        if comments_enabled && line.starts_with('#') {
            if line == "# no comment" {
                comments_enabled = false;
            }
            continue;
        }

        if let Some(entry) = metadata.parse_entry(line) {
            entries.push(entry);
        }
    }

    Ok((metadata, entries))
}

fn append_rime_import_table_entries(
    metadata: &RimeTableMetadata,
    entries: &mut Vec<RimeParsedTableEntry>,
    import_loader: &mut impl FnMut(&str) -> Option<String>,
) -> Result<(), TableDictionaryParseError> {
    for import_table in &metadata.import_tables {
        if Some(import_table.as_str()) == metadata.name.as_deref() {
            continue;
        }
        let import_yaml = import_loader(import_table).ok_or_else(|| {
            TableDictionaryParseError::new(format!(
                "RIME dictionary import table '{import_table}' is missing"
            ))
        })?;
        let (_, mut imported_entries) = parse_rime_dict_yaml_parts(&import_yaml)?;
        entries.append(&mut imported_entries);
    }
    Ok(())
}

fn finalize_rime_table_entries(
    metadata: &RimeTableMetadata,
    mut entries: Vec<RimeParsedTableEntry>,
) -> TableDictionary {
    let stems = collect_rime_table_stems(&entries);
    let lookup_records = collect_dictionary_lookup_records(&entries);
    dedupe_rime_table_entries(&mut entries);
    let mut entries = entries
        .into_iter()
        .map(|entry| entry.entry)
        .collect::<Vec<_>>();
    sort_rime_table_entries(metadata, &mut entries);
    TableDictionary {
        entries,
        stems,
        dict_settings: metadata.dict_settings.clone(),
        encoder: metadata.encoder.clone(),
        corrections: metadata.corrections.clone(),
        tolerance_rules: metadata.tolerance_rules.clone(),
        lookup_records,
    }
}

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

fn merge_rime_table_stems(
    target: &mut HashMap<String, Vec<String>>,
    source: HashMap<String, Vec<String>>,
) {
    for (text, stems) in source {
        let mut merged = target
            .remove(&text)
            .unwrap_or_default()
            .into_iter()
            .collect::<BTreeSet<_>>();
        merged.extend(stems);
        target.insert(text, merged.into_iter().collect());
    }
}

fn collect_dictionary_lookup_records(
    entries: &[RimeParsedTableEntry],
) -> HashMap<String, Vec<DictionaryLookupRecord>> {
    let mut lookup_records: HashMap<String, Vec<DictionaryLookupRecord>> = HashMap::new();
    for entry in entries {
        if entry.entry.code.is_empty() || entry.raw_fields.is_empty() {
            continue;
        }
        lookup_records
            .entry(entry.entry.text.clone())
            .or_default()
            .push(DictionaryLookupRecord {
                code: entry.entry.code.clone(),
                fields: entry.raw_fields.clone(),
            });
    }
    lookup_records
}

fn merge_dictionary_lookup_records(
    target: &mut HashMap<String, Vec<DictionaryLookupRecord>>,
    source: HashMap<String, Vec<DictionaryLookupRecord>>,
) {
    for (text, mut records) in source {
        target.entry(text).or_default().append(&mut records);
    }
}

fn apply_rime_preset_vocabulary_weights(
    metadata: &RimeTableMetadata,
    entries: &mut [RimeParsedTableEntry],
    vocabulary_loader: &mut impl FnMut(&str) -> Option<String>,
) -> Option<String> {
    if !metadata.uses_preset_vocabulary() {
        return None;
    }
    let vocabulary = vocabulary_loader(metadata.vocabulary_name())?;
    let vocabulary_weights = parse_rime_preset_vocabulary(&vocabulary);
    for entry in entries {
        let weight = entry.raw_weight.trim();
        let Some(vocabulary_weight) = vocabulary_weights.get(&entry.entry.text).copied() else {
            continue;
        };
        if weight.is_empty() {
            entry.entry.weight = vocabulary_weight;
        } else if weight.ends_with('%') {
            entry.entry.weight = vocabulary_weight * parse_rime_entry_weight_percentage(weight);
        }
    }
    Some(vocabulary)
}

fn apply_rime_table_encoder_phrase_entries(
    metadata: &RimeTableMetadata,
    entries: &mut Vec<RimeParsedTableEntry>,
    vocabulary: Option<&str>,
) {
    if !metadata.encoder.loaded() {
        return;
    }

    let source_collection = entries
        .iter()
        .map(|entry| entry.entry.text.clone())
        .collect::<HashSet<_>>();
    let phrase_encoder = RimeTablePhraseEncoder::new(metadata, entries);
    let mut encoded_entries = entries
        .iter()
        .filter(|entry| entry.entry.code.is_empty())
        .flat_map(|entry| {
            phrase_encoder.encode_phrase_entries(&entry.entry.text, entry.entry.weight)
        })
        .collect::<Vec<_>>();

    if let Some(vocabulary) = vocabulary {
        for (phrase, weight) in parse_rime_preset_vocabulary_entries(vocabulary) {
            if source_collection.contains(&phrase)
                || !metadata.is_qualified_preset_phrase(&phrase, weight)
            {
                continue;
            }
            encoded_entries.extend(phrase_encoder.encode_phrase_entries(&phrase, weight));
        }
    }

    entries.retain(|entry| !entry.entry.code.is_empty());
    entries.extend(encoded_entries);
}

struct RimeTablePhraseEncoder<'a> {
    metadata: &'a RimeTableMetadata,
    stems: HashMap<String, Vec<String>>,
    words: HashMap<String, Vec<(String, f32)>>,
    total_weight: HashMap<String, f32>,
}

impl<'a> RimeTablePhraseEncoder<'a> {
    const DFS_LIMIT: usize = 32;

    fn new(metadata: &'a RimeTableMetadata, entries: &[RimeParsedTableEntry]) -> Self {
        let stems = collect_rime_table_stems(entries);
        let mut words: HashMap<String, Vec<(String, f32)>> = HashMap::new();
        let mut total_weight: HashMap<String, f32> = HashMap::new();
        let mut seen_words = HashSet::new();
        for entry in entries {
            if entry.entry.code.is_empty() || entry.single_syllable_duplicate_key.is_none() {
                continue;
            }
            let key = (entry.entry.text.clone(), entry.entry.code.clone());
            if !seen_words.insert(key) {
                continue;
            }
            words
                .entry(entry.entry.text.clone())
                .or_default()
                .push((entry.entry.code.clone(), entry.entry.weight));
            *total_weight.entry(entry.entry.text.clone()).or_default() += entry.entry.weight;
        }

        Self {
            metadata,
            stems,
            words,
            total_weight,
        }
    }

    fn encode_phrase_entries(&self, phrase: &str, weight: f32) -> Vec<RimeParsedTableEntry> {
        self.encode_phrase(phrase)
            .into_iter()
            .map(|code| RimeParsedTableEntry {
                entry: TableEntry::new(code, phrase, weight),
                raw_weight: weight.to_string(),
                raw_stem: None,
                raw_fields: Vec::new(),
                single_syllable_duplicate_key: None,
            })
            .collect()
    }

    fn encode_phrase(&self, phrase: &str) -> Vec<String> {
        let phrase_length = phrase.chars().count();
        if phrase_length > self.metadata.encoder.max_phrase_length() {
            return Vec::new();
        }
        let characters = phrase.chars().map(|ch| ch.to_string()).collect::<Vec<_>>();
        let mut raw_code = Vec::new();
        let mut limit = Self::DFS_LIMIT;
        let mut encoded = Vec::new();
        self.dfs_encode(&characters, 0, &mut raw_code, &mut limit, &mut encoded);
        encoded
    }

    fn dfs_encode(
        &self,
        characters: &[String],
        start: usize,
        raw_code: &mut Vec<String>,
        limit: &mut usize,
        encoded: &mut Vec<String>,
    ) {
        if start == characters.len() {
            *limit = limit.saturating_sub(1);
            if let Some(code) = self.metadata.encoder.encode(raw_code) {
                encoded.push(code);
            }
            return;
        }

        for code in self.translate_word(&characters[start]) {
            if self.metadata.encoder.is_code_excluded(&code) {
                continue;
            }
            raw_code.push(code);
            self.dfs_encode(characters, start + 1, raw_code, limit, encoded);
            raw_code.pop();
            if *limit == 0 {
                return;
            }
        }
    }

    fn translate_word(&self, word: &str) -> Vec<String> {
        if let Some(stems) = self.stems.get(word) {
            return stems.clone();
        }

        let Some(words) = self.words.get(word) else {
            return Vec::new();
        };
        let min_weight = self.total_weight.get(word).copied().unwrap_or_default() * 0.05;
        let mut codes = words
            .iter()
            .filter(|(_, weight)| *weight >= min_weight)
            .map(|(code, _)| code.clone())
            .collect::<Vec<_>>();
        codes.sort();
        codes
    }
}

fn sort_rime_table_entries(metadata: &RimeTableMetadata, entries: &mut [TableEntry]) {
    if metadata.sort_by_weight {
        entries.sort_by(|left, right| {
            left.code.cmp(&right.code).then_with(|| {
                right
                    .weight
                    .partial_cmp(&left.weight)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });
    }
}

fn dedupe_rime_table_entries(entries: &mut Vec<RimeParsedTableEntry>) {
    let mut seen = HashSet::new();
    entries.retain(|entry| {
        let Some(key) = entry.single_syllable_duplicate_key.as_ref() else {
            return true;
        };
        seen.insert(key.clone())
    });
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableDictionaryParseError {
    message: String,
}

impl TableDictionaryParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for TableDictionaryParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TableDictionaryParseError {}

#[derive(Clone, Debug)]
struct RimeTableMetadata {
    columns: Vec<String>,
    import_tables: Vec<String>,
    reading_list: Option<RimeTableHeaderList>,
    pending_list_clear: Option<RimeTableHeaderList>,
    sort_by_weight: bool,
    use_preset_vocabulary: bool,
    vocabulary: Option<String>,
    max_phrase_length: usize,
    min_phrase_weight: f32,
    dict_settings: BTreeMap<String, String>,
    dict_settings_stack: Vec<String>,
    corrections: Vec<RimeCorrectionEntry>,
    tolerance_rules: Vec<RimeToleranceRule>,
    encoder: TableEncoder,
    in_encoder: bool,
    encoder_list: Option<RimeEncoderList>,
    pending_encoder_rule: Option<RimeEncoderRuleDraft>,
    name: Option<String>,
    has_name: bool,
    has_version: bool,
}

#[derive(Clone, Debug)]
struct RimeParsedTableEntry {
    entry: TableEntry,
    raw_weight: String,
    raw_stem: Option<String>,
    raw_fields: Vec<String>,
    single_syllable_duplicate_key: Option<(String, String)>,
}

impl Default for RimeTableMetadata {
    fn default() -> Self {
        Self {
            columns: vec!["text".to_owned(), "code".to_owned(), "weight".to_owned()],
            import_tables: Vec::new(),
            reading_list: None,
            pending_list_clear: None,
            sort_by_weight: true,
            use_preset_vocabulary: false,
            vocabulary: None,
            max_phrase_length: 0,
            min_phrase_weight: 0.0,
            dict_settings: BTreeMap::new(),
            dict_settings_stack: Vec::new(),
            corrections: Vec::new(),
            tolerance_rules: Vec::new(),
            encoder: TableEncoder::new(),
            in_encoder: false,
            encoder_list: None,
            pending_encoder_rule: None,
            name: None,
            has_name: false,
            has_version: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RimeTableHeaderList {
    Columns,
    ImportTables,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RimeEncoderList {
    ExcludePatterns,
    Rules,
}

#[derive(Clone, Debug, Default)]
struct RimeEncoderRuleDraft {
    length_equal: Option<usize>,
    length_range: Option<(usize, usize)>,
    formula: Option<String>,
}

impl RimeTableMetadata {
    fn read_header_line(&mut self, line: &str) {
        let indent = line.len() - line.trim_start().len();
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            return;
        }

        if self.in_encoder && indent > 0 {
            self.read_encoder_header_line(trimmed);
            return;
        }
        if self.in_encoder {
            self.finish_encoder_rule();
            self.in_encoder = false;
            self.encoder_list = None;
        }
        if indent > 0 && !self.dict_settings_stack.is_empty() {
            self.read_dict_settings_header_line(indent, trimmed);
            return;
        }
        if !self.dict_settings_stack.is_empty() && indent == 0 {
            self.dict_settings_stack.clear();
        }

        if let Some(list) = self.reading_list {
            if trimmed == "-" {
                self.push_header_list_item(list, "");
                return;
            }
            if let Some(column) = trimmed.strip_prefix("- ") {
                self.push_header_list_item(list, column);
                return;
            }
            self.reading_list = None;
            self.pending_list_clear = None;
        }

        if let Some(encoder) = rime_header_value(trimmed, "encoder") {
            self.finish_encoder_rule();
            self.in_encoder = parse_yaml_scalar_node(encoder).is_none();
            self.encoder_list = None;
            return;
        }

        if let Some(dict_settings) = rime_header_value(trimmed, "dict_settings") {
            self.dict_settings_stack.clear();
            if parse_yaml_scalar_node(dict_settings).is_none() {
                self.dict_settings_stack.push("dict_settings".to_owned());
            }
            return;
        }

        if let Some(correction) = rime_header_value(trimmed, "correction") {
            self.read_correction_header_line(correction);
            return;
        }

        if let Some(correction) = rime_header_value(trimmed, "corrections") {
            self.read_correction_header_line(correction);
            return;
        }

        if let Some(tolerance) = rime_header_value(trimmed, "tolerance") {
            self.read_tolerance_header_line(tolerance);
            return;
        }

        if let Some(tolerance) = rime_header_value(trimmed, "tolerance_rules") {
            self.read_tolerance_header_line(tolerance);
            return;
        }

        if let Some(columns) = rime_header_value(trimmed, "columns") {
            self.read_header_list(RimeTableHeaderList::Columns, columns);
            return;
        }

        if let Some(import_tables) = rime_header_value(trimmed, "import_tables") {
            self.read_header_list(RimeTableHeaderList::ImportTables, import_tables);
            return;
        }

        if let Some(sort_order) = rime_header_value(trimmed, "sort") {
            self.sort_by_weight = parse_yaml_scalar(sort_order) != "original";
            return;
        }

        if let Some(use_preset_vocabulary) = rime_header_value(trimmed, "use_preset_vocabulary") {
            self.use_preset_vocabulary = parse_yaml_bool(use_preset_vocabulary).unwrap_or(false);
            return;
        }

        if let Some(vocabulary) = rime_header_value(trimmed, "vocabulary") {
            self.vocabulary = parse_yaml_scalar_node(vocabulary);
            return;
        }

        if let Some(max_phrase_length) = rime_header_value(trimmed, "max_phrase_length") {
            self.max_phrase_length = parse_yaml_usize(max_phrase_length).unwrap_or(0);
            return;
        }

        if let Some(min_phrase_weight) = rime_header_value(trimmed, "min_phrase_weight") {
            self.min_phrase_weight = parse_yaml_f32(min_phrase_weight).unwrap_or(0.0);
            return;
        }

        if let Some(name) = rime_header_value(trimmed, "name") {
            if let Some(name) = parse_yaml_scalar_node(name) {
                self.has_name = true;
                self.name = Some(name);
            } else {
                self.has_name = false;
                self.name = None;
            }
            return;
        }

        if let Some(version) = rime_header_value(trimmed, "version") {
            self.has_version = parse_yaml_scalar_node(version).is_some();
        }
    }

    fn finish_header(&mut self) {
        self.finish_encoder_rule();
        self.in_encoder = false;
        self.encoder_list = None;
    }

    fn is_complete(&self) -> bool {
        self.has_name && self.has_version
    }

    fn uses_preset_vocabulary(&self) -> bool {
        self.use_preset_vocabulary || self.vocabulary.is_some()
    }

    fn vocabulary_name(&self) -> &str {
        self.vocabulary
            .as_deref()
            .filter(|vocabulary| !vocabulary.is_empty())
            .unwrap_or("essay")
    }

    fn is_qualified_preset_phrase(&self, phrase: &str, weight: f32) -> bool {
        if self.max_phrase_length > 0 && phrase.chars().count() > self.max_phrase_length {
            return false;
        }
        if self.min_phrase_weight > 0.0 && weight < self.min_phrase_weight {
            return false;
        }
        true
    }

    fn parse_entry(&self, line: &str) -> Option<RimeParsedTableEntry> {
        let fields = line.split('\t').collect::<Vec<_>>();
        let text_column = self.column_index("text")?;
        let text = fields.get(text_column).copied()?;
        if text.is_empty() {
            return None;
        }

        let code = self
            .column_index("code")
            .and_then(|column| fields.get(column))
            .copied()
            .unwrap_or("");
        let weight = self
            .column_index("weight")
            .and_then(|column| fields.get(column))
            .map(|value| parse_rime_entry_weight(value))
            .unwrap_or(0.0);
        let raw_weight = self
            .column_index("weight")
            .and_then(|column| fields.get(column))
            .copied()
            .unwrap_or("")
            .to_owned();
        let raw_stem = self
            .column_index("stem")
            .and_then(|column| fields.get(column))
            .map(|value| (*value).to_owned());
        let single_syllable_duplicate_key =
            (rime_code_syllable_count(code) == 1).then(|| (text.to_owned(), code.to_owned()));
        Some(RimeParsedTableEntry {
            entry: TableEntry::new(code, text, weight),
            raw_weight,
            raw_stem,
            raw_fields: fields.into_iter().map(str::to_owned).collect(),
            single_syllable_duplicate_key,
        })
    }

    fn column_index(&self, label: &str) -> Option<usize> {
        self.columns.iter().position(|column| column == label)
    }

    fn read_header_list(&mut self, list: RimeTableHeaderList, value: &str) {
        let value = value.trim();
        let uncommented = strip_yaml_comment(value).trim();
        if uncommented.is_empty() {
            self.reset_header_list_to_null(list);
            self.reading_list = Some(list);
            self.pending_list_clear = Some(list);
            return;
        }

        if parse_yaml_scalar_node(value).is_none() {
            self.reset_header_list_to_null(list);
            self.reading_list = None;
            self.pending_list_clear = None;
            return;
        }

        if let Some(items) = parse_inline_yaml_list(value) {
            self.clear_header_list(list);
            for item in items {
                self.push_header_list_item(list, &item);
            }
        } else {
            self.clear_header_list(list);
        }
        self.reading_list = None;
        self.pending_list_clear = None;
    }

    fn clear_header_list(&mut self, list: RimeTableHeaderList) {
        match list {
            RimeTableHeaderList::Columns => self.columns.clear(),
            RimeTableHeaderList::ImportTables => self.import_tables.clear(),
        }
    }

    fn reset_header_list_to_null(&mut self, list: RimeTableHeaderList) {
        match list {
            RimeTableHeaderList::Columns => {
                self.columns = vec!["text".to_owned(), "code".to_owned(), "weight".to_owned()];
            }
            RimeTableHeaderList::ImportTables => self.import_tables.clear(),
        }
    }

    fn push_header_list_item(&mut self, list: RimeTableHeaderList, value: &str) {
        if self.pending_list_clear == Some(list) {
            self.clear_header_list(list);
            self.pending_list_clear = None;
        }
        match list {
            RimeTableHeaderList::Columns => self.columns.push(parse_yaml_scalar(value)),
            RimeTableHeaderList::ImportTables => {
                let Some(value) = parse_yaml_import_table_scalar(value) else {
                    return;
                };
                if !value.is_empty() {
                    self.import_tables.push(value);
                }
            }
        }
    }

    fn read_correction_header_line(&mut self, value: &str) {
        for (observed_input, canonical_code) in parse_lookup_pairs(value) {
            if observed_input.is_empty() || canonical_code.is_empty() {
                continue;
            }
            self.corrections
                .push(RimeCorrectionEntry::new(observed_input, canonical_code));
        }
    }

    fn read_tolerance_header_line(&mut self, value: &str) {
        for (near_code, candidate_codes) in parse_tolerance_pairs(value) {
            if near_code.is_empty() || candidate_codes.is_empty() {
                continue;
            }
            self.tolerance_rules
                .push(RimeToleranceRule::new(near_code, candidate_codes));
        }
    }

    fn read_dict_settings_header_line(&mut self, indent: usize, trimmed: &str) {
        let depth = indent / 2;
        self.dict_settings_stack.truncate(depth);

        if let Some(item) = trimmed.strip_prefix("- ") {
            let index = self
                .dict_settings_stack
                .last()
                .and_then(|key| {
                    let prefix = format!("{key}/");
                    self.dict_settings
                        .keys()
                        .filter_map(|candidate| {
                            candidate
                                .strip_prefix(&prefix)?
                                .split('/')
                                .next()?
                                .parse::<usize>()
                                .ok()
                        })
                        .max()
                })
                .map_or(0, |index| index + 1);
            self.dict_settings_stack.push(index.to_string());
            self.read_dict_settings_header_line(indent + 2, item.trim());
            return;
        }

        let Some((key, value)) = trimmed.split_once(':') else {
            return;
        };
        let key = parse_yaml_scalar(key.trim());
        if key.is_empty() {
            return;
        }
        self.dict_settings_stack.push(key);
        if let Some(value) = parse_yaml_scalar_node(value) {
            let path = self
                .dict_settings_stack
                .iter()
                .skip(1)
                .cloned()
                .collect::<Vec<_>>()
                .join("/");
            if !path.is_empty() {
                self.dict_settings.insert(path, value);
            }
            self.dict_settings_stack.pop();
        }
    }

    fn read_encoder_header_line(&mut self, trimmed: &str) {
        if let Some(exclude_patterns) = rime_header_value(trimmed, "exclude_patterns") {
            self.finish_encoder_rule();
            self.encoder_list = Some(RimeEncoderList::ExcludePatterns);
            if let Some(patterns) = parse_inline_yaml_list(exclude_patterns) {
                for pattern in patterns {
                    self.encoder
                        .add_exclude_pattern_lossy(parse_yaml_scalar(&pattern));
                }
                self.encoder_list = None;
            }
            return;
        }

        if let Some(rules) = rime_header_value(trimmed, "rules") {
            self.finish_encoder_rule();
            self.encoder_list = Some(RimeEncoderList::Rules);
            if !strip_yaml_comment(rules).trim().is_empty() {
                self.encoder_list = None;
            }
            return;
        }

        if let Some(tail_anchor) = rime_header_value(trimmed, "tail_anchor") {
            self.finish_encoder_rule();
            if let Some(tail_anchor) = parse_yaml_scalar_node(tail_anchor) {
                self.encoder.set_tail_anchor(tail_anchor);
            }
            self.encoder_list = None;
            return;
        }

        match self.encoder_list {
            Some(RimeEncoderList::ExcludePatterns) => {
                if let Some(pattern) = trimmed.strip_prefix("- ") {
                    self.encoder
                        .add_exclude_pattern_lossy(parse_yaml_scalar(pattern));
                }
            }
            Some(RimeEncoderList::Rules) => self.read_encoder_rule_line(trimmed),
            None => {}
        }
    }

    fn read_encoder_rule_line(&mut self, trimmed: &str) {
        if let Some(rule_property) = trimmed.strip_prefix("- ") {
            self.finish_encoder_rule();
            self.pending_encoder_rule = Some(RimeEncoderRuleDraft::default());
            self.read_encoder_rule_property(rule_property.trim());
            return;
        }

        if self.pending_encoder_rule.is_some() {
            self.read_encoder_rule_property(trimmed);
        }
    }

    fn read_encoder_rule_property(&mut self, trimmed: &str) {
        if trimmed.is_empty() {
            return;
        }
        if let Some(length) = rime_header_value(trimmed, "length_equal") {
            if let Some(length) = parse_yaml_usize(length) {
                if let Some(rule) = &mut self.pending_encoder_rule {
                    rule.length_equal = Some(length);
                }
            }
            return;
        }
        if let Some(range) = rime_header_value(trimmed, "length_in_range") {
            if let Some(length_range) = parse_yaml_usize_pair(range) {
                if let Some(rule) = &mut self.pending_encoder_rule {
                    rule.length_range = Some(length_range);
                }
            }
            return;
        }
        if let Some(formula) = rime_header_value(trimmed, "formula") {
            if let Some(formula) = parse_yaml_scalar_node(formula) {
                if let Some(rule) = &mut self.pending_encoder_rule {
                    rule.formula = Some(formula);
                }
            }
        }
    }

    fn finish_encoder_rule(&mut self) {
        let Some(rule) = self.pending_encoder_rule.take() else {
            return;
        };
        let Some(formula) = rule.formula else {
            return;
        };
        if let Some(length) = rule.length_equal {
            let _ = self.encoder.add_length_equal_rule(length, &formula);
        } else if let Some((min_length, max_length)) = rule.length_range {
            let _ = self
                .encoder
                .add_length_in_range_rule(min_length, max_length, &formula);
        }
    }
}

fn parse_lookup_pairs(input: &str) -> Vec<(String, String)> {
    parse_inline_yaml_list(input)
        .unwrap_or_else(|| vec![strip_yaml_comment(input).trim().to_owned()])
        .into_iter()
        .filter_map(|item| {
            let item = item.trim();
            let (left, right) = item
                .split_once("=>")
                .or_else(|| item.split_once(':'))
                .or_else(|| item.split_once('='))?;
            Some((
                parse_yaml_scalar(left.trim()),
                parse_yaml_scalar(right.trim()),
            ))
        })
        .collect()
}

fn parse_tolerance_pairs(input: &str) -> Vec<(String, Vec<String>)> {
    parse_lookup_pairs(input)
        .into_iter()
        .map(|(near_code, codes)| {
            let candidate_codes = codes
                .split('|')
                .map(str::trim)
                .filter(|code| !code.is_empty())
                .map(parse_yaml_scalar)
                .collect();
            (near_code, candidate_codes)
        })
        .collect()
}

fn parse_inline_yaml_list(input: &str) -> Option<Vec<String>> {
    let input = strip_yaml_comment(input).trim();
    input
        .strip_prefix('[')
        .and_then(|items| items.strip_suffix(']'))
        .map(|items| {
            if items.trim().is_empty() {
                return Vec::new();
            }
            split_inline_yaml_list_items(items)
        })
}

fn rime_header_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    for prefix in [key.to_owned(), format!("'{key}'"), format!("\"{key}\"")] {
        let Some(rest) = line.strip_prefix(&prefix) else {
            continue;
        };
        let rest = rest.trim_start();
        if let Some(value) = rest.strip_prefix(':') {
            return Some(value);
        }
    }
    None
}

fn strip_utf8_bom(input: &str) -> &str {
    input.strip_prefix('\u{feff}').unwrap_or(input)
}

fn split_inline_yaml_list_items(items: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut start = 0;
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;
    let mut flow_depth = 0usize;

    for (index, character) in items.char_indices() {
        match character {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote && !escaped => in_double_quote = !in_double_quote,
            '[' | '{' if !in_single_quote && !in_double_quote => flow_depth += 1,
            ']' | '}' if !in_single_quote && !in_double_quote && flow_depth > 0 => {
                flow_depth -= 1;
            }
            ',' if !in_single_quote && !in_double_quote && flow_depth == 0 => {
                result.push(items[start..index].trim().to_owned());
                start = index + character.len_utf8();
            }
            _ => {}
        }
        escaped = character == '\\' && !escaped;
    }
    result.push(items[start..].trim().to_owned());
    result
}

fn parse_yaml_scalar(input: &str) -> String {
    parse_yaml_scalar_value(strip_yaml_comment(input).trim())
}

fn parse_yaml_scalar_node(input: &str) -> Option<String> {
    let value = strip_yaml_comment(input).trim();
    if value.is_empty() {
        return None;
    }

    let is_quoted = value.starts_with('"') || value.starts_with('\'');
    if !is_quoted && (value == "~" || value.eq_ignore_ascii_case("null")) {
        return None;
    }

    Some(parse_yaml_scalar_value(value))
}

fn parse_yaml_bool(input: &str) -> Option<bool> {
    match parse_yaml_scalar_node(input)?.to_ascii_lowercase().as_str() {
        "true" | "yes" | "on" | "1" => Some(true),
        "false" | "no" | "off" | "0" => Some(false),
        _ => None,
    }
}

fn parse_yaml_usize(input: &str) -> Option<usize> {
    parse_yaml_scalar_node(input)?.parse().ok()
}

fn parse_yaml_f32(input: &str) -> Option<f32> {
    parse_yaml_scalar_node(input)?.parse().ok()
}

fn parse_yaml_usize_pair(input: &str) -> Option<(usize, usize)> {
    let items = parse_inline_yaml_list(input)?;
    if items.len() != 2 {
        return None;
    }
    Some((parse_yaml_usize(&items[0])?, parse_yaml_usize(&items[1])?))
}

fn parse_yaml_scalar_value(value: &str) -> String {
    if let Some(value) = value
        .strip_prefix('\'')
        .and_then(|value| value.strip_suffix('\''))
    {
        return value.replace("''", "'");
    }

    if let Some(value) = value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
    {
        let mut result = String::with_capacity(value.len());
        let mut escaped = false;
        let mut characters = value.chars();
        while let Some(character) = characters.next() {
            if escaped {
                match character {
                    '"' => result.push('"'),
                    '\\' => result.push('\\'),
                    '/' => result.push('/'),
                    'b' => result.push('\u{0008}'),
                    'f' => result.push('\u{000c}'),
                    'n' => result.push('\n'),
                    'r' => result.push('\r'),
                    't' => result.push('\t'),
                    'x' => {
                        if let Some(decoded) = read_yaml_hex_escape(&mut characters, 2) {
                            result.push(decoded);
                        } else {
                            result.push(character);
                        }
                    }
                    'u' => {
                        if let Some(decoded) = read_yaml_hex_escape(&mut characters, 4) {
                            result.push(decoded);
                        } else {
                            result.push(character);
                        }
                    }
                    'U' => {
                        if let Some(decoded) = read_yaml_hex_escape(&mut characters, 8) {
                            result.push(decoded);
                        } else {
                            result.push(character);
                        }
                    }
                    other => result.push(other),
                }
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else {
                result.push(character);
            }
        }
        if escaped {
            result.push('\\');
        }
        return result;
    }

    value.to_owned()
}

fn read_yaml_hex_escape(characters: &mut std::str::Chars<'_>, digits: usize) -> Option<char> {
    let mut lookahead = characters.clone();
    let mut value = 0;
    for _ in 0..digits {
        let digit = lookahead.next()?.to_digit(16)?;
        value = (value << 4) | digit;
    }
    let decoded = char::from_u32(value)?;
    *characters = lookahead;
    Some(decoded)
}

fn parse_yaml_import_table_scalar(input: &str) -> Option<String> {
    let value = strip_yaml_comment(input).trim();
    let is_quoted = value.starts_with('"') || value.starts_with('\'');
    if !is_quoted
        && ((value.starts_with('[') && value.ends_with(']'))
            || (value.starts_with('{') && value.ends_with('}')))
    {
        return None;
    }
    parse_yaml_scalar_node(input)
}

fn parse_rime_entry_weight(input: &str) -> f32 {
    let value = input.trim();
    if value.ends_with('%') {
        return 0.0;
    }

    value
        .char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(value.len()))
        .rev()
        .find_map(|end| value[..end].parse::<f32>().ok())
        .unwrap_or(0.0)
}

fn parse_rime_entry_weight_percentage(input: &str) -> f32 {
    input
        .trim()
        .strip_suffix('%')
        .map(str::trim)
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(100.0)
        / 100.0
}

fn parse_rime_preset_vocabulary(input: &str) -> HashMap<String, f32> {
    parse_rime_preset_vocabulary_entries(input)
        .into_iter()
        .collect()
}

fn parse_rime_preset_vocabulary_entries(input: &str) -> Vec<(String, f32)> {
    let mut vocabulary = Vec::new();
    let mut comments_enabled = true;
    for line in input.lines() {
        let line = line.trim_end();
        if line.is_empty() {
            continue;
        }
        if comments_enabled && line.starts_with('#') {
            if line == "# no comment" {
                comments_enabled = false;
            }
            continue;
        }

        let fields = line.split('\t').collect::<Vec<_>>();
        let Some(phrase) = fields.first().copied().filter(|phrase| !phrase.is_empty()) else {
            continue;
        };
        let weight = fields
            .get(1)
            .map(|value| parse_rime_entry_weight(value))
            .unwrap_or(0.0);
        vocabulary.push((phrase.to_owned(), weight));
    }
    vocabulary
}

fn rime_code_syllable_count(code: &str) -> usize {
    code.split(' ')
        .filter(|syllable| !syllable.is_empty())
        .count()
}

fn strip_yaml_comment(input: &str) -> &str {
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    for (index, character) in input.char_indices() {
        match character {
            '\'' if !in_double_quote => in_single_quote = !in_single_quote,
            '"' if !in_single_quote && !escaped => in_double_quote = !in_double_quote,
            '#' if !in_single_quote && !in_double_quote => {
                let starts_comment = input[..index]
                    .chars()
                    .next_back()
                    .map_or(true, char::is_whitespace);
                if starts_comment {
                    return &input[..index];
                }
            }
            _ => {}
        }
        escaped = character == '\\' && !escaped;
    }

    input
}

pub(crate) fn normalize_table_code(code: &str) -> String {
    code.split_whitespace().collect()
}
