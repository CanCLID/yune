use regex::Regex;
use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};

mod dictionary;
mod key;
mod state;
pub use dictionary::{
    parse_rime_prism_bin_metadata, parse_rime_reverse_bin_metadata, parse_rime_table_bin_metadata,
    rime_checksum_bytes, rime_dict_rebuild_plan, rime_dict_source_checksum, RimeChecksumComputer,
    RimeCompiledMetadataError, RimeDictRebuildError, RimeDictRebuildInput, RimeDictRebuildPlan,
    RimePrismBinMetadata, RimePrismChecksumMetadata, RimeReverseBinMetadata, RimeTableBinMetadata,
};
pub use key::{parse_key_sequence, KeyCode, KeyEvent, KeyModifiers, KeySequenceParseError};
pub use state::{Candidate, CandidateSource, CommitRecord, Composition, Context, Snapshot, Status};

pub trait Translator: Send + Sync {
    fn name(&self) -> &'static str;

    fn translate(&self, input: &str) -> Vec<Candidate>;

    fn translate_with_status(&self, input: &str, _status: &Status) -> Vec<Candidate> {
        self.translate(input)
    }

    fn translate_with_state(
        &self,
        input: &str,
        status: &Status,
        _options: &HashMap<String, bool>,
    ) -> Vec<Candidate> {
        self.translate_with_status(input, status)
    }

    fn translate_with_context(
        &self,
        input: &str,
        status: &Status,
        options: &HashMap<String, bool>,
        _context: &Context,
    ) -> Vec<Candidate> {
        self.translate_with_state(input, status, options)
    }
}

pub trait CandidateRanker: Send + Sync {
    fn name(&self) -> &'static str;

    fn try_rerank(&self, context: &Context, candidates: &[Candidate]) -> RerankResult;
}

pub trait CandidateFilter: Send + Sync {
    fn name(&self) -> &'static str;

    fn apply(&self, candidates: &mut Vec<Candidate>);

    fn apply_with_options(
        &self,
        candidates: &mut Vec<Candidate>,
        _options: &HashMap<String, bool>,
    ) {
        self.apply(candidates);
    }

    fn apply_with_context(
        &self,
        candidates: &mut Vec<Candidate>,
        options: &HashMap<String, bool>,
        _context: &Context,
    ) {
        self.apply_with_options(candidates, options);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RerankResult {
    Pending,
    Ready(Vec<Candidate>),
}

pub struct MockAiRanker {
    preferred_texts: Vec<String>,
}

impl MockAiRanker {
    #[must_use]
    pub fn new(preferred_texts: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            preferred_texts: preferred_texts.into_iter().map(Into::into).collect(),
        }
    }
}

impl CandidateRanker for MockAiRanker {
    fn name(&self) -> &'static str {
        "mock_ai_ranker"
    }

    fn try_rerank(&self, _context: &Context, candidates: &[Candidate]) -> RerankResult {
        if self.preferred_texts.is_empty() || candidates.is_empty() {
            return RerankResult::Pending;
        }

        let mut ranked = candidates.to_vec();
        ranked.sort_by_key(|candidate| {
            self.preferred_texts
                .iter()
                .position(|text| text == &candidate.text)
                .unwrap_or(self.preferred_texts.len())
        });
        RerankResult::Ready(ranked)
    }
}

#[derive(Default)]
pub struct EchoTranslator;

impl Translator for EchoTranslator {
    fn name(&self) -> &'static str {
        "echo_translator"
    }

    fn translate(&self, input: &str) -> Vec<Candidate> {
        if input.is_empty() {
            return Vec::new();
        }
        vec![Candidate {
            text: input.to_owned(),
            comment: "echo".to_owned(),
            source: CandidateSource::Echo,
            quality: 0.0,
        }]
    }
}

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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TableDictionary {
    entries: Vec<TableEntry>,
    stems: HashMap<String, Vec<String>>,
    encoder: TableEncoder,
}

#[derive(Clone, Debug, Default)]
pub struct TableEncoder {
    rules: Vec<TableEncodingRule>,
    exclude_pattern_sources: Vec<String>,
    exclude_patterns: Vec<Regex>,
    tail_anchor: String,
    max_phrase_length: usize,
}

impl PartialEq for TableEncoder {
    fn eq(&self, other: &Self) -> bool {
        self.rules == other.rules
            && self.exclude_pattern_sources == other.exclude_pattern_sources
            && self.tail_anchor == other.tail_anchor
            && self.max_phrase_length == other.max_phrase_length
    }
}

impl TableEncoder {
    const MAX_PHRASE_LENGTH: usize = 32;

    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn loaded(&self) -> bool {
        !self.rules.is_empty()
    }

    #[must_use]
    pub fn rules(&self) -> &[TableEncodingRule] {
        &self.rules
    }

    #[must_use]
    pub fn max_phrase_length(&self) -> usize {
        self.max_phrase_length
    }

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

    pub fn add_length_in_range_rule(
        &mut self,
        min_length: usize,
        max_length: usize,
        formula: &str,
    ) -> Result<(), TableEncoderFormulaError> {
        if min_length > max_length {
            return Err(TableEncoderFormulaError::new(
                "invalid encoder length range",
            ));
        }
        let rule = TableEncodingRule::from_formula(min_length, max_length, formula)?;
        self.max_phrase_length = self
            .max_phrase_length
            .max(max_length)
            .min(Self::MAX_PHRASE_LENGTH);
        self.rules.push(rule);
        Ok(())
    }

    pub fn set_exclude_patterns(
        &mut self,
        patterns: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<(), regex::Error> {
        let mut sources = Vec::new();
        let mut compiled = Vec::new();
        for pattern in patterns {
            let pattern = pattern.as_ref().to_owned();
            compiled.push(Regex::new(&pattern)?);
            sources.push(pattern);
        }
        self.exclude_pattern_sources = sources;
        self.exclude_patterns = compiled;
        Ok(())
    }

    fn add_exclude_pattern_lossy(&mut self, pattern: impl Into<String>) {
        let pattern = pattern.into();
        let Ok(compiled) = Regex::new(&pattern) else {
            return;
        };
        self.exclude_pattern_sources.push(pattern);
        self.exclude_patterns.push(compiled);
    }

    pub fn set_tail_anchor(&mut self, tail_anchor: impl Into<String>) {
        self.tail_anchor = tail_anchor.into();
    }

    #[must_use]
    pub fn is_code_excluded(&self, code: &str) -> bool {
        self.exclude_patterns.iter().any(|pattern| {
            pattern
                .find(code)
                .is_some_and(|matched| matched.start() == 0 && matched.end() == code.len())
        })
    }

    #[must_use]
    pub fn encode(&self, raw_code: &[impl AsRef<str>]) -> Option<String> {
        let num_syllables = raw_code.len();
        for rule in &self.rules {
            if num_syllables < rule.min_word_length || num_syllables > rule.max_word_length {
                continue;
            }

            let mut encoded = String::new();
            let mut previous = CodeCoords::default();
            let mut current_encoded = CodeCoords::default();
            for original in &rule.coords {
                let mut coords = *original;
                if coords.char_index < 0 {
                    coords.char_index += num_syllables as isize;
                }
                if coords.char_index >= num_syllables as isize || coords.char_index < 0 {
                    continue;
                }
                if original.char_index < 0 && coords.char_index < current_encoded.char_index {
                    continue;
                }

                let start_index = if coords.char_index == current_encoded.char_index {
                    current_encoded.code_index + 1
                } else {
                    0
                };
                let code = raw_code[coords.char_index as usize].as_ref();
                coords.code_index = self.calculate_code_index(code, coords.code_index, start_index);
                if coords.code_index >= code.len() as isize || coords.code_index < 0 {
                    continue;
                }
                if (original.char_index < 0 || original.code_index < 0)
                    && coords.char_index == current_encoded.char_index
                    && coords.code_index <= current_encoded.code_index
                    && (original.char_index != previous.char_index
                        || original.code_index != previous.code_index)
                {
                    continue;
                }

                encoded.push(code.as_bytes()[coords.code_index as usize] as char);
                previous = *original;
                current_encoded = coords;
            }
            if !encoded.is_empty() {
                return Some(encoded);
            }
        }
        None
    }

    fn calculate_code_index(&self, code: &str, mut index: isize, start: isize) -> isize {
        let bytes = code.as_bytes();
        let tail_anchor = self.tail_anchor.as_bytes();
        let mut byte_index = 0;
        if index < 0 {
            byte_index = bytes.len() as isize - 1;
            if let Some(tail) = bytes
                .iter()
                .enumerate()
                .skip((start + 1).max(0) as usize)
                .find_map(|(tail_index, byte)| tail_anchor.contains(byte).then_some(tail_index))
            {
                byte_index = tail as isize - 1;
            }
            while {
                index += 1;
                index < 0
            } {
                loop {
                    byte_index -= 1;
                    if byte_index < 0 || !tail_anchor.contains(&bytes[byte_index as usize]) {
                        break;
                    }
                }
            }
        } else {
            while index > 0 {
                index -= 1;
                loop {
                    byte_index += 1;
                    if byte_index >= bytes.len() as isize
                        || !tail_anchor.contains(&bytes[byte_index as usize])
                    {
                        break;
                    }
                }
            }
        }
        byte_index
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableEncodingRule {
    pub min_word_length: usize,
    pub max_word_length: usize,
    pub coords: Vec<CodeCoords>,
}

impl TableEncodingRule {
    fn from_formula(
        min_word_length: usize,
        max_word_length: usize,
        formula: &str,
    ) -> Result<Self, TableEncoderFormulaError> {
        if formula.len() % 2 != 0 {
            return Err(TableEncoderFormulaError::new(
                "encoder formula length is odd",
            ));
        }
        let mut coords = Vec::new();
        for pair in formula.as_bytes().chunks_exact(2) {
            let char_index = parse_encoder_formula_index(pair[0], b'A', b'Z')
                .ok_or_else(|| TableEncoderFormulaError::new("invalid character index"))?;
            let code_index = parse_encoder_formula_index(pair[1], b'a', b'z')
                .ok_or_else(|| TableEncoderFormulaError::new("invalid code index"))?;
            coords.push(CodeCoords {
                char_index,
                code_index,
            });
        }
        Ok(Self {
            min_word_length,
            max_word_length,
            coords,
        })
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CodeCoords {
    pub char_index: isize,
    pub code_index: isize,
}

fn parse_encoder_formula_index(byte: u8, lower: u8, upper: u8) -> Option<isize> {
    if !(lower..=upper).contains(&byte) {
        return None;
    }
    Some(if byte >= lower + 20 {
        byte as isize - upper as isize - 1
    } else {
        byte as isize - lower as isize
    })
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TableEncoderFormulaError {
    message: String,
}

impl TableEncoderFormulaError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for TableEncoderFormulaError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TableEncoderFormulaError {}

impl TableDictionary {
    #[must_use]
    pub fn new(entries: impl IntoIterator<Item = TableEntry>) -> Self {
        Self {
            entries: entries.into_iter().collect(),
            stems: HashMap::new(),
            encoder: TableEncoder::new(),
        }
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
    pub fn encoder(&self) -> &TableEncoder {
        &self.encoder
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
    dedupe_rime_table_entries(&mut entries);
    let mut entries = entries
        .into_iter()
        .map(|entry| entry.entry)
        .collect::<Vec<_>>();
    sort_rime_table_entries(metadata, &mut entries);
    TableDictionary {
        entries,
        stems,
        encoder: metadata.encoder.clone(),
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

fn normalize_table_code(code: &str) -> String {
    code.split_whitespace().collect()
}

pub struct StaticTableTranslator {
    entries: Vec<(String, Candidate)>,
    enable_completion: bool,
    enable_charset_filter: bool,
    enable_sentence: bool,
    sentence_over_completion: bool,
    tags: Vec<String>,
    delimiters: String,
    initial_quality: f32,
    comment_format: CommentFormat,
    dictionary_exclude: HashSet<String>,
}

impl StaticTableTranslator {
    #[must_use]
    pub fn new(entries: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) -> Self {
        let entries = entries
            .into_iter()
            .map(|(code, text)| {
                let code = code.into();
                let text = text.into();
                (
                    code.clone(),
                    Candidate {
                        text,
                        comment: code,
                        source: CandidateSource::Table,
                        quality: 0.0,
                    },
                )
            })
            .collect();
        Self {
            entries,
            enable_completion: false,
            enable_charset_filter: false,
            enable_sentence: false,
            sentence_over_completion: false,
            tags: vec!["abc".to_owned()],
            delimiters: " ".to_owned(),
            initial_quality: 0.0,
            comment_format: CommentFormat::default(),
            dictionary_exclude: HashSet::new(),
        }
    }

    #[must_use]
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
        Self {
            entries,
            enable_completion: false,
            enable_charset_filter: false,
            enable_sentence: false,
            sentence_over_completion: false,
            tags: vec!["abc".to_owned()],
            delimiters: " ".to_owned(),
            initial_quality: 0.0,
            comment_format: CommentFormat::default(),
            dictionary_exclude: HashSet::new(),
        }
    }

    #[must_use]
    pub fn with_completion(mut self, enable_completion: bool) -> Self {
        self.enable_completion = enable_completion;
        self
    }

    #[must_use]
    pub fn with_charset_filter(mut self, enable_charset_filter: bool) -> Self {
        self.enable_charset_filter = enable_charset_filter;
        self
    }

    #[must_use]
    pub fn with_sentence(mut self, enable_sentence: bool) -> Self {
        self.enable_sentence = enable_sentence;
        self
    }

    #[must_use]
    pub fn with_sentence_over_completion(mut self, sentence_over_completion: bool) -> Self {
        self.sentence_over_completion = sentence_over_completion;
        self
    }

    #[must_use]
    pub fn with_delimiters(mut self, delimiters: impl Into<String>) -> Self {
        self.delimiters = delimiters.into();
        if self.delimiters.is_empty() {
            self.delimiters = " ".to_owned();
        }
        self
    }

    #[must_use]
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        if self.tags.is_empty() {
            self.tags.push("abc".to_owned());
        }
        self
    }

    #[must_use]
    pub fn with_initial_quality(mut self, initial_quality: f32) -> Self {
        self.initial_quality = initial_quality;
        self
    }

    #[must_use]
    pub fn with_comment_format(mut self, formulas: &[String]) -> Self {
        self.comment_format = CommentFormat::parse(formulas);
        self
    }

    #[must_use]
    pub fn with_dictionary_exclude(
        mut self,
        words: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.dictionary_exclude = words.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub fn with_spelling_algebra(mut self, formulas: &[String]) -> Self {
        let algebra = SpellingAlgebra::parse(formulas);
        if !algebra.is_empty() {
            self.entries = algebra.expand_entries(self.entries);
        }
        self
    }

    fn lookup_code<'a>(&self, input: &'a str) -> &'a str {
        input.trim_end_matches(|ch| self.delimiters.contains(ch))
    }

    fn accepts_default_segment(&self) -> bool {
        self.tags.iter().any(|tag| tag == "abc")
    }

    fn accepts_segment_tags(&self, segment_tags: &[String]) -> bool {
        self.tags
            .iter()
            .any(|tag| segment_tags.iter().any(|segment_tag| segment_tag == tag))
    }

    fn matches_lookup_code(&self, entry_code: &str, lookup_code: &str) -> bool {
        entry_code == lookup_code
            || (self.enable_completion
                && !lookup_code.is_empty()
                && entry_code.starts_with(lookup_code))
    }

    fn is_dictionary_word_allowed(&self, candidate: &Candidate) -> bool {
        !self.dictionary_exclude.contains(&candidate.text)
    }

    fn candidate_for_lookup(
        &self,
        entry_code: &str,
        candidate: &Candidate,
        lookup_code: &str,
    ) -> Candidate {
        let mut candidate = candidate.clone();
        candidate.comment = self.comment_format.apply(&candidate.comment);
        candidate.quality = candidate.quality.exp() + self.initial_quality;
        if entry_code != lookup_code {
            candidate.source = CandidateSource::Completion;
            candidate.quality -= 1.0;
        }
        candidate
    }

    fn translated_candidates(&self, input: &str, filter_by_charset: bool) -> Vec<Candidate> {
        self.translated_candidates_for_segment(input, filter_by_charset, None)
    }

    fn translated_candidates_for_segment(
        &self,
        input: &str,
        filter_by_charset: bool,
        segment_tags: Option<&[String]>,
    ) -> Vec<Candidate> {
        let accepts_segment = segment_tags
            .map(|tags| self.accepts_segment_tags(tags))
            .unwrap_or_else(|| self.accepts_default_segment());
        if !accepts_segment {
            return Vec::new();
        }

        let lookup_code = self.lookup_code(input);
        let mut candidates = self
            .entries
            .iter()
            .filter(|(entry_code, candidate)| {
                self.matches_lookup_code(entry_code, lookup_code)
                    && self.is_dictionary_word_allowed(candidate)
                    && (!filter_by_charset || !contains_extended_cjk(&candidate.text))
            })
            .map(|(entry_code, candidate)| {
                self.candidate_for_lookup(entry_code, candidate, lookup_code)
            })
            .collect::<Vec<_>>();

        if candidates.is_empty() && self.enable_sentence {
            if let Some(sentence) = self.sentence_candidate(input, filter_by_charset, None) {
                candidates.push(sentence);
            }
        } else if self.sentence_over_completion
            && candidates
                .first()
                .is_some_and(|candidate| candidate.source == CandidateSource::Completion)
        {
            let priority_floor = candidates
                .iter()
                .map(|candidate| candidate.quality)
                .max_by(|left, right| left.partial_cmp(right).unwrap_or(Ordering::Equal));
            if let Some(sentence) =
                self.sentence_candidate(input, filter_by_charset, priority_floor)
            {
                candidates.push(sentence);
            }
        }

        candidates
    }

    fn sentence_candidate(
        &self,
        input: &str,
        filter_by_charset: bool,
        priority_floor: Option<f32>,
    ) -> Option<Candidate> {
        if input.is_empty() {
            return None;
        }

        #[derive(Clone)]
        struct SentencePath {
            quality: f32,
            pieces: Vec<String>,
        }

        let mut paths: Vec<Option<SentencePath>> = vec![None; input.len() + 1];
        paths[0] = Some(SentencePath {
            quality: 0.0,
            pieces: Vec::new(),
        });
        for pos in input
            .char_indices()
            .map(|(index, _)| index)
            .chain(std::iter::once(input.len()))
        {
            let Some(path) = paths.get(pos).and_then(Clone::clone) else {
                continue;
            };
            let active_input = &input[pos..];
            for (entry_code, candidate) in &self.entries {
                if entry_code.is_empty()
                    || !active_input.starts_with(entry_code)
                    || !self.is_dictionary_word_allowed(candidate)
                    || (filter_by_charset && contains_extended_cjk(&candidate.text))
                {
                    continue;
                }
                let mut end_pos = pos + entry_code.len();
                while end_pos < input.len() {
                    let Some(ch) = input[end_pos..].chars().next() else {
                        break;
                    };
                    if !self.delimiters.contains(ch) {
                        break;
                    }
                    end_pos += ch.len_utf8();
                }
                let mut next_path = path.clone();
                next_path.quality += candidate.quality.exp();
                next_path.pieces.push(candidate.text.clone());
                let replace = match paths[end_pos].as_ref() {
                    Some(existing) => next_path.quality > existing.quality,
                    None => true,
                };
                if replace {
                    paths[end_pos] = Some(next_path);
                }
            }
        }

        let path = paths[input.len()].take()?;
        if path.pieces.len() <= 1 {
            return None;
        }
        let quality = priority_floor
            .map(|floor| floor + 1.0)
            .unwrap_or(path.quality + self.initial_quality);
        Some(Candidate {
            text: path.pieces.join(""),
            comment: " ☯ ".to_owned(),
            source: CandidateSource::Sentence,
            quality,
        })
    }

    pub fn parse_rime_dict_yaml(input: &str) -> Result<Self, TableDictionaryParseError> {
        TableDictionary::parse_rime_dict_yaml(input).map(Self::from_dictionary)
    }

    pub fn parse_rime_dict_yaml_with_imports(
        input: &str,
        import_loader: impl FnMut(&str) -> Option<String>,
    ) -> Result<Self, TableDictionaryParseError> {
        TableDictionary::parse_rime_dict_yaml_with_imports(input, import_loader)
            .map(Self::from_dictionary)
    }

    pub fn parse_rime_dict_yaml_with_imports_and_packs(
        input: &str,
        packs: impl IntoIterator<Item = impl AsRef<str>>,
        import_loader: impl FnMut(&str) -> Option<String>,
    ) -> Result<Self, TableDictionaryParseError> {
        TableDictionary::parse_rime_dict_yaml_with_imports_and_packs(input, packs, import_loader)
            .map(Self::from_dictionary)
    }

    pub fn parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
        input: &str,
        packs: impl IntoIterator<Item = impl AsRef<str>>,
        import_loader: impl FnMut(&str) -> Option<String>,
        vocabulary_loader: impl FnMut(&str) -> Option<String>,
    ) -> Result<Self, TableDictionaryParseError> {
        TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
            input,
            packs,
            import_loader,
            vocabulary_loader,
        )
        .map(Self::from_dictionary)
    }
}

impl Translator for StaticTableTranslator {
    fn name(&self) -> &'static str {
        "static_table_translator"
    }

    fn translate(&self, input: &str) -> Vec<Candidate> {
        self.translated_candidates(input, false)
    }

    fn translate_with_state(
        &self,
        input: &str,
        _status: &Status,
        options: &HashMap<String, bool>,
    ) -> Vec<Candidate> {
        let filter_by_charset = self.enable_charset_filter
            && !options.get("extended_charset").copied().unwrap_or(false);
        self.translated_candidates(input, filter_by_charset)
    }

    fn translate_with_context(
        &self,
        input: &str,
        _status: &Status,
        options: &HashMap<String, bool>,
        context: &Context,
    ) -> Vec<Candidate> {
        let filter_by_charset = self.enable_charset_filter
            && !options.get("extended_charset").copied().unwrap_or(false);
        self.translated_candidates_for_segment(
            input,
            filter_by_charset,
            Some(&context.segment_tags),
        )
    }
}

pub struct ReverseLookupTranslator {
    entries: Vec<TableEntry>,
    reverse_comments: HashMap<String, Vec<String>>,
    prefix: String,
    suffix: String,
    tag: String,
    enable_completion: bool,
    comment_format: CommentFormat,
}

impl ReverseLookupTranslator {
    #[must_use]
    pub fn new(
        dictionary: TableDictionary,
        reverse_dictionary: Option<TableDictionary>,
        prefix: impl Into<String>,
        suffix: impl Into<String>,
    ) -> Self {
        let mut reverse_comments: HashMap<String, Vec<String>> = HashMap::new();
        if let Some(reverse_dictionary) = reverse_dictionary {
            for entry in reverse_dictionary.entries {
                reverse_comments
                    .entry(entry.text)
                    .or_default()
                    .push(entry.code);
            }
        }

        Self {
            entries: dictionary.entries,
            reverse_comments,
            prefix: prefix.into(),
            suffix: suffix.into(),
            tag: "reverse_lookup".to_owned(),
            enable_completion: false,
            comment_format: CommentFormat::default(),
        }
    }

    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = tag.into();
        self
    }

    #[must_use]
    pub fn with_completion(mut self, enable_completion: bool) -> Self {
        self.enable_completion = enable_completion;
        self
    }

    #[must_use]
    pub fn with_comment_format(mut self, formulas: &[String]) -> Self {
        self.comment_format = CommentFormat::parse(formulas);
        self
    }

    fn accepts_segment_tags(&self, segment_tags: &[String]) -> bool {
        segment_tags
            .iter()
            .any(|segment_tag| segment_tag == &self.tag)
    }
}

impl Translator for ReverseLookupTranslator {
    fn name(&self) -> &'static str {
        "reverse_lookup_translator"
    }

    fn translate(&self, input: &str) -> Vec<Candidate> {
        if input.is_empty() {
            return Vec::new();
        }

        let start = if !self.prefix.is_empty() && input.starts_with(&self.prefix) {
            self.prefix.len()
        } else {
            0
        };
        let mut code = &input[start..];
        if !self.suffix.is_empty() && code.ends_with(&self.suffix) {
            code = &code[..code.len() - self.suffix.len()];
        }
        let code = normalize_table_code(code);
        if code.is_empty() {
            return Vec::new();
        }

        self.entries
            .iter()
            .filter(|entry| {
                if self.enable_completion {
                    entry.code.starts_with(&code)
                } else {
                    entry.code == code
                }
            })
            .map(|entry| {
                let comment = self
                    .reverse_comments
                    .get(&entry.text)
                    .filter(|comments| !comments.is_empty())
                    .map(|comments| self.comment_format.apply(&comments.join(" ")))
                    .unwrap_or_else(|| entry.code.clone());
                Candidate {
                    text: entry.text.clone(),
                    comment,
                    source: CandidateSource::ReverseLookup,
                    quality: entry.weight,
                }
            })
            .collect()
    }

    fn translate_with_context(
        &self,
        input: &str,
        _status: &Status,
        _options: &HashMap<String, bool>,
        context: &Context,
    ) -> Vec<Candidate> {
        if !self.accepts_segment_tags(&context.segment_tags) {
            return Vec::new();
        }
        self.translate(input)
    }
}

pub struct HistoryTranslator {
    input: String,
    size: usize,
    initial_quality: f32,
    tag: String,
}

impl HistoryTranslator {
    #[must_use]
    pub fn new(input: impl Into<String>) -> Self {
        Self {
            input: input.into(),
            size: 1,
            initial_quality: 1000.0,
            tag: "abc".to_owned(),
        }
    }

    #[must_use]
    pub const fn with_size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    #[must_use]
    pub const fn with_initial_quality(mut self, initial_quality: f32) -> Self {
        self.initial_quality = initial_quality;
        self
    }

    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tag = tag.into();
        if self.tag.is_empty() {
            self.tag = "abc".to_owned();
        }
        self
    }

    fn accepts_segment_tags(&self, segment_tags: &[String]) -> bool {
        segment_tags
            .iter()
            .any(|segment_tag| segment_tag == &self.tag)
    }
}

impl Translator for HistoryTranslator {
    fn name(&self) -> &'static str {
        "history_translator"
    }

    fn translate(&self, _input: &str) -> Vec<Candidate> {
        Vec::new()
    }

    fn translate_with_context(
        &self,
        input: &str,
        _status: &Status,
        _options: &HashMap<String, bool>,
        context: &Context,
    ) -> Vec<Candidate> {
        if !self.accepts_segment_tags(&context.segment_tags)
            || self.input.is_empty()
            || self.input != input
        {
            return Vec::new();
        }

        context
            .commit_history
            .iter()
            .rev()
            .filter(|record| record.candidate_type != "thru")
            .take(self.size)
            .map(|record| Candidate {
                text: record.text.clone(),
                comment: String::new(),
                source: CandidateSource::History,
                quality: self.initial_quality,
            })
            .collect()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SwitchTranslatorSwitch {
    Toggle {
        option_name: String,
        states: [String; 2],
        abbrev: [Option<String>; 2],
    },
    Radio {
        options: Vec<String>,
        states: Vec<String>,
        abbrev: Vec<Option<String>>,
    },
}

impl SwitchTranslatorSwitch {
    #[must_use]
    pub fn toggle(
        option_name: impl Into<String>,
        state0: impl Into<String>,
        state1: impl Into<String>,
    ) -> Self {
        Self::Toggle {
            option_name: option_name.into(),
            states: [state0.into(), state1.into()],
            abbrev: [None, None],
        }
    }

    #[must_use]
    pub fn radio(
        options: impl IntoIterator<Item = impl Into<String>>,
        states: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self::Radio {
            options: options.into_iter().map(Into::into).collect(),
            states: states.into_iter().map(Into::into).collect(),
            abbrev: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_abbrev(
        mut self,
        abbrev: impl IntoIterator<Item = Option<impl Into<String>>>,
    ) -> Self {
        match &mut self {
            Self::Toggle { abbrev: values, .. } => {
                for (index, value) in abbrev.into_iter().take(2).enumerate() {
                    values[index] = value.map(Into::into);
                }
            }
            Self::Radio { abbrev: values, .. } => {
                *values = abbrev
                    .into_iter()
                    .map(|value| value.map(Into::into))
                    .collect();
            }
        }
        self
    }
}

pub struct SwitchTranslator {
    switches: Vec<SwitchTranslatorSwitch>,
    folded_options: FoldedSwitchOptions,
}

impl SwitchTranslator {
    #[must_use]
    pub fn new(switches: impl IntoIterator<Item = SwitchTranslatorSwitch>) -> Self {
        Self {
            switches: switches.into_iter().collect(),
            folded_options: FoldedSwitchOptions::default(),
        }
    }

    #[must_use]
    pub fn with_folded_options(mut self, folded_options: FoldedSwitchOptions) -> Self {
        self.folded_options = folded_options;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FoldedSwitchOptions {
    pub prefix: String,
    pub suffix: String,
    pub separator: String,
    pub abbreviate_options: bool,
}

impl Default for FoldedSwitchOptions {
    fn default() -> Self {
        Self {
            prefix: String::new(),
            suffix: String::new(),
            separator: " ".to_owned(),
            abbreviate_options: false,
        }
    }
}

impl Translator for SwitchTranslator {
    fn name(&self) -> &'static str {
        "switch_translator"
    }

    fn translate(&self, _input: &str) -> Vec<Candidate> {
        Vec::new()
    }

    fn translate_with_state(
        &self,
        input: &str,
        _status: &Status,
        runtime_options: &HashMap<String, bool>,
    ) -> Vec<Candidate> {
        if input.is_empty() {
            return Vec::new();
        }

        let mut candidates = Vec::new();
        for the_switch in &self.switches {
            match the_switch {
                SwitchTranslatorSwitch::Toggle {
                    option_name,
                    states,
                    ..
                } => {
                    let current_state = runtime_options.get(option_name).copied().unwrap_or(false);
                    let current_index = usize::from(current_state);
                    candidates.push(Candidate {
                        text: states[current_index].clone(),
                        comment: format!("→ {}", states[1 - current_index]),
                        source: CandidateSource::Switch,
                        quality: 0.5,
                    });
                }
                SwitchTranslatorSwitch::Radio {
                    options, states, ..
                } => {
                    if options.is_empty() || states.is_empty() {
                        continue;
                    }
                    let selected_index = options
                        .iter()
                        .position(|option| options_get_bool(runtime_options, option))
                        .unwrap_or(0);
                    for (option_index, state) in states.iter().enumerate().take(options.len()) {
                        if state.is_empty() {
                            continue;
                        }
                        candidates.push(Candidate {
                            text: state.clone(),
                            comment: if option_index == selected_index {
                                " ✓".to_owned()
                            } else {
                                String::new()
                            },
                            source: CandidateSource::Switch,
                            quality: 0.5,
                        });
                    }
                }
            }
        }
        if options_get_bool(runtime_options, "_fold_options") {
            let labels = self.folded_option_labels(runtime_options);
            if labels.len() > 1 {
                return vec![Candidate {
                    text: format!(
                        "{}{}{}",
                        self.folded_options.prefix,
                        labels.join(&self.folded_options.separator),
                        self.folded_options.suffix
                    ),
                    comment: String::new(),
                    source: CandidateSource::Unfold,
                    quality: 0.5,
                }];
            }
        }
        candidates
    }
}

impl SwitchTranslator {
    fn folded_option_labels(&self, runtime_options: &HashMap<String, bool>) -> Vec<String> {
        let mut labels = Vec::new();
        for the_switch in &self.switches {
            match the_switch {
                SwitchTranslatorSwitch::Toggle {
                    option_name,
                    states,
                    abbrev,
                } => {
                    let current_state =
                        usize::from(runtime_options.get(option_name).copied().unwrap_or(false));
                    if !states
                        .get(current_state)
                        .is_some_and(|state| !state.is_empty())
                    {
                        continue;
                    }
                    labels.push(folded_state_label(
                        &states[current_state],
                        abbrev.get(current_state).and_then(Option::as_deref),
                        self.folded_options.abbreviate_options,
                    ));
                }
                SwitchTranslatorSwitch::Radio {
                    options,
                    states,
                    abbrev,
                } => {
                    let selected_index = options
                        .iter()
                        .position(|option| options_get_bool(runtime_options, option))
                        .unwrap_or(0);
                    if !states
                        .get(selected_index)
                        .is_some_and(|state| !state.is_empty())
                    {
                        continue;
                    }
                    labels.push(folded_state_label(
                        &states[selected_index],
                        abbrev.get(selected_index).and_then(Option::as_deref),
                        self.folded_options.abbreviate_options,
                    ));
                }
            }
        }
        labels
    }
}

pub struct SchemaListTranslator {
    entries: Vec<(String, String)>,
}

impl SchemaListTranslator {
    #[must_use]
    pub fn new(entries: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) -> Self {
        Self {
            entries: entries
                .into_iter()
                .map(|(schema_id, schema_name)| (schema_id.into(), schema_name.into()))
                .collect(),
        }
    }
}

impl Translator for SchemaListTranslator {
    fn name(&self) -> &'static str {
        "schema_list_translator"
    }

    fn translate(&self, _input: &str) -> Vec<Candidate> {
        Vec::new()
    }

    fn translate_with_status(&self, input: &str, status: &Status) -> Vec<Candidate> {
        if input.is_empty() {
            return Vec::new();
        }

        let mut candidates = vec![Candidate {
            text: status.schema_name.clone(),
            comment: String::new(),
            source: CandidateSource::Schema,
            quality: 0.5,
        }];
        candidates.extend(
            self.entries
                .iter()
                .filter(|(schema_id, _)| schema_id != &status.schema_id)
                .map(|(_, schema_name)| Candidate {
                    text: schema_name.clone(),
                    comment: String::new(),
                    source: CandidateSource::Schema,
                    quality: 0.5,
                }),
        );
        candidates
    }
}

fn folded_state_label(state: &str, abbrev: Option<&str>, abbreviate: bool) -> String {
    if !abbreviate {
        return state.to_owned();
    }
    if let Some(abbrev) = abbrev {
        return abbrev.to_owned();
    }
    state.chars().next().into_iter().collect()
}

fn options_get_bool(options: &HashMap<String, bool>, option: &str) -> bool {
    options.get(option).copied().unwrap_or(false)
}

pub struct UniquifierFilter;

impl CandidateFilter for UniquifierFilter {
    fn name(&self) -> &'static str {
        "uniquifier"
    }

    fn apply(&self, candidates: &mut Vec<Candidate>) {
        let mut seen = HashSet::new();
        candidates.retain(|candidate| seen.insert(candidate.text.clone()));
    }
}

pub struct SingleCharFilter;

impl CandidateFilter for SingleCharFilter {
    fn name(&self) -> &'static str {
        "single_char_filter"
    }

    fn apply(&self, candidates: &mut Vec<Candidate>) {
        let table_prefix_len = candidates
            .iter()
            .position(|candidate| candidate.source != CandidateSource::Table)
            .unwrap_or(candidates.len());
        if table_prefix_len <= 1 {
            return;
        }

        let mut phrases = candidates.drain(..table_prefix_len).collect::<Vec<_>>();
        let mut single_chars = Vec::new();
        let mut multi_chars = Vec::new();
        for candidate in phrases.drain(..) {
            if candidate.text.chars().count() == 1 {
                single_chars.push(candidate);
            } else {
                multi_chars.push(candidate);
            }
        }
        single_chars.append(&mut multi_chars);
        candidates.splice(..0, single_chars);
    }
}

pub struct CharsetFilter;

impl CandidateFilter for CharsetFilter {
    fn name(&self) -> &'static str {
        "charset_filter"
    }

    fn apply(&self, candidates: &mut Vec<Candidate>) {
        candidates.retain(|candidate| !contains_extended_cjk(&candidate.text));
    }

    fn apply_with_options(&self, candidates: &mut Vec<Candidate>, options: &HashMap<String, bool>) {
        if !options.get("extended_charset").copied().unwrap_or(false) {
            self.apply(candidates);
        }
    }
}

pub struct TaggedFilter {
    filter: Box<dyn CandidateFilter>,
    tags: Vec<String>,
}

impl TaggedFilter {
    #[must_use]
    pub fn new(
        filter: impl CandidateFilter + 'static,
        tags: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            filter: Box::new(filter),
            tags: tags.into_iter().map(Into::into).collect(),
        }
    }

    fn accepts_segment_tags(&self, segment_tags: &[String]) -> bool {
        self.tags.is_empty()
            || self
                .tags
                .iter()
                .any(|tag| segment_tags.iter().any(|segment_tag| segment_tag == tag))
    }
}

impl CandidateFilter for TaggedFilter {
    fn name(&self) -> &'static str {
        self.filter.name()
    }

    fn apply(&self, candidates: &mut Vec<Candidate>) {
        self.filter.apply(candidates);
    }

    fn apply_with_options(&self, candidates: &mut Vec<Candidate>, options: &HashMap<String, bool>) {
        self.filter.apply_with_options(candidates, options);
    }

    fn apply_with_context(
        &self,
        candidates: &mut Vec<Candidate>,
        options: &HashMap<String, bool>,
        context: &Context,
    ) {
        if self.accepts_segment_tags(&context.segment_tags) {
            self.filter.apply_with_context(candidates, options, context);
        }
    }
}

fn contains_extended_cjk(text: &str) -> bool {
    text.chars().any(is_extended_cjk)
}

fn is_extended_cjk(ch: char) -> bool {
    matches!(
        ch as u32,
        0x3400..=0x4dbf
            | 0x20000..=0x2a6df
            | 0x2a700..=0x2b73f
            | 0x2b740..=0x2b81f
            | 0x2b820..=0x2ceaf
            | 0x2ceb0..=0x2ebef
            | 0x30000..=0x3134f
            | 0x31350..=0x323af
            | 0x2ebf0..=0x2ee5f
            | 0x323b0..=0x3347f
            | 0x3300..=0x33ff
            | 0xfe30..=0xfe4f
            | 0xf900..=0xfaff
            | 0x2f800..=0x2fa1f
    )
}

pub struct SimplifierFilter {
    option_name: String,
    conversion: SimplifierConversion,
    tips_level: SimplifierTipsLevel,
    show_in_comment: bool,
    inherit_comment: bool,
    comment_format: CommentFormat,
    excluded_types: HashSet<String>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SimplifierConversion {
    None,
    TraditionalToSimplified,
    SimplifiedToTraditional,
    TraditionalToTaiwan,
    SimplifiedToTaiwan,
    TaiwanToSimplified,
    TaiwanToTraditional,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SimplifierTipsLevel {
    None,
    Char,
    All,
}

impl Default for SimplifierFilter {
    fn default() -> Self {
        Self::new()
    }
}

impl SimplifierFilter {
    #[must_use]
    pub fn new() -> Self {
        Self {
            option_name: "simplification".to_owned(),
            conversion: SimplifierConversion::TraditionalToSimplified,
            tips_level: SimplifierTipsLevel::None,
            show_in_comment: false,
            inherit_comment: true,
            comment_format: CommentFormat::default(),
            excluded_types: HashSet::new(),
        }
    }

    #[must_use]
    pub fn with_option_name(mut self, option_name: impl Into<String>) -> Self {
        let option_name = option_name.into();
        if !option_name.is_empty() {
            self.option_name = option_name;
        }
        self
    }

    #[must_use]
    pub fn with_opencc_config(mut self, opencc_config: impl AsRef<str>) -> Self {
        self.conversion = SimplifierConversion::from_opencc_config(opencc_config.as_ref());
        self
    }

    #[must_use]
    pub fn with_tips(mut self, tips: impl AsRef<str>) -> Self {
        self.tips_level = match tips.as_ref() {
            "char" => SimplifierTipsLevel::Char,
            "all" => SimplifierTipsLevel::All,
            _ => SimplifierTipsLevel::None,
        };
        self
    }

    #[must_use]
    pub fn with_show_in_comment(mut self, show_in_comment: bool) -> Self {
        self.show_in_comment = show_in_comment;
        self
    }

    #[must_use]
    pub fn with_inherit_comment(mut self, inherit_comment: bool) -> Self {
        self.inherit_comment = inherit_comment;
        self
    }

    #[must_use]
    pub fn with_comment_format(mut self, formulas: &[String]) -> Self {
        self.comment_format = CommentFormat::parse(formulas);
        self
    }

    #[must_use]
    pub fn with_excluded_types(mut self, excluded_types: impl IntoIterator<Item = String>) -> Self {
        self.excluded_types = excluded_types
            .into_iter()
            .filter(|candidate_type| !candidate_type.is_empty())
            .collect();
        self
    }
}

impl SimplifierConversion {
    fn from_opencc_config(opencc_config: &str) -> Self {
        let config_name = opencc_config
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or(opencc_config)
            .to_ascii_lowercase();
        let config_stem = config_name.strip_suffix(".json").unwrap_or(&config_name);
        match config_stem {
            "" | "t2s" | "hk2s" => Self::TraditionalToSimplified,
            "s2t" => Self::SimplifiedToTraditional,
            "t2tw" => Self::TraditionalToTaiwan,
            "s2tw" => Self::SimplifiedToTaiwan,
            "tw2s" => Self::TaiwanToSimplified,
            "tw2t" => Self::TaiwanToTraditional,
            _ if config_stem.ends_with(".ini") => Self::None,
            _ => Self::None,
        }
    }

    fn convert(self, text: &str) -> String {
        match self {
            Self::None => text.to_owned(),
            Self::TraditionalToSimplified => simplify_traditional_text(text),
            Self::SimplifiedToTraditional => traditionalize_simplified_text(text),
            Self::TraditionalToTaiwan => traditional_to_taiwan_text(text),
            Self::SimplifiedToTaiwan => {
                traditional_to_taiwan_text(&traditionalize_simplified_text(text))
            }
            Self::TaiwanToSimplified => {
                simplify_traditional_text(&taiwan_to_traditional_text(text))
            }
            Self::TaiwanToTraditional => taiwan_to_traditional_text(text),
        }
    }
}

impl CandidateFilter for SimplifierFilter {
    fn name(&self) -> &'static str {
        "simplifier"
    }

    fn apply(&self, _candidates: &mut Vec<Candidate>) {}

    fn apply_with_options(&self, candidates: &mut Vec<Candidate>, options: &HashMap<String, bool>) {
        if !options.get(&self.option_name).copied().unwrap_or(false) {
            return;
        }

        for candidate in candidates {
            if self.excluded_types.contains(candidate.source.as_str()) {
                continue;
            }

            let original = candidate.text.clone();
            let simplified = self.conversion.convert(&original);
            if simplified == original {
                continue;
            }

            let show_tips = match self.tips_level {
                SimplifierTipsLevel::None => false,
                SimplifierTipsLevel::Char => original.chars().count() == 1,
                SimplifierTipsLevel::All => true,
            };

            if self.show_in_comment {
                if show_tips {
                    candidate.comment = self.comment_format.apply(&simplified);
                } else if !self.inherit_comment {
                    candidate.comment.clear();
                }
            } else {
                candidate.text = simplified;
                if show_tips {
                    let (comment, modified) = self.comment_format.apply_with_modified(&original);
                    candidate.comment = if modified {
                        comment
                    } else {
                        format!("〔{original}〕")
                    };
                } else if !self.inherit_comment {
                    candidate.comment.clear();
                }
            }
        }
    }
}

fn simplify_traditional_text(text: &str) -> String {
    text.chars().map(simplify_traditional_char).collect()
}

fn simplify_traditional_char(ch: char) -> char {
    match ch {
        '臺' | '檯' | '颱' => '台',
        '灣' => '湾',
        '龍' => '龙',
        '風' => '风',
        '雲' => '云',
        '馬' => '马',
        '門' => '门',
        '車' => '车',
        '書' => '书',
        '學' => '学',
        '國' => '国',
        '語' => '语',
        '體' => '体',
        '電' => '电',
        '腦' => '脑',
        '麵' => '面',
        '裏' | '裡' => '里',
        '後' => '后',
        '萬' => '万',
        '與' => '与',
        '為' => '为',
        '會' => '会',
        '個' => '个',
        '們' => '们',
        '來' => '来',
        '時' => '时',
        '對' => '对',
        '說' => '说',
        '這' => '这',
        '還' => '还',
        '過' => '过',
        '開' => '开',
        '關' => '关',
        '見' => '见',
        '長' => '长',
        '發' => '发',
        '頭' => '头',
        '東' => '东',
        '廣' => '广',
        '愛' => '爱',
        '氣' => '气',
        '無' => '无',
        '點' => '点',
        '話' => '话',
        '機' => '机',
        '樂' => '乐',
        '貓' => '猫',
        '鳥' => '鸟',
        '魚' => '鱼',
        _ => ch,
    }
}

fn traditionalize_simplified_text(text: &str) -> String {
    text.chars().map(traditionalize_simplified_char).collect()
}

fn traditionalize_simplified_char(ch: char) -> char {
    match ch {
        '台' => '臺',
        '湾' => '灣',
        '龙' => '龍',
        '风' => '風',
        '云' => '雲',
        '马' => '馬',
        '门' => '門',
        '车' => '車',
        '书' => '書',
        '学' => '學',
        '国' => '國',
        '语' => '語',
        '体' => '體',
        '电' => '電',
        '脑' => '腦',
        '面' => '麵',
        '里' => '裏',
        '后' => '後',
        '万' => '萬',
        '与' => '與',
        '为' => '為',
        '会' => '會',
        '个' => '個',
        '们' => '們',
        '来' => '來',
        '时' => '時',
        '对' => '對',
        '说' => '說',
        '这' => '這',
        '还' => '還',
        '过' => '過',
        '开' => '開',
        '关' => '關',
        '见' => '見',
        '长' => '長',
        '发' => '發',
        '头' => '頭',
        '东' => '東',
        '广' => '廣',
        '爱' => '愛',
        '气' => '氣',
        '无' => '無',
        '点' => '點',
        '话' => '話',
        '机' => '機',
        '乐' => '樂',
        '猫' => '貓',
        '鸟' => '鳥',
        '鱼' => '魚',
        _ => ch,
    }
}

fn traditional_to_taiwan_text(text: &str) -> String {
    text.chars().map(traditional_to_taiwan_char).collect()
}

fn traditional_to_taiwan_char(ch: char) -> char {
    match ch {
        '台' | '臺' => '臺',
        '裏' | '裡' => '裡',
        _ => ch,
    }
}

fn taiwan_to_traditional_text(text: &str) -> String {
    text.chars().map(taiwan_to_traditional_char).collect()
}

fn taiwan_to_traditional_char(ch: char) -> char {
    match ch {
        '裡' => '裏',
        _ => ch,
    }
}

pub struct ReverseLookupFilter {
    reverse_comments: HashMap<String, Vec<String>>,
    overwrite_comment: bool,
    append_comment: bool,
    comment_format: CommentFormat,
}

impl ReverseLookupFilter {
    #[must_use]
    pub fn new(reverse_dictionary: TableDictionary) -> Self {
        let mut reverse_comments: HashMap<String, Vec<String>> = HashMap::new();
        for entry in reverse_dictionary.entries {
            reverse_comments
                .entry(entry.text)
                .or_default()
                .push(entry.code);
        }

        Self {
            reverse_comments,
            overwrite_comment: false,
            append_comment: false,
            comment_format: CommentFormat::default(),
        }
    }

    #[must_use]
    pub fn with_overwrite_comment(mut self, overwrite_comment: bool) -> Self {
        self.overwrite_comment = overwrite_comment;
        self
    }

    #[must_use]
    pub fn with_append_comment(mut self, append_comment: bool) -> Self {
        self.append_comment = append_comment;
        self
    }

    #[must_use]
    pub fn with_comment_format(mut self, formulas: &[String]) -> Self {
        self.comment_format = CommentFormat::parse(formulas);
        self
    }
}

impl CandidateFilter for ReverseLookupFilter {
    fn name(&self) -> &'static str {
        "reverse_lookup_filter"
    }

    fn apply(&self, candidates: &mut Vec<Candidate>) {
        for candidate in candidates {
            if !matches!(
                candidate.source,
                CandidateSource::Table | CandidateSource::Completion | CandidateSource::Sentence
            ) {
                continue;
            }
            if !(candidate.comment.is_empty() || self.overwrite_comment || self.append_comment) {
                continue;
            }

            let Some(comments) = self.reverse_comments.get(&candidate.text) else {
                continue;
            };
            if comments.is_empty() {
                continue;
            }

            let reverse_comment = self.comment_format.apply(&comments.join(" "));
            if self.overwrite_comment || candidate.comment.is_empty() {
                candidate.comment = reverse_comment;
            } else {
                candidate.comment = format!("{} {reverse_comment}", candidate.comment);
            }
        }
    }
}

#[derive(Clone, Default)]
struct CommentFormat {
    formulas: Vec<CommentFormatFormula>,
}

impl CommentFormat {
    fn parse(formulas: &[String]) -> Self {
        let mut parsed = Vec::new();
        for formula in formulas {
            let Some(parsed_formula) = CommentFormatFormula::parse(formula) else {
                return Self::default();
            };
            parsed.push(parsed_formula);
        }
        Self { formulas: parsed }
    }

    fn apply(&self, value: &str) -> String {
        self.apply_with_modified(value).0
    }

    fn apply_with_modified(&self, value: &str) -> (String, bool) {
        let mut formatted = value.to_owned();
        for formula in &self.formulas {
            formula.apply(&mut formatted);
            if formatted.is_empty() {
                break;
            }
        }
        let modified = formatted != value;
        (formatted, modified)
    }
}

#[derive(Clone)]
enum CommentFormatFormula {
    Transliterate(Vec<(char, char)>),
    Transform { pattern: Regex, replacement: String },
    Erase(Regex),
}

impl CommentFormatFormula {
    fn parse(definition: &str) -> Option<Self> {
        let separator = definition.chars().find(|ch| !ch.is_ascii_lowercase())?;
        let args = definition.split(separator).collect::<Vec<_>>();
        match args.first().copied()? {
            "xlit" => Self::parse_xlit(&args),
            "xform" => Self::parse_xform(&args),
            "erase" => Self::parse_erase(&args),
            _ => None,
        }
    }

    fn parse_xlit(args: &[&str]) -> Option<Self> {
        if args.len() < 3 {
            return None;
        }
        let left = args[1].chars().collect::<Vec<_>>();
        let right = args[2].chars().collect::<Vec<_>>();
        if left.len() != right.len() {
            return None;
        }
        Some(Self::Transliterate(left.into_iter().zip(right).collect()))
    }

    fn parse_xform(args: &[&str]) -> Option<Self> {
        if args.len() < 3 || args[1].is_empty() {
            return None;
        }
        Some(Self::Transform {
            pattern: Regex::new(args[1]).ok()?,
            replacement: args[2].to_owned(),
        })
    }

    fn parse_erase(args: &[&str]) -> Option<Self> {
        if args.len() < 2 || args[1].is_empty() {
            return None;
        }
        Some(Self::Erase(Regex::new(args[1]).ok()?))
    }

    fn apply(&self, value: &mut String) {
        match self {
            Self::Transliterate(char_map) => {
                let mut modified = false;
                let transformed = value
                    .chars()
                    .map(|ch| {
                        if let Some((_, replacement)) =
                            char_map.iter().find(|(source, _)| *source == ch)
                        {
                            modified = true;
                            *replacement
                        } else {
                            ch
                        }
                    })
                    .collect::<String>();
                if modified {
                    *value = transformed;
                }
            }
            Self::Transform {
                pattern,
                replacement,
            } => {
                let transformed = pattern
                    .replace_all(value, replacement.as_str())
                    .into_owned();
                if transformed != *value {
                    *value = transformed;
                }
            }
            Self::Erase(pattern) => {
                if pattern.is_match(value) {
                    value.clear();
                }
            }
        }
    }
}

#[derive(Clone, Default)]
struct SpellingAlgebra {
    formulas: Vec<SpellingAlgebraFormula>,
}

const SPELLING_ALGEBRA_FUZZY_PENALTY: f32 = -std::f32::consts::LN_2;
const SPELLING_ALGEBRA_ABBREVIATION_PENALTY: f32 = -std::f32::consts::LN_2;
const SPELLING_ALGEBRA_CORRECTION_PENALTY: f32 = -std::f32::consts::LN_10 * 2.0;

impl SpellingAlgebra {
    fn parse(formulas: &[String]) -> Self {
        let mut parsed = Vec::new();
        for formula in formulas {
            let Some(parsed_formula) = SpellingAlgebraFormula::parse(formula) else {
                return Self::default();
            };
            parsed.push(parsed_formula);
        }
        Self { formulas: parsed }
    }

    fn is_empty(&self) -> bool {
        self.formulas.is_empty()
    }

    fn expand_entries(&self, mut entries: Vec<(String, Candidate)>) -> Vec<(String, Candidate)> {
        for formula in &self.formulas {
            let mut next = Vec::new();
            for (code, candidate) in entries {
                let mut transformed = code.clone();
                let applied = formula.apply(&mut transformed);
                if applied {
                    if formula.keep_original() {
                        next.push((code, candidate.clone()));
                    }
                    if formula.add_transformed() && !transformed.is_empty() {
                        let mut candidate = candidate;
                        candidate.quality += formula.quality_penalty();
                        next.push((transformed, candidate));
                    }
                } else {
                    next.push((code, candidate));
                }
            }
            entries = dedupe_spelling_algebra_entries(next);
        }
        entries
    }
}

#[derive(Clone)]
enum SpellingAlgebraFormula {
    Transliterate(Vec<(char, char)>),
    Transform {
        pattern: Regex,
        replacement: String,
        keep_original: bool,
        add_transformed: bool,
        quality_penalty: f32,
    },
    Erase(Regex),
}

impl SpellingAlgebraFormula {
    fn parse(definition: &str) -> Option<Self> {
        let separator = definition.chars().find(|ch| !ch.is_ascii_lowercase())?;
        let args = definition.split(separator).collect::<Vec<_>>();
        match args.first().copied()? {
            "xlit" => Self::parse_xlit(&args),
            "xform" => Self::parse_transform(&args, false, true, 0.0),
            "derive" => Self::parse_derivation(&args),
            "fuzz" => Self::parse_transform(&args, true, true, SPELLING_ALGEBRA_FUZZY_PENALTY),
            "abbrev" => {
                Self::parse_transform(&args, true, true, SPELLING_ALGEBRA_ABBREVIATION_PENALTY)
            }
            "erase" => Self::parse_erase(&args),
            _ => None,
        }
    }

    fn parse_xlit(args: &[&str]) -> Option<Self> {
        if args.len() < 3 {
            return None;
        }
        let left = args[1].chars().collect::<Vec<_>>();
        let right = args[2].chars().collect::<Vec<_>>();
        if left.len() != right.len() {
            return None;
        }
        Some(Self::Transliterate(left.into_iter().zip(right).collect()))
    }

    fn parse_transform(
        args: &[&str],
        keep_original: bool,
        add_transformed: bool,
        quality_penalty: f32,
    ) -> Option<Self> {
        if args.len() < 3 || args[1].is_empty() {
            return None;
        }
        Some(Self::Transform {
            pattern: Regex::new(args[1]).ok()?,
            replacement: args[2].to_owned(),
            keep_original,
            add_transformed,
            quality_penalty,
        })
    }

    fn parse_derivation(args: &[&str]) -> Option<Self> {
        let quality_penalty = match args.get(3).copied() {
            Some("abbrev") => SPELLING_ALGEBRA_ABBREVIATION_PENALTY,
            Some("fuzz") => SPELLING_ALGEBRA_FUZZY_PENALTY,
            Some("correction") => SPELLING_ALGEBRA_CORRECTION_PENALTY,
            _ => 0.0,
        };
        Self::parse_transform(args, true, true, quality_penalty)
    }

    fn parse_erase(args: &[&str]) -> Option<Self> {
        if args.len() < 2 || args[1].is_empty() {
            return None;
        }
        Some(Self::Erase(Regex::new(args[1]).ok()?))
    }

    fn keep_original(&self) -> bool {
        match self {
            Self::Transform { keep_original, .. } => *keep_original,
            _ => false,
        }
    }

    fn quality_penalty(&self) -> f32 {
        match self {
            Self::Transform {
                quality_penalty, ..
            } => *quality_penalty,
            _ => 0.0,
        }
    }

    fn add_transformed(&self) -> bool {
        !matches!(self, Self::Erase(_))
    }

    fn apply(&self, value: &mut String) -> bool {
        match self {
            Self::Transliterate(char_map) => {
                let mut modified = false;
                let transformed = value
                    .chars()
                    .map(|ch| {
                        if let Some((_, replacement)) =
                            char_map.iter().find(|(source, _)| *source == ch)
                        {
                            modified = true;
                            *replacement
                        } else {
                            ch
                        }
                    })
                    .collect::<String>();
                if modified {
                    *value = transformed;
                }
                modified
            }
            Self::Transform {
                pattern,
                replacement,
                add_transformed,
                ..
            } => {
                let transformed = pattern
                    .replace_all(value, replacement.as_str())
                    .into_owned();
                let modified = transformed != *value;
                if modified && *add_transformed {
                    *value = transformed;
                }
                modified
            }
            Self::Erase(pattern) => {
                let should_erase = pattern
                    .find(value)
                    .is_some_and(|matched| matched.start() == 0 && matched.end() == value.len());
                if should_erase {
                    value.clear();
                }
                should_erase
            }
        }
    }
}

fn dedupe_spelling_algebra_entries(entries: Vec<(String, Candidate)>) -> Vec<(String, Candidate)> {
    let mut deduped: Vec<(String, Candidate)> = Vec::new();
    for (code, candidate) in entries {
        if let Some((_, existing_candidate)) =
            deduped
                .iter_mut()
                .find(|(existing_code, existing_candidate)| {
                    existing_code == &code
                        && existing_candidate.text == candidate.text
                        && existing_candidate.comment == candidate.comment
                })
        {
            if candidate.quality > existing_candidate.quality {
                *existing_candidate = candidate;
            }
        } else {
            deduped.push((code, candidate));
        }
    }
    deduped
}

pub struct PunctuationTranslator {
    half_shape_entries: Vec<(String, Candidate)>,
    full_shape_entries: Vec<(String, Candidate)>,
    symbol_entries: Vec<(String, Candidate)>,
    required_tags: Option<Vec<String>>,
}

impl PunctuationTranslator {
    #[must_use]
    pub fn new(entries: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>) -> Self {
        Self::with_shape_entries(entries, std::iter::empty::<(String, String)>())
    }

    #[must_use]
    pub fn with_shape_entries(
        half_shape_entries: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
        full_shape_entries: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        Self::with_shape_and_symbol_entries(
            half_shape_entries,
            full_shape_entries,
            std::iter::empty::<(String, String)>(),
        )
    }

    #[must_use]
    pub fn with_shape_and_symbol_entries(
        half_shape_entries: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
        full_shape_entries: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
        symbol_entries: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
    ) -> Self {
        Self {
            half_shape_entries: punctuation_candidates(half_shape_entries),
            full_shape_entries: punctuation_candidates(full_shape_entries),
            symbol_entries: punctuation_candidates(symbol_entries),
            required_tags: None,
        }
    }

    #[must_use]
    pub fn default_half_shape() -> Self {
        Self::new([
            (",", "，"),
            (".", "。"),
            ("?", "？"),
            ("!", "！"),
            (";", "；"),
            (":", "："),
        ])
    }

    #[must_use]
    pub fn with_required_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        let tags = tags.into_iter().map(Into::into).collect::<Vec<_>>();
        self.required_tags = (!tags.is_empty()).then_some(tags);
        self
    }
}

impl Translator for PunctuationTranslator {
    fn name(&self) -> &'static str {
        "punct_translator"
    }

    fn translate(&self, input: &str) -> Vec<Candidate> {
        self.translate_with_entries(input, &self.half_shape_entries)
    }

    fn translate_with_status(&self, input: &str, status: &Status) -> Vec<Candidate> {
        let entries = if status.is_full_shape {
            &self.full_shape_entries
        } else {
            &self.half_shape_entries
        };
        self.translate_with_entries(input, entries)
    }

    fn translate_with_context(
        &self,
        input: &str,
        status: &Status,
        _options: &HashMap<String, bool>,
        context: &Context,
    ) -> Vec<Candidate> {
        if context
            .segment_tags
            .iter()
            .any(|segment_tag| segment_tag == "punct_number")
            && !input.is_empty()
        {
            let text = shape_formatted_ascii_text(input, status.is_full_shape);
            return vec![Candidate {
                comment: punctuation_candidate_comment(&text).to_owned(),
                text,
                source: CandidateSource::Punctuation,
                quality: 1.0,
            }];
        }
        if self.required_tags.as_ref().is_some_and(|required_tags| {
            !required_tags.iter().any(|tag| {
                context
                    .segment_tags
                    .iter()
                    .any(|segment_tag| segment_tag == tag)
            })
        }) {
            return Vec::new();
        }
        self.translate_with_status(input, status)
    }
}

impl PunctuationTranslator {
    fn translate_with_entries(
        &self,
        input: &str,
        shape_entries: &[(String, Candidate)],
    ) -> Vec<Candidate> {
        let shape_candidates = shape_entries
            .iter()
            .filter(|(key, _)| key == input)
            .map(|(_, candidate)| candidate.clone())
            .collect::<Vec<_>>();
        if !shape_candidates.is_empty() {
            return shape_candidates;
        }
        self.symbol_entries
            .iter()
            .filter(|(key, _)| key == input)
            .map(|(_, candidate)| candidate.clone())
            .collect()
    }
}

fn punctuation_candidates(
    entries: impl IntoIterator<Item = (impl Into<String>, impl Into<String>)>,
) -> Vec<(String, Candidate)> {
    entries
        .into_iter()
        .map(|(key, text)| {
            let key = key.into();
            let text = text.into();
            (
                key.clone(),
                Candidate {
                    comment: punctuation_candidate_comment(&text).to_owned(),
                    text,
                    source: CandidateSource::Punctuation,
                    quality: 1.0,
                },
            )
        })
        .collect()
}

fn punctuation_candidate_comment(punct: &str) -> &'static str {
    let mut characters = punct.chars();
    let Some(ch) = characters.next() else {
        return "";
    };
    if characters.next().is_some() {
        return "";
    }

    if is_librime_half_shape_punct(ch) {
        "\u{3014}\u{534a}\u{89d2}\u{3015}"
    } else if is_librime_full_shape_punct(ch) {
        "\u{3014}\u{5168}\u{89d2}\u{3015}"
    } else {
        ""
    }
}

fn shape_formatted_ascii_text(text: &str, full_shape: bool) -> String {
    if !full_shape {
        return text.to_owned();
    }
    text.chars()
        .map(|ch| match ch {
            ' ' => '\u{3000}',
            '!'..='~' => char::from_u32(ch as u32 + 0xfee0)
                .expect("printable ASCII has a full-shape compatibility form"),
            _ => ch,
        })
        .collect()
}

fn is_librime_half_shape_punct(ch: char) -> bool {
    let code = ch as u32;
    matches!(
        code,
        0x20..=0x7e
            | 0xff61..=0xff9f
            | 0xffa0..=0xffdc
            | 0x00a2
            | 0x00a3
            | 0x00a5
            | 0x00a6
            | 0x00ac
            | 0x00af
            | 0x2985
            | 0x2986
            | 0xffe8..=0xffee
    )
}

fn is_librime_full_shape_punct(ch: char) -> bool {
    let code = ch as u32;
    matches!(
        code,
        0x3000
            | 0xff01..=0xff5e
            | 0x30a1..=0x30fc
            | 0x3001
            | 0x3002
            | 0x300c
            | 0x300d
            | 0x309b
            | 0x309c
            | 0x3131..=0x3164
            | 0xff5f
            | 0xff60
            | 0xffe0..=0xffe6
            | 0x2190..=0x2193
            | 0x2502
            | 0x25a0
            | 0x25cb
    )
}

pub struct Engine {
    context: Context,
    status: Status,
    options: HashMap<String, bool>,
    properties: HashMap<String, String>,
    translators: Vec<Box<dyn Translator>>,
    filters: Vec<Box<dyn CandidateFilter>>,
    rankers: Vec<Box<dyn CandidateRanker>>,
}

const DEFAULT_PAGE_SIZE: usize = 5;

impl Default for Engine {
    fn default() -> Self {
        Self {
            context: Context::default(),
            status: Status::default(),
            options: HashMap::new(),
            properties: HashMap::new(),
            translators: vec![Box::new(EchoTranslator)],
            filters: Vec::new(),
            rankers: Vec::new(),
        }
    }
}

impl Engine {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_translator(&mut self, translator: impl Translator + 'static) {
        let insert_at = self
            .translators
            .iter()
            .position(|existing| existing.name() == "echo_translator")
            .unwrap_or(self.translators.len());
        self.translators.insert(insert_at, Box::new(translator));
        self.refresh_candidates();
    }

    pub fn reset_translators(&mut self) {
        self.translators = vec![Box::new(EchoTranslator)];
        self.refresh_candidates();
    }

    pub fn add_filter(&mut self, filter: impl CandidateFilter + 'static) {
        self.filters.push(Box::new(filter));
        self.refresh_candidates();
    }

    pub fn reset_filters(&mut self) {
        self.filters.clear();
        self.refresh_candidates();
    }

    pub fn add_ranker(&mut self, ranker: impl CandidateRanker + 'static) {
        self.rankers.push(Box::new(ranker));
        self.refresh_candidates();
    }

    pub fn set_schema(&mut self, id: impl Into<String>, name: impl Into<String>) {
        self.status.schema_id = id.into();
        self.status.schema_name = name.into();
    }

    pub fn set_option(&mut self, option: impl Into<String>, value: bool) {
        let option = option.into();
        match option.as_str() {
            "disabled" => self.status.is_disabled = value,
            "ascii_mode" => self.status.is_ascii_mode = value,
            "full_shape" => self.status.is_full_shape = value,
            "simplification" | "simplified" => self.status.is_simplified = value,
            "traditionalization" | "traditional" => self.status.is_traditional = value,
            "ascii_punct" => self.status.is_ascii_punct = value,
            _ => {}
        }
        self.options.insert(option, value);
        self.refresh_candidates();
    }

    #[must_use]
    pub fn get_option(&self, option: &str) -> bool {
        match option {
            "disabled" => self.status.is_disabled,
            "ascii_mode" => self.status.is_ascii_mode,
            "full_shape" => self.status.is_full_shape,
            "simplification" | "simplified" => self.status.is_simplified,
            "traditionalization" | "traditional" => self.status.is_traditional,
            "ascii_punct" => self.status.is_ascii_punct,
            _ => self.options.get(option).copied().unwrap_or(false),
        }
    }

    pub fn set_property(&mut self, property: impl Into<String>, value: impl Into<String>) {
        self.properties.insert(property.into(), value.into());
    }

    pub fn set_segment_tags(&mut self, tags: impl IntoIterator<Item = impl Into<String>>) {
        self.context.segment_tags = tags.into_iter().map(Into::into).collect();
        if self.context.segment_tags.is_empty() {
            self.context.segment_tags.push("abc".to_owned());
        }
        self.refresh_candidates();
    }

    #[must_use]
    pub fn get_property(&self, property: &str) -> Option<&str> {
        self.properties.get(property).map(String::as_str)
    }

    pub fn process_char(&mut self, ch: char) -> Option<String> {
        match ch {
            '\u{8}' | '\u{7f}' => self.backspace(),
            ' ' => self.commit_highlighted(),
            '0'..='9' if self.has_selectable_candidates() => {
                self.commit_candidate_at_page_index(select_index_from_digit(ch))
            }
            _ if !ch.is_control() => {
                self.context.composition.input.push(ch);
                self.context.composition.caret = self.context.composition.input.len();
                self.context.composition.preedit = self.context.composition.input.clone();
                self.refresh_candidates();
                None
            }
            _ => None,
        }
    }

    pub fn process_key_event(&mut self, key_event: KeyEvent) -> Option<String> {
        if is_exact_control_shift_modifier(key_event.modifiers) && key_event.code == KeyCode::Return
        {
            return self.commit_comment();
        }
        if is_exact_control_shift_modifier(key_event.modifiers) {
            match key_event.code {
                KeyCode::Character(ch)
                    if ch.is_ascii_digit() && self.has_selectable_candidates() =>
                {
                    return self.commit_candidate_at_page_index(select_index_from_digit(ch));
                }
                KeyCode::KeypadDigit(ch) if self.has_selectable_candidates() => {
                    return self.commit_candidate_at_page_index(select_index_from_digit(ch));
                }
                _ => {}
            }
        }

        if is_exact_shift_modifier(key_event.modifiers) {
            match key_event.code {
                KeyCode::Return => {
                    return self.commit_script_text();
                }
                KeyCode::Backspace => {
                    return self.backspace();
                }
                KeyCode::Delete => {
                    self.delete_candidate(self.context.highlighted);
                    return None;
                }
                KeyCode::Escape => {
                    self.clear_composition();
                    return None;
                }
                KeyCode::MoveCaretLeft => {
                    self.move_caret_left_by_syllable();
                    return None;
                }
                KeyCode::MoveCaretRight => {
                    self.move_caret_right_by_syllable();
                    return None;
                }
                KeyCode::MoveCaretLeftBySyllable => {
                    self.move_caret_left_by_syllable();
                    return None;
                }
                KeyCode::MoveCaretRightBySyllable => {
                    self.move_caret_right_by_syllable();
                    return None;
                }
                KeyCode::MoveCaretLeftByChar => {
                    self.move_caret_left_by_char();
                    return None;
                }
                KeyCode::MoveCaretRightByChar => {
                    self.move_caret_right_by_char();
                    return None;
                }
                KeyCode::PreviousCandidate => {
                    self.move_caret_left_by_char();
                    return None;
                }
                KeyCode::NextCandidate => {
                    self.move_caret_right_by_char();
                    return None;
                }
                KeyCode::Home => {
                    self.move_caret_home();
                    return None;
                }
                KeyCode::End => {
                    self.move_caret_end();
                    return None;
                }
                KeyCode::Character(ch) if ch == ' ' || is_printable_ascii(ch) => {
                    return self.process_char(ch);
                }
                KeyCode::KeypadDigit(ch) if self.has_selectable_candidates() => {
                    return self.commit_candidate_at_page_index(select_index_from_digit(ch));
                }
                _ => {}
            }
        }

        if is_exact_control_modifier(key_event.modifiers) {
            match key_event.code {
                KeyCode::Backspace => {
                    return self.backspace();
                }
                KeyCode::Delete => {
                    self.delete_candidate(self.context.highlighted);
                    return None;
                }
                KeyCode::Return => {
                    return self.commit_raw_input();
                }
                KeyCode::MoveCaretLeft => {
                    self.move_caret_left_by_syllable();
                    return None;
                }
                KeyCode::MoveCaretRight => {
                    self.move_caret_right_by_syllable();
                    return None;
                }
                KeyCode::MoveCaretLeftBySyllable => {
                    self.move_caret_left_by_syllable();
                    return None;
                }
                KeyCode::MoveCaretRightBySyllable => {
                    self.move_caret_right_by_syllable();
                    return None;
                }
                KeyCode::Character(ch)
                    if ch.is_ascii_digit() && self.has_selectable_candidates() =>
                {
                    return self.commit_candidate_at_page_index(select_index_from_digit(ch));
                }
                KeyCode::KeypadDigit(ch) if self.has_selectable_candidates() => {
                    return self.commit_candidate_at_page_index(select_index_from_digit(ch));
                }
                _ => {}
            }
        }

        if !key_event.modifiers.is_empty() {
            return None;
        }

        match key_event.code {
            KeyCode::Character(ch) => self.process_char(ch),
            KeyCode::KeypadDigit(ch) if self.has_selectable_candidates() => {
                self.commit_candidate_at_page_index(select_index_from_digit(ch))
            }
            KeyCode::KeypadDigit(_) => None,
            KeyCode::Tab => None,
            KeyCode::Ignored => None,
            KeyCode::Backspace => self.backspace(),
            KeyCode::Delete => self.delete_at_caret(),
            KeyCode::Escape => {
                self.clear_composition();
                None
            }
            KeyCode::MoveCaretLeft => {
                self.move_caret_left();
                None
            }
            KeyCode::MoveCaretRight => {
                self.move_caret_right();
                None
            }
            KeyCode::MoveCaretLeftByChar => {
                self.move_caret_left_by_char();
                None
            }
            KeyCode::MoveCaretRightByChar => {
                self.move_caret_right_by_char();
                None
            }
            KeyCode::MoveCaretLeftBySyllable => {
                self.move_caret_left_by_syllable();
                None
            }
            KeyCode::MoveCaretRightBySyllable => {
                self.move_caret_right_by_syllable();
                None
            }
            KeyCode::Home => {
                if !self.first_candidate() {
                    self.move_caret_home();
                }
                None
            }
            KeyCode::End => {
                if self.context.composition.caret < self.context.composition.input.len()
                    || !self.first_candidate()
                {
                    self.move_caret_end();
                }
                None
            }
            KeyCode::PreviousCandidate => {
                self.previous_candidate();
                None
            }
            KeyCode::NextCandidate => {
                self.next_candidate();
                None
            }
            KeyCode::FirstCandidate => {
                self.first_candidate();
                None
            }
            KeyCode::PreviousPage => {
                self.change_page(true);
                None
            }
            KeyCode::NextPage => {
                self.change_page(false);
                None
            }
            KeyCode::Return | KeyCode::KeypadEnter => self.commit_highlighted(),
        }
    }

    pub fn process_sequence(&mut self, input: &str) -> Vec<String> {
        input
            .chars()
            .filter_map(|ch| self.process_char(ch))
            .collect()
    }

    pub fn process_key_sequence(
        &mut self,
        input: &str,
    ) -> Result<Vec<String>, KeySequenceParseError> {
        Ok(parse_key_sequence(input)?
            .into_iter()
            .filter_map(|key_event| self.process_key_event(key_event))
            .collect())
    }

    pub fn commit_composition(&mut self) -> Option<String> {
        self.commit_highlighted()
    }

    pub fn commit_raw_input(&mut self) -> Option<String> {
        self.commit_raw_input_text()
    }

    pub fn select_candidate(&mut self, index: usize) -> Option<String> {
        self.commit_candidate(index)
    }

    pub fn select_candidate_on_current_page(&mut self, index: usize) -> Option<String> {
        self.commit_candidate_at_page_index(index)
    }

    pub fn highlight_candidate(&mut self, index: usize) -> bool {
        if index >= self.context.candidates.len() {
            return false;
        }
        self.context.highlighted = index;
        true
    }

    pub fn highlight_candidate_on_current_page(&mut self, index: usize) -> bool {
        if index >= DEFAULT_PAGE_SIZE {
            return false;
        }
        let page_start = (self.context.highlighted / DEFAULT_PAGE_SIZE) * DEFAULT_PAGE_SIZE;
        self.highlight_candidate(page_start + index)
    }

    pub fn delete_candidate(&mut self, index: usize) -> bool {
        if index >= self.context.candidates.len() {
            return false;
        }
        self.context.candidates.remove(index);
        if self.context.candidates.is_empty() {
            self.context.highlighted = 0;
        } else if index < self.context.highlighted {
            self.context.highlighted -= 1;
        } else if self.context.highlighted >= self.context.candidates.len() {
            self.context.highlighted = self.context.candidates.len() - 1;
        }
        true
    }

    pub fn delete_candidate_on_current_page(&mut self, index: usize) -> bool {
        if index >= DEFAULT_PAGE_SIZE {
            return false;
        }
        let page_start = (self.context.highlighted / DEFAULT_PAGE_SIZE) * DEFAULT_PAGE_SIZE;
        self.delete_candidate(page_start + index)
    }

    pub fn change_page(&mut self, backward: bool) -> bool {
        self.change_page_by(DEFAULT_PAGE_SIZE, backward)
    }

    pub fn change_page_by(&mut self, page_size: usize, backward: bool) -> bool {
        if !self.has_selectable_candidates() {
            return false;
        }

        let page_size = page_size.max(1);
        let current_index = self.context.highlighted;
        let next_index = if backward {
            current_index.saturating_sub(page_size)
        } else {
            current_index + page_size
        };
        let next_index = next_index.min(self.context.candidates.len() - 1);
        if current_index == next_index {
            return false;
        }
        self.highlight_candidate(next_index)
    }

    pub fn previous_candidate(&mut self) -> bool {
        if !self.has_selectable_candidates() {
            return false;
        }
        if self.context.highlighted == 0 {
            return true;
        }
        self.highlight_candidate(self.context.highlighted - 1)
    }

    pub fn next_candidate(&mut self) -> bool {
        if !self.has_selectable_candidates() {
            return false;
        }
        let next_index = self.context.highlighted + 1;
        if next_index >= self.context.candidates.len() {
            return true;
        }
        self.highlight_candidate(next_index)
    }

    pub fn first_candidate(&mut self) -> bool {
        if !self.has_selectable_candidates() {
            return false;
        }
        if self.context.highlighted == 0 {
            return false;
        }
        self.highlight_candidate(0)
    }

    fn has_selectable_candidates(&self) -> bool {
        !self.context.candidates.is_empty()
            && !self.context.segment_tags.iter().any(|tag| tag == "raw")
    }

    pub fn clear_composition(&mut self) {
        self.context.composition = Composition::default();
        self.context.candidates.clear();
        self.context.highlighted = 0;
    }

    pub fn set_input(&mut self, input: impl Into<String>) {
        let input = input.into();
        self.context.composition.input = input.clone();
        self.context.composition.caret = input.len();
        self.context.composition.preedit = input;
        self.refresh_candidates();
    }

    pub fn set_punctuation_composition(
        &mut self,
        input: impl Into<String>,
        text: impl Into<String>,
    ) {
        let input = input.into();
        let text = text.into();
        self.context.composition.input = input.clone();
        self.context.composition.caret = input.len();
        self.context.composition.preedit = input;
        self.context.candidates = vec![Candidate {
            comment: punctuation_candidate_comment(&text).to_owned(),
            text,
            source: CandidateSource::Punctuation,
            quality: 1.0,
        }];
        self.context.highlighted = 0;
    }

    pub fn record_commit(&mut self, text: impl Into<String>) -> String {
        let text = text.into();
        self.record_commit_with_type("raw", text.clone());
        self.clear_composition();
        text
    }

    pub fn set_caret_pos(&mut self, caret_pos: usize) {
        self.context.composition.caret = caret_pos.min(self.context.composition.input.len());
    }

    pub fn move_caret_left(&mut self) -> bool {
        if self.context.composition.caret == 0 {
            return false;
        }
        self.context.composition.caret -= 1;
        true
    }

    pub fn move_caret_right(&mut self) -> bool {
        if self.context.composition.caret >= self.context.composition.input.len() {
            return false;
        }
        self.context.composition.caret += 1;
        true
    }

    pub fn move_caret_left_by_char(&mut self) -> bool {
        if self.move_caret_left() {
            return true;
        }
        if self.context.composition.input.is_empty()
            || self.context.composition.caret == self.context.composition.input.len()
        {
            return false;
        }
        self.context.composition.caret = self.context.composition.input.len();
        true
    }

    pub fn move_caret_right_by_char(&mut self) -> bool {
        if self.move_caret_right() {
            return true;
        }
        if self.context.composition.input.is_empty() || self.context.composition.caret == 0 {
            return false;
        }
        self.context.composition.caret = 0;
        true
    }

    pub fn move_caret_left_by_syllable(&mut self) -> bool {
        if self.context.composition.input.is_empty() || self.context.composition.caret == 0 {
            return false;
        }
        self.context.composition.caret = 0;
        true
    }

    pub fn move_caret_right_by_syllable(&mut self) -> bool {
        if self.context.composition.caret >= self.context.composition.input.len() {
            return false;
        }
        self.context.composition.caret = self.context.composition.input.len();
        true
    }

    pub fn move_caret_home(&mut self) -> bool {
        if self.context.composition.caret == 0 {
            return false;
        }
        self.context.composition.caret = 0;
        true
    }

    pub fn move_caret_end(&mut self) -> bool {
        if self.context.composition.caret >= self.context.composition.input.len() {
            return false;
        }
        self.context.composition.caret = self.context.composition.input.len();
        true
    }

    #[must_use]
    pub fn context(&self) -> &Context {
        &self.context
    }

    #[must_use]
    pub fn status(&self) -> Status {
        let mut status = self.status.clone();
        status.is_composing = !self.context.composition.input.is_empty();
        status
    }

    #[must_use]
    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            context: self.context.clone(),
            status: self.status(),
        }
    }

    fn backspace(&mut self) -> Option<String> {
        if self.context.composition.caret == 0 {
            return None;
        }
        self.context.composition.caret -= 1;
        self.context
            .composition
            .input
            .remove(self.context.composition.caret);
        self.context.composition.preedit = self.context.composition.input.clone();
        self.refresh_candidates();
        None
    }

    fn delete_at_caret(&mut self) -> Option<String> {
        if self.context.composition.caret < self.context.composition.input.len() {
            self.context
                .composition
                .input
                .remove(self.context.composition.caret);
            self.context.composition.preedit = self.context.composition.input.clone();
            self.refresh_candidates();
        }
        None
    }

    fn commit_highlighted(&mut self) -> Option<String> {
        self.commit_candidate(self.context.highlighted)
    }

    fn commit_raw_input_text(&mut self) -> Option<String> {
        if self.context.composition.input.is_empty() {
            return None;
        }
        let text = self.context.composition.input.clone();
        self.record_commit_with_type("raw", text.clone());
        self.clear_composition();
        Some(text)
    }

    pub fn commit_script_text(&mut self) -> Option<String> {
        if self.context.composition.preedit.is_empty() {
            return None;
        }
        let text = self.context.composition.preedit.clone();
        self.record_commit_with_type("raw", text.clone());
        self.clear_composition();
        Some(text)
    }

    pub fn commit_comment(&mut self) -> Option<String> {
        let text = self
            .context
            .candidates
            .get(self.context.highlighted)
            .and_then(|candidate| {
                (!candidate.comment.is_empty()).then(|| candidate.comment.clone())
            })?;
        self.record_commit_with_type("raw", text.clone());
        self.clear_composition();
        Some(text)
    }

    pub fn back_to_previous_input(&mut self) -> Option<String> {
        self.backspace()
    }

    pub fn delete_input(&mut self) -> Option<String> {
        self.delete_at_caret()
    }

    fn commit_candidate_at_page_index(&mut self, page_index: usize) -> Option<String> {
        if page_index >= DEFAULT_PAGE_SIZE {
            return None;
        }
        let page_start = (self.context.highlighted / DEFAULT_PAGE_SIZE) * DEFAULT_PAGE_SIZE;
        self.commit_candidate(page_start + page_index)
    }

    fn commit_candidate(&mut self, candidate_index: usize) -> Option<String> {
        let (text, candidate_type) = self
            .context
            .candidates
            .get(candidate_index)
            .map(|candidate| (candidate.text.clone(), candidate.source.as_str().to_owned()))?;
        self.record_commit_with_type(candidate_type, text.clone());
        self.clear_composition();
        Some(text)
    }

    fn record_commit_with_type(&mut self, candidate_type: impl Into<String>, text: String) {
        self.context.last_commit = Some(text.clone());
        self.context.commit_history.push(CommitRecord {
            candidate_type: candidate_type.into(),
            text,
        });
    }

    fn refresh_candidates(&mut self) {
        let input = self.context.composition.input.as_str();
        let mut candidates = self
            .translators
            .iter()
            .flat_map(|translator| {
                translator.translate_with_context(input, &self.status, &self.options, &self.context)
            })
            .collect::<Vec<_>>();
        candidates.sort_by(|left, right| {
            right
                .quality
                .partial_cmp(&left.quality)
                .unwrap_or(Ordering::Equal)
        });
        for filter in &self.filters {
            filter.apply_with_context(&mut candidates, &self.options, &self.context);
        }
        for ranker in &self.rankers {
            if let RerankResult::Ready(ranked) = ranker.try_rerank(&self.context, &candidates) {
                candidates = ranked;
            }
        }
        self.context.candidates = candidates;
        self.context.highlighted = 0;
    }
}

const fn is_exact_control_modifier(modifiers: KeyModifiers) -> bool {
    modifiers.control
        && !modifiers.shift
        && !modifiers.lock
        && !modifiers.alt
        && !modifiers.super_key
        && !modifiers.hyper
        && !modifiers.meta
        && !modifiers.release
}

const fn is_exact_shift_modifier(modifiers: KeyModifiers) -> bool {
    modifiers.shift
        && !modifiers.lock
        && !modifiers.control
        && !modifiers.alt
        && !modifiers.super_key
        && !modifiers.hyper
        && !modifiers.meta
        && !modifiers.release
}

const fn is_exact_control_shift_modifier(modifiers: KeyModifiers) -> bool {
    modifiers.control
        && modifiers.shift
        && !modifiers.lock
        && !modifiers.alt
        && !modifiers.super_key
        && !modifiers.hyper
        && !modifiers.meta
        && !modifiers.release
}

const fn is_printable_ascii(ch: char) -> bool {
    matches!(ch, '!'..='~')
}

const fn select_index_from_digit(ch: char) -> usize {
    match ch {
        '1'..='9' => ch as usize - '1' as usize,
        '0' => 9,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        parse_key_sequence, parse_rime_prism_bin_metadata, parse_rime_reverse_bin_metadata,
        parse_rime_table_bin_metadata, rime_checksum_bytes, rime_dict_rebuild_plan,
        rime_dict_source_checksum, Candidate, CandidateFilter, CandidateRanker, CandidateSource,
        CharsetFilter, CodeCoords, Context, Engine, HistoryTranslator, KeyCode, MockAiRanker,
        PunctuationTranslator, RerankResult, ReverseLookupFilter, ReverseLookupTranslator,
        RimeChecksumComputer, RimeCompiledMetadataError, RimeDictRebuildError,
        RimeDictRebuildInput, RimeDictRebuildPlan, RimePrismBinMetadata, RimePrismChecksumMetadata,
        RimeReverseBinMetadata, RimeTableBinMetadata, SimplifierFilter, SingleCharFilter,
        StaticTableTranslator, TableDictionary, TableEncoder, TaggedFilter, Translator,
        UniquifierFilter,
    };

    struct CommentTranslator;

    impl Translator for CommentTranslator {
        fn name(&self) -> &'static str {
            "comment_translator"
        }

        fn translate(&self, input: &str) -> Vec<Candidate> {
            if input != "ni" {
                return Vec::new();
            }
            vec![
                Candidate {
                    text: "你".to_owned(),
                    comment: "first-comment".to_owned(),
                    source: CandidateSource::Table,
                    quality: 1.0,
                },
                Candidate {
                    text: "呢".to_owned(),
                    comment: "second-comment".to_owned(),
                    source: CandidateSource::Table,
                    quality: 1.0,
                },
            ]
        }
    }

    #[test]
    fn parses_librime_style_key_sequence_names() {
        let keys = parse_key_sequence(
            "zyx 123{Shift+space}ABC{Control+Alt+Return}{KP_Enter}{KP_2}{Tab}{Delete}{Escape}{Left}{Right}{KP_Left}{KP_Right}{Home}{KP_End}{Page_Down}{KP_Page_Up}{Down}{KP_Up}{Control+Up}{Control+Down}",
        )
        .expect("key sequence should parse");

        assert_eq!(keys.len(), 29);
        assert_eq!(keys[3].code, KeyCode::Character(' '));
        assert!(!keys[3].modifiers.shift);
        assert_eq!(keys[7].code, KeyCode::Character(' '));
        assert!(keys[7].modifiers.shift);
        assert_eq!(keys[11].code, KeyCode::Return);
        assert!(keys[11].modifiers.control);
        assert!(keys[11].modifiers.alt);
        assert_eq!(keys[12].code, KeyCode::KeypadEnter);
        assert_eq!(keys[13].code, KeyCode::KeypadDigit('2'));
        assert_eq!(keys[14].code, KeyCode::Tab);
        assert_eq!(keys[15].code, KeyCode::Delete);
        assert_eq!(keys[16].code, KeyCode::Escape);
        assert_eq!(keys[17].code, KeyCode::MoveCaretLeft);
        assert_eq!(keys[18].code, KeyCode::MoveCaretRight);
        assert_eq!(keys[19].code, KeyCode::MoveCaretLeftByChar);
        assert_eq!(keys[20].code, KeyCode::MoveCaretRightByChar);
        assert_eq!(keys[21].code, KeyCode::Home);
        assert_eq!(keys[22].code, KeyCode::End);
        assert_eq!(keys[23].code, KeyCode::NextPage);
        assert_eq!(keys[24].code, KeyCode::PreviousPage);
        assert_eq!(keys[25].code, KeyCode::NextCandidate);
        assert_eq!(keys[26].code, KeyCode::PreviousCandidate);
        assert_eq!(keys[27].code, KeyCode::MoveCaretLeftBySyllable);
        assert!(keys[27].modifiers.control);
        assert_eq!(keys[28].code, KeyCode::MoveCaretRightBySyllable);
        assert!(keys[28].modifiers.control);
    }

    #[test]
    fn parses_named_braces_for_literal_brace_keys() {
        let keys =
            parse_key_sequence("{braceleft}{braceright}").expect("key sequence should parse");

        assert_eq!(keys[0].code, KeyCode::Character('{'));
        assert_eq!(keys[1].code, KeyCode::Character('}'));
    }

    #[test]
    fn parses_librime_ascii_symbol_key_names_as_printable_characters() {
        let cases = [
            ("exclam", '!'),
            ("quotedbl", '"'),
            ("numbersign", '#'),
            ("dollar", '$'),
            ("percent", '%'),
            ("ampersand", '&'),
            ("apostrophe", '\''),
            ("quoteright", '\''),
            ("parenleft", '('),
            ("parenright", ')'),
            ("asterisk", '*'),
            ("plus", '+'),
            ("comma", ','),
            ("minus", '-'),
            ("period", '.'),
            ("slash", '/'),
            ("colon", ':'),
            ("semicolon", ';'),
            ("less", '<'),
            ("equal", '='),
            ("greater", '>'),
            ("question", '?'),
            ("at", '@'),
            ("bracketleft", '['),
            ("backslash", '\\'),
            ("bracketright", ']'),
            ("asciicircum", '^'),
            ("underscore", '_'),
            ("grave", '`'),
            ("quoteleft", '`'),
            ("braceleft", '{'),
            ("bar", '|'),
            ("braceright", '}'),
            ("asciitilde", '~'),
        ];
        let sequence = cases
            .iter()
            .map(|(name, _)| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), cases.len());
        for (key, (_, expected)) in keys.iter().zip(cases) {
            assert_eq!(key.code, KeyCode::Character(expected));
            assert!(key.modifiers.is_empty());
        }
    }

    #[test]
    fn parses_librime_known_noop_key_names() {
        let keys = parse_key_sequence(
            "{Linefeed}{Clear}{Pause}{Scroll_Lock}{Sys_Req}{Begin}{Select}{Print}{Execute}{Insert}{Undo}{Redo}{Menu}{Find}{Cancel}{Help}{Break}{Arabic_switch}{Greek_switch}{Hangul_switch}{Hebrew_switch}{ISO_Group_Shift}{Mode_switch}{kana_switch}{script_switch}{Num_Lock}{F1}{Alt+F4}{F12}{F13}{F35}{Shift_L}{Shift_R}{Control_L}{Control_R}{Caps_Lock}{Shift_Lock}{Meta_L}{Meta_R}{Alt_L}{Alt_R}{Super_L}{Super_R}{Hyper_L}{Release+Hyper_R}{ISO_Lock}{ISO_Level2_Latch}{ISO_Level3_Shift}{ISO_Level3_Latch}{ISO_Level3_Lock}{ISO_Group_Latch}{ISO_Group_Lock}{ISO_Next_Group}{ISO_Next_Group_Lock}{ISO_Prev_Group}{ISO_Prev_Group_Lock}{ISO_First_Group}{ISO_First_Group_Lock}{ISO_Last_Group}{ISO_Last_Group_Lock}{ISO_Left_Tab}{ISO_Move_Line_Up}{ISO_Move_Line_Down}{ISO_Partial_Line_Up}{ISO_Partial_Line_Down}{ISO_Partial_Space_Left}{ISO_Partial_Space_Right}{ISO_Set_Margin_Left}{ISO_Set_Margin_Right}{ISO_Release_Margin_Left}{ISO_Release_Margin_Right}{ISO_Release_Both_Margins}{ISO_Fast_Cursor_Left}{ISO_Fast_Cursor_Right}{ISO_Fast_Cursor_Up}{ISO_Fast_Cursor_Down}{ISO_Continuous_Underline}{ISO_Discontinuous_Underline}{ISO_Emphasize}{ISO_Center_Object}{Release+ISO_Enter}",
        )
        .expect("key sequence should parse");

        assert_eq!(keys.len(), 81);
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys.iter().enumerate().all(|(index, key)| index == 27
            || index == 44
            || index == 80
            || key.modifiers.is_empty()));
        assert!(keys[27].modifiers.alt);
        assert!(keys[44].modifiers.release);
        assert!(keys[80].modifiers.release);
    }

    #[test]
    fn parses_librime_xkb_noop_key_names() {
        let names = [
            "dead_grave",
            "dead_acute",
            "dead_circumflex",
            "dead_tilde",
            "dead_macron",
            "dead_breve",
            "dead_abovedot",
            "dead_diaeresis",
            "dead_abovering",
            "dead_doubleacute",
            "dead_caron",
            "dead_cedilla",
            "dead_ogonek",
            "dead_iota",
            "dead_voiced_sound",
            "dead_semivoiced_sound",
            "dead_belowdot",
            "dead_hook",
            "dead_horn",
            "AccessX_Enable",
            "AccessX_Feedback_Enable",
            "RepeatKeys_Enable",
            "SlowKeys_Enable",
            "BounceKeys_Enable",
            "StickyKeys_Enable",
            "MouseKeys_Enable",
            "MouseKeys_Accel_Enable",
            "Overlay1_Enable",
            "Overlay2_Enable",
            "AudibleBell_Enable",
            "First_Virtual_Screen",
            "Prev_Virtual_Screen",
            "Next_Virtual_Screen",
            "Last_Virtual_Screen",
            "Terminate_Server",
            "Pointer_Left",
            "Pointer_Right",
            "Pointer_Up",
            "Pointer_Down",
            "Pointer_UpLeft",
            "Pointer_UpRight",
            "Pointer_DownLeft",
            "Pointer_DownRight",
            "Pointer_Button_Dflt",
            "Pointer_Button1",
            "Pointer_Button2",
            "Pointer_Button3",
            "Pointer_Button4",
            "Pointer_Button5",
            "Pointer_DblClick_Dflt",
            "Pointer_DblClick1",
            "Pointer_DblClick2",
            "Pointer_DblClick3",
            "Pointer_DblClick4",
            "Pointer_DblClick5",
            "Pointer_Drag_Dflt",
            "Pointer_Drag1",
            "Pointer_Drag2",
            "Pointer_Drag3",
            "Pointer_Drag4",
            "Pointer_EnableKeys",
            "Pointer_Accelerate",
            "Pointer_DfltBtnNext",
            "Pointer_DfltBtnPrev",
            "Pointer_Drag5",
            "Release+Pointer_Drag5",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_input_method_noop_key_names() {
        let names = [
            "Multi_key",
            "Kanji",
            "Muhenkan",
            "Henkan",
            "Henkan_Mode",
            "Romaji",
            "Hiragana",
            "Katakana",
            "Hiragana_Katakana",
            "Zenkaku",
            "Hankaku",
            "Zenkaku_Hankaku",
            "Touroku",
            "Massyo",
            "Kana_Lock",
            "Kana_Shift",
            "Eisu_Shift",
            "Eisu_toggle",
            "Hangul",
            "Hangul_Start",
            "Hangul_End",
            "Hangul_Hanja",
            "Hangul_Jamo",
            "Hangul_Romaja",
            "Codeinput",
            "Hangul_Jeonja",
            "Hangul_Banja",
            "Hangul_PreHanja",
            "Hangul_PostHanja",
            "SingleCandidate",
            "MultipleCandidate",
            "PreviousCandidate",
            "Release+Hangul_Special",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_keypad_noop_key_names() {
        let names = [
            "KP_Space",
            "KP_Tab",
            "KP_F1",
            "KP_F2",
            "KP_F3",
            "KP_F4",
            "KP_Begin",
            "KP_Insert",
            "KP_Delete",
            "KP_Multiply",
            "KP_Add",
            "KP_Separator",
            "KP_Subtract",
            "KP_Decimal",
            "KP_Divide",
            "Release+KP_Equal",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_latin1_noop_key_names() {
        let names = [
            "nobreakspace",
            "exclamdown",
            "cent",
            "sterling",
            "currency",
            "yen",
            "brokenbar",
            "section",
            "diaeresis",
            "copyright",
            "ordfeminine",
            "guillemotleft",
            "notsign",
            "hyphen",
            "registered",
            "macron",
            "degree",
            "plusminus",
            "twosuperior",
            "threesuperior",
            "acute",
            "mu",
            "paragraph",
            "periodcentered",
            "cedilla",
            "onesuperior",
            "masculine",
            "guillemotright",
            "onequarter",
            "onehalf",
            "threequarters",
            "questiondown",
            "Agrave",
            "Aacute",
            "Acircumflex",
            "Atilde",
            "Adiaeresis",
            "Aring",
            "AE",
            "Ccedilla",
            "Egrave",
            "Eacute",
            "Ecircumflex",
            "Ediaeresis",
            "Igrave",
            "Iacute",
            "Icircumflex",
            "Idiaeresis",
            "ETH",
            "Eth",
            "Ntilde",
            "Ograve",
            "Oacute",
            "Ocircumflex",
            "Otilde",
            "Odiaeresis",
            "multiply",
            "Ooblique",
            "Ugrave",
            "Uacute",
            "Ucircumflex",
            "Udiaeresis",
            "Yacute",
            "THORN",
            "Thorn",
            "ssharp",
            "agrave",
            "aacute",
            "acircumflex",
            "atilde",
            "adiaeresis",
            "aring",
            "ae",
            "ccedilla",
            "egrave",
            "eacute",
            "ecircumflex",
            "ediaeresis",
            "igrave",
            "iacute",
            "icircumflex",
            "idiaeresis",
            "eth",
            "ntilde",
            "ograve",
            "oacute",
            "ocircumflex",
            "otilde",
            "odiaeresis",
            "division",
            "oslash",
            "ugrave",
            "uacute",
            "ucircumflex",
            "udiaeresis",
            "yacute",
            "thorn",
            "Release+ydiaeresis",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_latin2_noop_key_names() {
        let names = [
            "Aogonek",
            "breve",
            "Lstroke",
            "Lcaron",
            "Sacute",
            "Scaron",
            "Scedilla",
            "Tcaron",
            "Zacute",
            "Zcaron",
            "Zabovedot",
            "aogonek",
            "ogonek",
            "lstroke",
            "lcaron",
            "sacute",
            "caron",
            "scaron",
            "scedilla",
            "tcaron",
            "zacute",
            "doubleacute",
            "zcaron",
            "zabovedot",
            "Racute",
            "Abreve",
            "Lacute",
            "Cacute",
            "Ccaron",
            "Eogonek",
            "Ecaron",
            "Dcaron",
            "Dstroke",
            "Nacute",
            "Ncaron",
            "Odoubleacute",
            "Rcaron",
            "Uring",
            "Udoubleacute",
            "Tcedilla",
            "racute",
            "abreve",
            "lacute",
            "cacute",
            "ccaron",
            "eogonek",
            "ecaron",
            "dcaron",
            "dstroke",
            "nacute",
            "ncaron",
            "odoubleacute",
            "udoubleacute",
            "rcaron",
            "uring",
            "tcedilla",
            "Release+abovedot",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_latin3_noop_key_names() {
        let names = [
            "Hstroke",
            "Hcircumflex",
            "Iabovedot",
            "Gbreve",
            "Jcircumflex",
            "hstroke",
            "hcircumflex",
            "idotless",
            "gbreve",
            "jcircumflex",
            "Cabovedot",
            "Ccircumflex",
            "Gabovedot",
            "Gcircumflex",
            "Ubreve",
            "Scircumflex",
            "cabovedot",
            "ccircumflex",
            "gabovedot",
            "gcircumflex",
            "ubreve",
            "Release+scircumflex",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_latin4_noop_key_names() {
        let names = [
            "kappa",
            "kra",
            "Rcedilla",
            "Itilde",
            "Lcedilla",
            "Emacron",
            "Gcedilla",
            "Tslash",
            "rcedilla",
            "itilde",
            "lcedilla",
            "emacron",
            "gcedilla",
            "tslash",
            "ENG",
            "eng",
            "Amacron",
            "Iogonek",
            "Eabovedot",
            "Imacron",
            "Ncedilla",
            "Omacron",
            "Kcedilla",
            "Uogonek",
            "Utilde",
            "Umacron",
            "amacron",
            "iogonek",
            "eabovedot",
            "imacron",
            "ncedilla",
            "omacron",
            "kcedilla",
            "uogonek",
            "utilde",
            "Release+umacron",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_kana_noop_key_names() {
        let names = [
            "overline",
            "kana_fullstop",
            "kana_openingbracket",
            "kana_closingbracket",
            "kana_comma",
            "kana_conjunctive",
            "kana_middledot",
            "kana_WO",
            "kana_a",
            "kana_i",
            "kana_u",
            "kana_e",
            "kana_o",
            "kana_ya",
            "kana_yu",
            "kana_yo",
            "kana_tsu",
            "kana_tu",
            "prolongedsound",
            "kana_A",
            "kana_I",
            "kana_U",
            "kana_E",
            "kana_O",
            "kana_KA",
            "kana_KI",
            "kana_KU",
            "kana_KE",
            "kana_KO",
            "kana_SA",
            "kana_SHI",
            "kana_SU",
            "kana_SE",
            "kana_SO",
            "kana_TA",
            "kana_CHI",
            "kana_TI",
            "kana_TSU",
            "kana_TU",
            "kana_TE",
            "kana_TO",
            "kana_NA",
            "kana_NI",
            "kana_NU",
            "kana_NE",
            "kana_NO",
            "kana_HA",
            "kana_HI",
            "kana_FU",
            "kana_HU",
            "kana_HE",
            "kana_HO",
            "kana_MA",
            "kana_MI",
            "kana_MU",
            "kana_ME",
            "kana_MO",
            "kana_YA",
            "kana_YU",
            "kana_YO",
            "kana_RA",
            "kana_RI",
            "kana_RU",
            "kana_RE",
            "kana_RO",
            "kana_WA",
            "kana_N",
            "voicedsound",
            "Release+semivoicedsound",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_arabic_noop_key_names() {
        let names = [
            "Arabic_comma",
            "Arabic_semicolon",
            "Arabic_question_mark",
            "Arabic_hamza",
            "Arabic_maddaonalef",
            "Arabic_hamzaonalef",
            "Arabic_hamzaonwaw",
            "Arabic_hamzaunderalef",
            "Arabic_hamzaonyeh",
            "Arabic_alef",
            "Arabic_beh",
            "Arabic_tehmarbuta",
            "Arabic_teh",
            "Arabic_theh",
            "Arabic_jeem",
            "Arabic_hah",
            "Arabic_khah",
            "Arabic_dal",
            "Arabic_thal",
            "Arabic_ra",
            "Arabic_zain",
            "Arabic_seen",
            "Arabic_sheen",
            "Arabic_sad",
            "Arabic_dad",
            "Arabic_tah",
            "Arabic_zah",
            "Arabic_ain",
            "Arabic_ghain",
            "Arabic_tatweel",
            "Arabic_feh",
            "Arabic_qaf",
            "Arabic_kaf",
            "Arabic_lam",
            "Arabic_meem",
            "Arabic_noon",
            "Arabic_ha",
            "Arabic_heh",
            "Arabic_waw",
            "Arabic_alefmaksura",
            "Arabic_yeh",
            "Arabic_fathatan",
            "Arabic_dammatan",
            "Arabic_kasratan",
            "Arabic_fatha",
            "Arabic_damma",
            "Arabic_kasra",
            "Arabic_shadda",
            "Release+Arabic_sukun",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_cyrillic_noop_key_names() {
        let names = [
            "Serbian_dje",
            "Macedonia_gje",
            "Cyrillic_io",
            "Ukrainian_ie",
            "Ukranian_je",
            "Macedonia_dse",
            "Ukrainian_i",
            "Ukranian_i",
            "Ukrainian_yi",
            "Ukranian_yi",
            "Cyrillic_je",
            "Serbian_je",
            "Cyrillic_lje",
            "Serbian_lje",
            "Cyrillic_nje",
            "Serbian_nje",
            "Serbian_tshe",
            "Macedonia_kje",
            "Byelorussian_shortu",
            "Cyrillic_dzhe",
            "Serbian_dze",
            "numerosign",
            "Serbian_DJE",
            "Macedonia_GJE",
            "Cyrillic_IO",
            "Ukrainian_IE",
            "Ukranian_JE",
            "Macedonia_DSE",
            "Ukrainian_I",
            "Ukranian_I",
            "Ukrainian_YI",
            "Ukranian_YI",
            "Cyrillic_JE",
            "Serbian_JE",
            "Cyrillic_LJE",
            "Serbian_LJE",
            "Cyrillic_NJE",
            "Serbian_NJE",
            "Serbian_TSHE",
            "Macedonia_KJE",
            "Byelorussian_SHORTU",
            "Cyrillic_DZHE",
            "Serbian_DZE",
            "Cyrillic_yu",
            "Cyrillic_a",
            "Cyrillic_be",
            "Cyrillic_tse",
            "Cyrillic_de",
            "Cyrillic_ie",
            "Cyrillic_ef",
            "Cyrillic_ghe",
            "Cyrillic_ha",
            "Cyrillic_i",
            "Cyrillic_shorti",
            "Cyrillic_ka",
            "Cyrillic_el",
            "Cyrillic_em",
            "Cyrillic_en",
            "Cyrillic_o",
            "Cyrillic_pe",
            "Cyrillic_ya",
            "Cyrillic_er",
            "Cyrillic_es",
            "Cyrillic_te",
            "Cyrillic_u",
            "Cyrillic_zhe",
            "Cyrillic_ve",
            "Cyrillic_softsign",
            "Cyrillic_yeru",
            "Cyrillic_ze",
            "Cyrillic_sha",
            "Cyrillic_e",
            "Cyrillic_shcha",
            "Cyrillic_che",
            "Cyrillic_hardsign",
            "Cyrillic_YU",
            "Cyrillic_A",
            "Cyrillic_BE",
            "Cyrillic_TSE",
            "Cyrillic_DE",
            "Cyrillic_IE",
            "Cyrillic_EF",
            "Cyrillic_GHE",
            "Cyrillic_HA",
            "Cyrillic_I",
            "Cyrillic_SHORTI",
            "Cyrillic_KA",
            "Cyrillic_EL",
            "Cyrillic_EM",
            "Cyrillic_EN",
            "Cyrillic_O",
            "Cyrillic_PE",
            "Cyrillic_YA",
            "Cyrillic_ER",
            "Cyrillic_ES",
            "Cyrillic_TE",
            "Cyrillic_U",
            "Cyrillic_ZHE",
            "Cyrillic_VE",
            "Cyrillic_SOFTSIGN",
            "Cyrillic_YERU",
            "Cyrillic_ZE",
            "Cyrillic_SHA",
            "Cyrillic_E",
            "Cyrillic_SHCHA",
            "Cyrillic_CHE",
            "Release+Cyrillic_HARDSIGN",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_greek_noop_key_names() {
        let names = [
            "Greek_ALPHAaccent",
            "Greek_EPSILONaccent",
            "Greek_ETAaccent",
            "Greek_IOTAaccent",
            "Greek_IOTAdieresis",
            "Greek_IOTAdiaeresis",
            "Greek_OMICRONaccent",
            "Greek_UPSILONaccent",
            "Greek_UPSILONdieresis",
            "Greek_OMEGAaccent",
            "Greek_accentdieresis",
            "Greek_horizbar",
            "Greek_alphaaccent",
            "Greek_epsilonaccent",
            "Greek_etaaccent",
            "Greek_iotaaccent",
            "Greek_iotadieresis",
            "Greek_iotaaccentdieresis",
            "Greek_omicronaccent",
            "Greek_upsilonaccent",
            "Greek_upsilondieresis",
            "Greek_upsilonaccentdieresis",
            "Greek_omegaaccent",
            "Greek_ALPHA",
            "Greek_BETA",
            "Greek_GAMMA",
            "Greek_DELTA",
            "Greek_EPSILON",
            "Greek_ZETA",
            "Greek_ETA",
            "Greek_THETA",
            "Greek_IOTA",
            "Greek_KAPPA",
            "Greek_LAMBDA",
            "Greek_LAMDA",
            "Greek_MU",
            "Greek_NU",
            "Greek_XI",
            "Greek_OMICRON",
            "Greek_PI",
            "Greek_RHO",
            "Greek_SIGMA",
            "Greek_TAU",
            "Greek_UPSILON",
            "Greek_PHI",
            "Greek_CHI",
            "Greek_PSI",
            "Greek_OMEGA",
            "Greek_alpha",
            "Greek_beta",
            "Greek_gamma",
            "Greek_delta",
            "Greek_epsilon",
            "Greek_zeta",
            "Greek_eta",
            "Greek_theta",
            "Greek_iota",
            "Greek_kappa",
            "Greek_lambda",
            "Greek_lamda",
            "Greek_mu",
            "Greek_nu",
            "Greek_xi",
            "Greek_omicron",
            "Greek_pi",
            "Greek_rho",
            "Greek_sigma",
            "Greek_finalsmallsigma",
            "Greek_tau",
            "Greek_upsilon",
            "Greek_phi",
            "Greek_chi",
            "Greek_psi",
            "Release+Greek_omega",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_technical_noop_key_names() {
        let names = [
            "leftradical",
            "topleftradical",
            "horizconnector",
            "topintegral",
            "botintegral",
            "vertconnector",
            "topleftsqbracket",
            "botleftsqbracket",
            "toprightsqbracket",
            "botrightsqbracket",
            "topleftparens",
            "botleftparens",
            "toprightparens",
            "botrightparens",
            "leftmiddlecurlybrace",
            "rightmiddlecurlybrace",
            "topleftsummation",
            "botleftsummation",
            "topvertsummationconnector",
            "botvertsummationconnector",
            "toprightsummation",
            "botrightsummation",
            "rightmiddlesummation",
            "lessthanequal",
            "notequal",
            "greaterthanequal",
            "integral",
            "therefore",
            "variation",
            "infinity",
            "nabla",
            "approximate",
            "similarequal",
            "ifonlyif",
            "implies",
            "identical",
            "radical",
            "includedin",
            "includes",
            "intersection",
            "union",
            "logicaland",
            "logicalor",
            "partialderivative",
            "function",
            "leftarrow",
            "uparrow",
            "rightarrow",
            "downarrow",
            "blank",
            "soliddiamond",
            "checkerboard",
            "ht",
            "ff",
            "cr",
            "lf",
            "nl",
            "vt",
            "lowrightcorner",
            "uprightcorner",
            "upleftcorner",
            "lowleftcorner",
            "crossinglines",
            "horizlinescan1",
            "horizlinescan3",
            "horizlinescan5",
            "horizlinescan7",
            "horizlinescan9",
            "leftt",
            "rightt",
            "bott",
            "topt",
            "Release+vertbar",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_publishing_noop_key_names() {
        let names = [
            "emspace",
            "enspace",
            "em3space",
            "em4space",
            "digitspace",
            "punctspace",
            "thinspace",
            "hairspace",
            "emdash",
            "endash",
            "signifblank",
            "ellipsis",
            "doubbaselinedot",
            "onethird",
            "twothirds",
            "onefifth",
            "twofifths",
            "threefifths",
            "fourfifths",
            "onesixth",
            "fivesixths",
            "careof",
            "figdash",
            "leftanglebracket",
            "decimalpoint",
            "rightanglebracket",
            "marker",
            "oneeighth",
            "threeeighths",
            "fiveeighths",
            "seveneighths",
            "trademark",
            "signaturemark",
            "trademarkincircle",
            "leftopentriangle",
            "rightopentriangle",
            "emopencircle",
            "emopenrectangle",
            "leftsinglequotemark",
            "rightsinglequotemark",
            "leftdoublequotemark",
            "rightdoublequotemark",
            "prescription",
            "minutes",
            "seconds",
            "latincross",
            "hexagram",
            "filledrectbullet",
            "filledlefttribullet",
            "filledrighttribullet",
            "emfilledcircle",
            "emfilledrect",
            "enopencircbullet",
            "enopensquarebullet",
            "openrectbullet",
            "opentribulletup",
            "opentribulletdown",
            "openstar",
            "enfilledcircbullet",
            "enfilledsqbullet",
            "filledtribulletup",
            "filledtribulletdown",
            "leftpointer",
            "rightpointer",
            "club",
            "diamond",
            "heart",
            "maltesecross",
            "dagger",
            "doubledagger",
            "checkmark",
            "ballotcross",
            "musicalsharp",
            "musicalflat",
            "malesymbol",
            "femalesymbol",
            "telephone",
            "telephonerecorder",
            "phonographcopyright",
            "caret",
            "singlelowquotemark",
            "doublelowquotemark",
            "cursor",
            "leftcaret",
            "rightcaret",
            "downcaret",
            "upcaret",
            "overbar",
            "downtack",
            "upshoe",
            "downstile",
            "underbar",
            "jot",
            "quad",
            "uptack",
            "circle",
            "upstile",
            "downshoe",
            "rightshoe",
            "leftshoe",
            "lefttack",
            "Release+righttack",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn parses_librime_hebrew_noop_key_names() {
        let names = [
            "hebrew_doublelowline",
            "hebrew_aleph",
            "hebrew_bet",
            "hebrew_beth",
            "hebrew_gimel",
            "hebrew_gimmel",
            "hebrew_dalet",
            "hebrew_daleth",
            "hebrew_he",
            "hebrew_waw",
            "hebrew_zain",
            "hebrew_zayin",
            "hebrew_chet",
            "hebrew_het",
            "hebrew_tet",
            "hebrew_teth",
            "hebrew_yod",
            "hebrew_finalkaph",
            "hebrew_kaph",
            "hebrew_lamed",
            "hebrew_finalmem",
            "hebrew_mem",
            "hebrew_finalnun",
            "hebrew_nun",
            "hebrew_samech",
            "hebrew_samekh",
            "hebrew_ayin",
            "hebrew_finalpe",
            "hebrew_pe",
            "hebrew_finalzade",
            "hebrew_finalzadi",
            "hebrew_zade",
            "hebrew_zadi",
            "hebrew_kuf",
            "hebrew_qoph",
            "hebrew_resh",
            "hebrew_shin",
            "hebrew_taf",
            "Release+hebrew_taw",
        ];
        let sequence = names
            .iter()
            .map(|name| format!("{{{name}}}"))
            .collect::<String>();
        let keys = parse_key_sequence(&sequence).expect("key sequence should parse");

        assert_eq!(keys.len(), names.len());
        assert!(keys.iter().all(|key| key.code == KeyCode::Ignored));
        assert!(keys[..names.len() - 1]
            .iter()
            .all(|key| key.modifiers.is_empty()));
        assert!(keys[names.len() - 1].modifiers.release);
    }

    #[test]
    fn commits_table_candidate_before_echo_candidate() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        engine.process_char('n');
        engine.process_char('i');

        assert_eq!(engine.context().composition.preedit, "ni");
        assert_eq!(engine.context().candidates[0].text, "你");
        assert_eq!(engine.context().candidates[1].text, "ni");

        let commit = engine.process_char(' ');
        assert_eq!(commit.as_deref(), Some("你"));
    }

    #[test]
    fn numeric_selection_commits_candidate_on_current_page() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

        let commits = engine
            .process_key_sequence("ba2")
            .expect("key sequence should parse");

        assert_eq!(commits, ["吧"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn keypad_numeric_selection_matches_librime_selector_without_text_input() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

        let commits = engine
            .process_key_sequence("{KP_1}ba{KP_2}")
            .expect("key sequence should parse");

        assert_eq!(commits, ["吧"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn shift_keypad_numeric_selection_matches_librime_selector() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

        let commits = engine
            .process_key_sequence("{Shift+KP_2}ba{Shift+KP_2}")
            .expect("key sequence should parse");

        assert_eq!(commits, ["吧"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn shift_ascii_numeric_selection_matches_librime_selector() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

        let commits = engine
            .process_key_sequence("ba{Shift+2}")
            .expect("key sequence should parse");

        assert_eq!(commits, ["吧"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn control_ascii_numeric_selection_matches_librime_selector() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

        let commits = engine
            .process_key_sequence("{Control+2}ba{Control+2}")
            .expect("key sequence should parse");

        assert_eq!(commits, ["吧"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn control_keypad_numeric_selection_matches_librime_selector() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

        let commits = engine
            .process_key_sequence("{Control+KP_2}ba{Control+KP_2}")
            .expect("key sequence should parse");

        assert_eq!(commits, ["吧"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn control_shift_numeric_selection_matches_librime_selector_digit_fallback() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

        let commits = engine
            .process_key_sequence("{Control+Shift+2}{Control+Shift+KP_2}ba{Control+Shift+2}")
            .expect("key sequence should parse");

        assert_eq!(commits, ["吧"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
        assert!(!engine.status().is_composing);

        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

        let commits = engine
            .process_key_sequence("ba{Control+Shift+KP_2}")
            .expect("key sequence should parse");

        assert_eq!(commits, ["吧"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn escape_clears_composition_like_librime_editor_cancel() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        let commits = engine
            .process_key_sequence("ni{Escape}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert!(engine.context().composition.input.is_empty());
        assert!(engine.context().candidates.is_empty());
        assert_eq!(engine.context().last_commit, None);
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn shift_escape_ignores_shift_like_librime_editor_cancel_fallback() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        let commits = engine
            .process_key_sequence("ni{Shift+Escape}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert!(engine.context().composition.input.is_empty());
        assert!(engine.context().candidates.is_empty());
        assert_eq!(engine.context().last_commit, None);
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn delete_key_removes_input_at_caret_like_librime_editor_delete() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        engine.set_input("nix");
        engine.set_caret_pos(2);
        let commits = engine
            .process_key_sequence("{Delete}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["你"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
        assert!(!engine.status().is_composing);

        engine.set_input("ni");
        engine.set_caret_pos(2);
        let commits = engine
            .process_key_sequence("{Delete}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().composition.input, "ni");
        assert_eq!(engine.context().composition.caret, 2);
    }

    #[test]
    fn backspace_removes_input_before_caret_like_librime_editor_back() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        engine.set_input("nxi");
        engine.set_caret_pos(2);
        let commits = engine
            .process_key_sequence("{BackSpace}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["你"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
        assert!(!engine.status().is_composing);

        engine.set_input("ni");
        engine.set_caret_pos(0);
        let commits = engine
            .process_key_sequence("{BackSpace}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().composition.input, "ni");
        assert_eq!(engine.context().composition.caret, 0);
    }

    #[test]
    fn control_backspace_falls_back_to_previous_input_like_librime_editor_back_syllable() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        engine.set_input("nxi");
        engine.set_caret_pos(2);
        let commits = engine
            .process_key_sequence("{Control+BackSpace}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["你"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn shift_backspace_uses_librime_editor_shift_as_control_fallback() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        engine.set_input("nxi");
        engine.set_caret_pos(2);
        let commits = engine
            .process_key_sequence("{Shift+BackSpace}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["你"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn control_return_commits_raw_input_like_librime_fluid_editor() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        let commits = engine
            .process_key_sequence("ni{Control+Return}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ni"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ni"));
        assert!(!engine.status().is_composing);

        let commits = engine
            .process_key_sequence("{Control+Return}")
            .expect("key sequence should parse");
        assert!(commits.is_empty());
    }

    #[test]
    fn shift_return_commits_script_text_like_librime_fluid_editor() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        let commits = engine
            .process_key_sequence("ni{Shift+Return}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ni"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ni"));
        assert!(!engine.status().is_composing);

        let commits = engine
            .process_key_sequence("{Shift+Return}")
            .expect("key sequence should parse");
        assert!(commits.is_empty());
    }

    #[test]
    fn shift_printable_keys_enter_input_and_shift_space_confirms_like_librime_editor() {
        let mut engine = Engine::new();

        let commits = engine
            .process_key_sequence("{Shift+A}b{Shift+space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["Ab"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("Ab"));
        assert!(!engine.status().is_composing);

        let commits = engine
            .process_key_sequence("{Shift+space}")
            .expect("key sequence should parse");
        assert!(commits.is_empty());
        assert_eq!(engine.context().last_commit.as_deref(), Some("Ab"));
    }

    #[test]
    fn modified_keypad_enter_does_not_trigger_librime_return_only_editor_bindings() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        let commits = engine
            .process_key_sequence(
                "ni{Control+KP_Enter}{Shift+KP_Enter}{Control+Shift+KP_Enter}{KP_Enter}",
            )
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["你"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
    }

    #[test]
    fn control_shift_return_commits_selected_comment_like_librime_fluid_editor() {
        let mut engine = Engine::new();
        engine.add_translator(CommentTranslator);

        let commits = engine
            .process_key_sequence("ni{Down}{Control+Shift+Return}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["second-comment"]);
        assert_eq!(
            engine.context().last_commit.as_deref(),
            Some("second-comment")
        );
        assert!(!engine.status().is_composing);

        let commits = engine
            .process_key_sequence("{Control+Shift+Return}")
            .expect("key sequence should parse");
        assert!(commits.is_empty());
    }

    #[test]
    fn left_right_keys_move_caret_like_librime_navigator() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        let commits = engine
            .process_key_sequence("nix{Left}{Delete}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["你"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
        assert!(!engine.status().is_composing);

        engine.set_input("nix");
        engine.set_caret_pos(0);
        let commits = engine
            .process_key_sequence("{Right}{Right}{Delete}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["你"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
    }

    #[test]
    fn control_left_right_jump_across_simplified_syllable_span_like_librime_navigator() {
        let mut engine = Engine::new();

        engine.set_input("nix");
        engine.set_caret_pos(2);
        let commits = engine
            .process_key_sequence("{Control+Left}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().composition.caret, 0);

        let commits = engine
            .process_key_sequence("{Control+Right}{BackSpace}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ni"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ni"));

        engine.set_input("nix");
        let commits = engine
            .process_key_sequence("{Control+Left}{Delete}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ix"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ix"));
    }

    #[test]
    fn shift_left_right_fall_back_to_control_syllable_jump_like_librime_navigator() {
        let mut engine = Engine::new();

        engine.set_input("nix");
        engine.set_caret_pos(2);
        let commits = engine
            .process_key_sequence("{Shift+Left}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().composition.caret, 0);

        let commits = engine
            .process_key_sequence("{Shift+Right}{BackSpace}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ni"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ni"));

        engine.set_input("nix");
        let commits = engine
            .process_key_sequence("{Shift+Left}{Delete}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ix"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ix"));
    }

    #[test]
    fn control_up_down_jump_across_simplified_syllable_span_like_librime_vertical_navigator() {
        let mut engine = Engine::new();

        engine.set_input("nix");
        engine.set_caret_pos(2);
        let commits = engine
            .process_key_sequence("{Control+Up}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().composition.caret, 0);

        let commits = engine
            .process_key_sequence("{Control+Down}{BackSpace}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ni"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ni"));

        engine.set_input("nix");
        let commits = engine
            .process_key_sequence("{Control+Up}{Delete}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ix"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ix"));
    }

    #[test]
    fn shift_up_down_fall_back_to_control_syllable_jump_like_librime_navigator() {
        let mut engine = Engine::new();

        engine.set_input("nix");
        engine.set_caret_pos(2);
        let commits = engine
            .process_key_sequence("{Shift+Up}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().composition.caret, 0);

        let commits = engine
            .process_key_sequence("{Shift+Down}{BackSpace}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ni"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ni"));

        engine.set_input("nix");
        let commits = engine
            .process_key_sequence("{Shift+Up}{Delete}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ix"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ix"));
    }

    #[test]
    fn keypad_left_right_keys_move_caret_by_char_with_librime_navigator_looping() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        engine.set_input("nix");
        engine.set_caret_pos(0);
        let commits = engine
            .process_key_sequence("{KP_Left}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().composition.caret, 3);
        let commits = engine
            .process_key_sequence("{KP_Left}{Delete}{space}")
            .expect("key sequence should parse");
        assert_eq!(commits, vec!["你"]);

        engine.set_input("nix");
        engine.set_caret_pos(3);
        let commits = engine
            .process_key_sequence("{KP_Right}{Delete}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ix"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ix"));
    }

    #[test]
    fn shift_keypad_left_right_ignore_shift_like_librime_navigator() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        engine.set_input("nix");
        engine.set_caret_pos(0);
        let commits = engine
            .process_key_sequence("{Shift+KP_Left}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().composition.caret, 3);
        let commits = engine
            .process_key_sequence("{Shift+KP_Left}{Delete}{space}")
            .expect("key sequence should parse");
        assert_eq!(commits, vec!["你"]);

        engine.set_input("nix");
        engine.set_caret_pos(3);
        let commits = engine
            .process_key_sequence("{Shift+KP_Right}{Delete}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ix"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ix"));
    }

    #[test]
    fn shift_keypad_up_down_ignore_shift_like_librime_navigator() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        engine.set_input("nix");
        engine.set_caret_pos(0);
        let commits = engine
            .process_key_sequence("{Shift+KP_Up}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().composition.caret, 3);
        let commits = engine
            .process_key_sequence("{Shift+KP_Up}{Delete}{space}")
            .expect("key sequence should parse");
        assert_eq!(commits, vec!["你"]);

        engine.set_input("nix");
        engine.set_caret_pos(3);
        let commits = engine
            .process_key_sequence("{Shift+KP_Down}{Delete}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["ix"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("ix"));
    }

    #[test]
    fn page_keys_move_candidate_page_like_librime_selector() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
            ("ba", "巴"),
            ("ba", "把"),
            ("ba", "拔"),
        ]));

        let commits = engine
            .process_key_sequence("{Page_Down}ba{Page_Down}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().highlighted, 5);
        assert_eq!(engine.context().candidates[5].text, "拔");

        engine
            .process_key_sequence("{KP_Page_Up}")
            .expect("key sequence should parse");

        assert_eq!(engine.context().highlighted, 0);
        assert_eq!(engine.context().last_commit, None);
    }

    #[test]
    fn up_down_keys_move_candidate_highlight_like_librime_selector() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
        ]));

        let commits = engine
            .process_key_sequence("{Down}ba{Down}{KP_Down}{KP_Up}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["吧"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn raw_segments_do_not_select_candidates_like_librime_selector() {
        let mut engine = Engine::new();

        assert_eq!(engine.process_char('a'), None);
        engine.set_segment_tags(["raw"]);
        assert_eq!(engine.process_char('1'), None);

        assert_eq!(engine.context().composition.input, "a1");
        assert_eq!(engine.context().last_commit, None);
    }

    #[test]
    fn home_end_keys_reset_candidate_highlight_like_librime_selector() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
        ]));

        let commits = engine
            .process_key_sequence("ba{Down}{Down}{Home}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["八"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("八"));

        let commits = engine
            .process_key_sequence("ba{Down}{KP_End}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["八"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("八"));
    }

    #[test]
    fn home_end_keys_fall_back_to_librime_navigator_caret_movement() {
        let mut engine = Engine::new();

        let commits = engine
            .process_key_sequence("nix{Home}{Delete}{End}{BackSpace}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["i"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("i"));
    }

    #[test]
    fn shift_home_end_keys_ignore_shift_like_librime_navigator() {
        let mut engine = Engine::new();

        engine.set_input("nix");
        let commits = engine
            .process_key_sequence("{Shift+Home}{Delete}{Shift+KP_End}{BackSpace}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, vec!["i"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("i"));

        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));
        engine
            .process_key_sequence("ba{Down}{Shift+Home}")
            .expect("key sequence should parse");

        assert_eq!(engine.context().highlighted, 1);
        assert_eq!(engine.context().composition.caret, 0);
    }

    #[test]
    fn direct_candidate_selection_commits_by_global_or_page_index() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

        engine
            .process_key_sequence("ba")
            .expect("key sequence should parse");
        assert_eq!(engine.select_candidate(1).as_deref(), Some("吧"));
        assert_eq!(engine.context().last_commit.as_deref(), Some("吧"));
        assert!(!engine.status().is_composing);

        engine
            .process_key_sequence("ba")
            .expect("key sequence should parse");
        assert_eq!(
            engine.select_candidate_on_current_page(0).as_deref(),
            Some("八")
        );
        assert_eq!(engine.context().last_commit.as_deref(), Some("八"));
    }

    #[test]
    fn direct_candidate_highlighting_moves_selection_without_committing() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
            ("ba", "巴"),
            ("ba", "把"),
            ("ba", "拔"),
        ]));

        engine
            .process_key_sequence("ba")
            .expect("key sequence should parse");
        assert!(engine.highlight_candidate(1));
        assert_eq!(engine.context().highlighted, 1);
        assert_eq!(engine.context().last_commit, None);
        assert!(!engine.highlight_candidate(99));
        assert_eq!(engine.context().highlighted, 1);

        assert!(engine.change_page(false));
        assert_eq!(engine.context().highlighted, 6);
        assert!(!engine.change_page(false));
        assert_eq!(engine.context().highlighted, 6);
        assert!(engine.highlight_candidate_on_current_page(0));
        assert_eq!(engine.context().highlighted, 5);
        assert!(!engine.highlight_candidate_on_current_page(5));
        assert_eq!(engine.context().highlighted, 5);
        assert!(engine.change_page(true));
        assert_eq!(engine.context().highlighted, 0);
        assert!(!engine.change_page(true));
        assert_eq!(engine.context().highlighted, 0);

        assert_eq!(engine.commit_composition().as_deref(), Some("八"));
    }

    #[test]
    fn direct_candidate_deletion_removes_menu_items_without_committing() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
            ("ba", "巴"),
            ("ba", "把"),
            ("ba", "拔"),
        ]));

        engine
            .process_key_sequence("ba")
            .expect("key sequence should parse");
        assert!(engine.delete_candidate(1));
        assert_eq!(engine.context().candidates[1].text, "爸");
        assert_eq!(engine.context().last_commit, None);
        assert!(!engine.delete_candidate(99));

        assert!(engine.change_page(false));
        assert!(engine.delete_candidate_on_current_page(0));
        assert_eq!(
            engine
                .context()
                .candidates
                .last()
                .map(|candidate| candidate.text.as_str()),
            Some("拔")
        );
        assert!(!engine.delete_candidate_on_current_page(5));
    }

    #[test]
    fn control_delete_removes_highlighted_candidate_like_librime_editor_delete_candidate() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
        ]));

        let commits = engine
            .process_key_sequence("ba{Down}{Control+Delete}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().candidates.len(), 3);
        assert_eq!(engine.context().candidates[1].text, "爸");
        assert_eq!(engine.context().highlighted, 1);
        assert_eq!(engine.context().last_commit, None);
    }

    #[test]
    fn shift_delete_removes_highlighted_candidate_like_librime_editor_shift_as_control_fallback() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([
            ("ba", "八"),
            ("ba", "吧"),
            ("ba", "爸"),
        ]));

        let commits = engine
            .process_key_sequence("ba{Down}{Shift+Delete}")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().candidates.len(), 3);
        assert_eq!(engine.context().candidates[1].text, "爸");
        assert_eq!(engine.context().highlighted, 1);
        assert_eq!(engine.context().last_commit, None);
    }

    #[test]
    fn numeric_selection_consumes_out_of_page_digit_without_extending_input() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ba", "八"), ("ba", "吧")]));

        let commits = engine
            .process_key_sequence("ba0")
            .expect("key sequence should parse");

        assert!(commits.is_empty());
        assert_eq!(engine.context().composition.input, "ba");
        assert_eq!(engine.context().candidates.len(), 3);
    }

    #[test]
    fn rime_checksum_computer_matches_librime_crc32_initial_remainder() {
        assert_eq!(rime_checksum_bytes(b"abc"), 0x359a_672f);

        let mut checksum = RimeChecksumComputer::new(0);
        checksum.process_bytes(b"ab");
        checksum.process_bytes(b"c");
        assert_eq!(checksum.checksum(), 0x359a_672f);

        let mut chained = RimeChecksumComputer::new(0x359a_672f);
        chained.process_bytes(b"def");
        assert_eq!(chained.checksum(), 0x050d_415e);
    }

    #[test]
    fn rime_dict_source_checksum_matches_librime_dict_compiler_ordering() {
        let checksum = rime_dict_source_checksum(
            0,
            [b"dict one\n".as_slice(), b"dict two\n".as_slice()],
            Some(b"vocab\n".as_slice()),
        );
        assert_eq!(checksum, 0x0300_9e82);

        let primary = rime_dict_source_checksum(0, [b"primary\n".as_slice()], None);
        let pack = rime_dict_source_checksum(primary, [b"pack\n".as_slice()], None);
        assert_eq!(pack, 0x9024_58b9);

        assert_eq!(
            rime_dict_source_checksum(
                0x1234_5678,
                std::iter::empty::<&[u8]>(),
                Some(b"ignored vocabulary\n".as_slice()),
            ),
            0x1234_5678
        );
    }

    #[test]
    fn rime_dict_rebuild_plan_matches_librime_compiler_checksum_decisions() {
        let input = RimeDictRebuildInput {
            source_available: true,
            source_dict_file_checksum: 0x1111_1111,
            schema_file_checksum: 0x2222_2222,
            table_dict_file_checksum: Some(0x1111_1111),
            prism: Some(RimePrismChecksumMetadata {
                dict_file_checksum: 0x1111_1111,
                schema_file_checksum: 0x2222_2222,
            }),
            reverse_dict_file_checksum: Some(0x1111_1111),
            force_rebuild_table: false,
            force_rebuild_prism: false,
        };
        assert_eq!(
            rime_dict_rebuild_plan(input),
            Ok(RimeDictRebuildPlan {
                dict_file_checksum: 0x1111_1111,
                rebuild_table: false,
                rebuild_prism: false,
            })
        );

        let changed_source = RimeDictRebuildInput {
            source_dict_file_checksum: 0x3333_3333,
            ..input
        };
        assert_eq!(
            rime_dict_rebuild_plan(changed_source),
            Ok(RimeDictRebuildPlan {
                dict_file_checksum: 0x3333_3333,
                rebuild_table: true,
                rebuild_prism: true,
            })
        );

        let changed_schema = RimeDictRebuildInput {
            schema_file_checksum: 0x4444_4444,
            ..input
        };
        assert_eq!(
            rime_dict_rebuild_plan(changed_schema),
            Ok(RimeDictRebuildPlan {
                dict_file_checksum: 0x1111_1111,
                rebuild_table: false,
                rebuild_prism: true,
            })
        );

        let stale_reverse = RimeDictRebuildInput {
            reverse_dict_file_checksum: Some(0x5555_5555),
            ..input
        };
        assert_eq!(
            rime_dict_rebuild_plan(stale_reverse),
            Ok(RimeDictRebuildPlan {
                dict_file_checksum: 0x1111_1111,
                rebuild_table: true,
                rebuild_prism: false,
            })
        );
    }

    #[test]
    fn rime_dict_rebuild_plan_reuses_table_checksum_when_source_is_missing() {
        let input = RimeDictRebuildInput {
            source_available: false,
            source_dict_file_checksum: 0,
            schema_file_checksum: 0x2222_2222,
            table_dict_file_checksum: Some(0x1111_1111),
            prism: Some(RimePrismChecksumMetadata {
                dict_file_checksum: 0x1111_1111,
                schema_file_checksum: 0x2222_2222,
            }),
            reverse_dict_file_checksum: Some(0x1111_1111),
            force_rebuild_table: true,
            force_rebuild_prism: false,
        };
        assert_eq!(
            rime_dict_rebuild_plan(input),
            Ok(RimeDictRebuildPlan {
                dict_file_checksum: 0x1111_1111,
                rebuild_table: false,
                rebuild_prism: false,
            })
        );

        assert_eq!(
            rime_dict_rebuild_plan(RimeDictRebuildInput {
                table_dict_file_checksum: None,
                ..input
            }),
            Err(RimeDictRebuildError::MissingSourceAndTable)
        );
    }

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

        let mut prism = vec![0; 320];
        put_c_string(&mut prism, 0, b"Rime::Prism/4.0");
        put_u32_le(&mut prism, 32, 0x2222_2222);
        put_u32_le(&mut prism, 36, 0x3333_3333);
        put_u32_le(&mut prism, 40, 17);
        put_u32_le(&mut prism, 44, 19);
        put_u32_le(&mut prism, 48, 23);
        put_u32_le(&mut prism, 52, 0x50);
        assert_eq!(
            parse_rime_prism_bin_metadata(&prism),
            Ok(RimePrismBinMetadata {
                dict_file_checksum: 0x2222_2222,
                schema_file_checksum: 0x3333_3333,
                num_syllables: 17,
                num_spellings: 19,
                double_array_size: 23,
            })
        );

        let mut reverse = vec![0; 64];
        put_c_string(&mut reverse, 0, b"Rime::Reverse/3.1");
        put_u32_le(&mut reverse, 32, 0x4444_4444);
        put_u32_le(&mut reverse, 52, 29);
        put_u32_le(&mut reverse, 60, 31);
        assert_eq!(
            parse_rime_reverse_bin_metadata(&reverse),
            Ok(RimeReverseBinMetadata {
                dict_file_checksum: 0x4444_4444,
                key_trie_size: 29,
                value_trie_size: 31,
            })
        );
    }

    #[test]
    fn compiled_metadata_parser_matches_librime_load_rejection_cases() {
        let mut table = vec![0; 68];
        put_c_string(&mut table, 0, b"Rime::Table/3.0");
        assert_eq!(
            parse_rime_table_bin_metadata(&table),
            Err(RimeCompiledMetadataError::UnsupportedVersion)
        );
        put_c_string(&mut table, 0, b"Rime::Table/4.0");
        put_u32_le(&mut table, 44, 0x40);
        assert_eq!(
            parse_rime_table_bin_metadata(&table),
            Err(RimeCompiledMetadataError::MissingRequiredSection)
        );

        let mut prism = vec![0; 320];
        put_c_string(&mut prism, 0, b"Rime::Prism/3.9");
        assert_eq!(
            parse_rime_prism_bin_metadata(&prism),
            Err(RimeCompiledMetadataError::UnsupportedVersion)
        );
        put_c_string(&mut prism, 0, b"Rime::Prism/4.0");
        assert_eq!(
            parse_rime_prism_bin_metadata(&prism),
            Err(RimeCompiledMetadataError::MissingRequiredSection)
        );

        let mut reverse = vec![0; 64];
        put_c_string(&mut reverse, 0, b"Rime::Reverse/2.9");
        assert_eq!(
            parse_rime_reverse_bin_metadata(&reverse),
            Err(RimeCompiledMetadataError::UnsupportedVersion)
        );
        put_c_string(&mut reverse, 0, b"Rime::Reverse/4.1");
        assert_eq!(
            parse_rime_reverse_bin_metadata(&reverse),
            Err(RimeCompiledMetadataError::UnsupportedVersion)
        );

        let mut invalid = vec![0; 68];
        put_c_string(&mut invalid, 0, b"Rime::Wrong/4.0");
        assert_eq!(
            parse_rime_table_bin_metadata(&invalid),
            Err(RimeCompiledMetadataError::InvalidFormat)
        );
        assert_eq!(
            parse_rime_table_bin_metadata(&invalid[..20]),
            Err(RimeCompiledMetadataError::TooShort)
        );
    }

    fn put_c_string(bytes: &mut [u8], offset: usize, value: &[u8]) {
        bytes[offset..offset + value.len()].copy_from_slice(value);
    }

    fn put_u32_le(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    #[test]
    fn parses_rime_dict_yaml_default_columns_and_weight_order() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: sample
version: "0.1"
sort: by_weight
...

巴	ba	3193
爸	ba	3625
八	ba	6677
"#,
        )
        .expect("dictionary should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[1].text, "爸");
        assert_eq!(entries[2].text, "巴");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 6677.0);
    }

    #[test]
    fn parses_rime_dict_yaml_custom_columns_for_shape_tables() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: cangjie_sample
version: "0.1"
sort: original
columns:
  - text
  - code
  - stem
...

明	ab
晭	abgr	ab'gr
"#,
        )
        .expect("dictionary should parse");

        let entries = dictionary.entries();
        assert_eq!(entries[0].text, "明");
        assert_eq!(entries[0].code, "ab");
        assert_eq!(entries[1].text, "晭");
        assert_eq!(entries[1].code, "abgr");
        assert_eq!(
            dictionary.stems().get("晭").cloned(),
            Some(vec!["ab'gr".to_owned()])
        );
    }

    #[test]
    fn parses_rime_dict_yaml_stem_columns_like_librime_entry_collector() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: stem_sample
version: "0.1"
sort: original
columns: [text, code, stem]
...

明	ab	a'b
明	ab	a'b
明	ac	a'c
未编码		ignored
"#,
        )
        .expect("dictionary should parse");

        assert_eq!(
            dictionary.stems().get("明").cloned(),
            Some(vec!["a'b".to_owned(), "a'c".to_owned()])
        );
        assert!(!dictionary.stems().contains_key("未编码"));
    }

    #[test]
    fn parses_rime_dict_yaml_encoder_settings_like_librime_dict_settings() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: encoder_sample
version: "0.1"
sort: original
encoder:
  exclude_patterns:
    - '^x.*$'
  rules:
    - length_equal: 2
      formula: "AaAzBaBz"
    - length_in_range: [3, 5]
      formula: "AaBaCaZz"
  tail_anchor: "'"
...

甲	abc
乙	def
"#,
        )
        .expect("dictionary should parse");

        let encoder = dictionary.encoder();
        assert!(encoder.loaded());
        assert_eq!(encoder.max_phrase_length(), 5);
        assert_eq!(encoder.rules().len(), 2);
        assert_eq!(encoder.rules()[0].min_word_length, 2);
        assert_eq!(encoder.rules()[0].max_word_length, 2);
        assert_eq!(encoder.rules()[1].min_word_length, 3);
        assert_eq!(encoder.rules()[1].max_word_length, 5);
        assert!(encoder.is_code_excluded("xyz"));
        assert!(!encoder.is_code_excluded("axyz"));
        assert_eq!(
            encoder.encode(&["zyx'wvu'tsr", "qpo'nmlk'jih", "gfedcba"]),
            Some("zqga".to_owned())
        );
    }

    #[test]
    fn parses_rime_dict_yaml_rule_encoder_phrase_entries_like_librime_entry_collector() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
            r#"
---
name: encoder_phrase_sample
version: "0.1"
sort: by_weight
use_preset_vocabulary: true
max_phrase_length: 2
min_phrase_weight: 10
encoder:
  rules:
    - length_equal: 2
      formula: "AaBa"
...

你	ni	10
好	hao	9
您	nin	8
你好		50%
"#,
            std::iter::empty::<&str>(),
            |_| None,
            |name| {
                (name == "essay").then(|| {
                    "\
你好\t12
您好\t11
你好啊\t20
低频\t9
"
                    .to_owned()
                })
            },
        )
        .expect("rule-based encoder phrases should parse");

        let entries = dictionary.entries();
        let encoded_source_phrase = entries
            .iter()
            .find(|entry| entry.text == "你好")
            .expect("source phrase should be encoded");
        assert_eq!(encoded_source_phrase.code, "nh");
        assert_eq!(encoded_source_phrase.weight, 6.0);
        assert!(!entries
            .iter()
            .any(|entry| entry.text == "你好" && entry.code.is_empty()));

        let injected_phrase = entries
            .iter()
            .find(|entry| entry.text == "您好")
            .expect("preset phrase should be injected when all characters are encodable");
        assert_eq!(injected_phrase.code, "nh");
        assert_eq!(injected_phrase.weight, 11.0);
        assert!(!entries.iter().any(|entry| entry.text == "你好啊"));
        assert!(!entries.iter().any(|entry| entry.text == "低频"));
    }

    #[test]
    fn table_encoder_parses_librime_formula_settings() {
        let mut encoder = TableEncoder::new();
        encoder
            .add_length_equal_rule(2, "AaAzBaBz")
            .expect("librime encoder formula should parse");
        encoder
            .add_length_equal_rule(3, "AaBaCaBz")
            .expect("librime encoder formula should parse");
        encoder
            .add_length_in_range_rule(4, 9, "AaBaCaZz")
            .expect("librime encoder formula should parse");

        assert!(encoder.loaded());
        assert_eq!(encoder.max_phrase_length(), 9);
        assert_eq!(encoder.rules().len(), 3);
        assert_eq!(encoder.rules()[0].min_word_length, 2);
        assert_eq!(encoder.rules()[0].max_word_length, 2);
        assert_eq!(
            encoder.rules()[0].coords,
            [
                CodeCoords {
                    char_index: 0,
                    code_index: 0
                },
                CodeCoords {
                    char_index: 0,
                    code_index: -1
                },
                CodeCoords {
                    char_index: 1,
                    code_index: 0
                },
                CodeCoords {
                    char_index: 1,
                    code_index: -1
                },
            ]
        );
        assert_eq!(
            encoder.rules()[2].coords,
            [
                CodeCoords {
                    char_index: 0,
                    code_index: 0
                },
                CodeCoords {
                    char_index: 1,
                    code_index: 0
                },
                CodeCoords {
                    char_index: 2,
                    code_index: 0
                },
                CodeCoords {
                    char_index: -1,
                    code_index: -1
                },
            ]
        );
    }

    #[test]
    fn table_encoder_matches_librime_raw_code_encoding_cases() {
        let code2 = ["abc", "def"];
        let code3 = ["abc", "def", "ghi"];

        let mut encoder = TableEncoder::new();
        encoder
            .add_length_equal_rule(2, "AaAbBaBb")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code2), Some("abde".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder
            .add_length_in_range_rule(3, 5, "AaAzBaBzCaCz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code3), Some("acdfgi".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder
            .add_length_equal_rule(2, "AaAzBaBzCaCz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code2), Some("acdf".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder
            .add_length_equal_rule(2, "AaAbZyZz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code2), Some("abef".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder
            .add_length_equal_rule(2, "AaAaBbBbZzZz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code2), Some("aaeeff".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder
            .add_length_in_range_rule(3, 5, "AzAzByByZaZa")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code3), Some("cceegg".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder
            .add_length_equal_rule(2, "AaBaYaZaZz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code2), Some("adf".to_owned()));
    }

    #[test]
    fn table_encoder_honors_librime_exclude_patterns_and_tail_anchor() {
        let mut encoder = TableEncoder::new();
        encoder
            .set_exclude_patterns(["^x.*$"])
            .expect("exclude regex should compile");
        assert!(encoder.is_code_excluded("x"));
        assert!(encoder.is_code_excluded("xyz"));
        assert!(!encoder.is_code_excluded("XYZ"));
        assert!(!encoder.is_code_excluded("ax"));

        let code = ["zyx'wvu'tsr", "qpo'nmlk'jih", "gfedcba"];

        let mut encoder = TableEncoder::new();
        encoder.set_tail_anchor("'");
        encoder
            .add_length_equal_rule(3, "AaAzBaBzCaCz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code), Some("zxqoga".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder.set_tail_anchor("'");
        encoder
            .add_length_equal_rule(3, "AaAbAcAzBwBxByBz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code), Some("zyxuqpo".to_owned()));

        let mut encoder = TableEncoder::new();
        encoder.set_tail_anchor("'");
        encoder
            .add_length_equal_rule(3, "AaAbAcAdAzBaBxByBz")
            .expect("formula should parse");
        assert_eq!(encoder.encode(&code), Some("zyxwuqpo".to_owned()));
    }

    #[test]
    fn parses_rime_dict_yaml_inline_custom_columns() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: inline_columns_sample
version: "0.1"
sort: original
columns: [code, text, weight]
...

ba	八	10
ba	吧	9
"#,
        )
        .expect("dictionary should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].weight, 10.0);
        assert_eq!(entries[1].code, "ba");
        assert_eq!(entries[1].text, "吧");
    }

    #[test]
    fn parses_rime_dict_yaml_quoted_header_scalars() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: quoted_header_sample
version: "0.1"
sort: 'original'
columns:
  - 'code'
  - "text"
  - 'weight'
...

ba	八	1
ba	吧	9
"#,
        )
        .expect("dictionary should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].weight, 1.0);
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[1].weight, 9.0);
    }

    #[test]
    fn parses_rime_dict_yaml_header_scalars_with_comments() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: commented_header_sample
version: "0.1" # dictionary version
sort: original # preserve source order
columns:
  - code # input code
  - text # candidate text
  - weight # absolute weight
...

ba	八	1
ba	吧	9
"#,
        )
        .expect("dictionary should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].weight, 1.0);
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[1].weight, 9.0);
    }

    #[test]
    fn parses_rime_dict_yaml_block_lists_after_commented_keys() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
name: commented_list_key_primary
version: "0.1"
sort: original
columns: # dictionary field order
  - code
  - text
  - weight
import_tables: # extra tables
  - secondary
...

ba	八	1
"#,
            |name| {
                (name == "secondary").then(|| {
                    r#"
---
name: secondary
version: "0.1"
sort: original
columns: # imported field order
  - code
  - text
  - weight
...

ba	吧	2
"#
                    .to_owned()
                })
            },
        )
        .expect("yaml-cpp accepts comments after block-list mapping keys");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 1.0);
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[1].code, "ba");
        assert_eq!(entries[1].weight, 2.0);
    }

    #[test]
    fn parses_rime_dict_yaml_inline_columns_with_trailing_comment() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: commented_inline_columns_sample
version: "0.1"
sort: original
columns: [code, text, weight] # inline RIME config comment
...

ba	八	10
ba	吧	9
"#,
        )
        .expect("dictionary should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].weight, 10.0);
        assert_eq!(entries[1].text, "吧");
    }

    #[test]
    fn parses_rime_dict_yaml_quoted_empty_required_header_scalars() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: ""
version: ''
sort: original
...

八	ba	1
"#,
        )
        .expect("quoted empty required metadata is a present YAML scalar");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
    }

    #[test]
    fn parses_rime_dict_yaml_text_only_entries_for_later_encoding() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: text_only_sample
version: "0.1"
sort: original
columns: [text, weight]
...

你好	10
你	1
"#,
        )
        .expect("dictionary with text-only entries should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "你好");
        assert_eq!(entries[0].code, "");
        assert_eq!(entries[0].weight, 10.0);
        assert_eq!(entries[1].text, "你");
        assert_eq!(entries[1].code, "");
        assert_eq!(entries[1].weight, 1.0);
    }

    #[test]
    fn parses_rime_dict_yaml_preserves_raw_text_column_whitespace() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: spaced_text_sample
version: "0.1"
sort: original
columns: [code, text, weight]
...

ba	 八 	10
"#,
        )
        .expect("RIME dictionary text fields should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, " 八 ");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 10.0);
    }

    #[test]
    fn parses_rime_dict_yaml_weight_numeric_prefixes() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: weight_prefix_sample
version: "0.1"
sort: original
columns: [code, text, weight]
...

ba	八	10oops
ba	吧	-2.5x
ba	巴	abc
ba	把	50%
"#,
        )
        .expect("dictionary with librime-style row weights should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].weight, 10.0);
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[1].weight, -2.5);
        assert_eq!(entries[2].text, "巴");
        assert_eq!(entries[2].weight, 0.0);
        assert_eq!(entries[3].text, "把");
        assert_eq!(entries[3].weight, 0.0);
    }

    #[test]
    fn parses_rime_dict_yaml_no_comment_marker_as_literal_hash_entries() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: no_comment_sample
version: "0.1"
sort: original
columns: [text, code, weight]
...

# skipped comment
# no comment
#hash	ha	1
#literal	li	2
"#,
        )
        .expect("RIME dictionary '# no comment' marker should allow literal hash entries");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "#hash");
        assert_eq!(entries[0].code, "ha");
        assert_eq!(entries[0].weight, 1.0);
        assert_eq!(entries[1].text, "#literal");
        assert_eq!(entries[1].code, "li");
        assert_eq!(entries[1].weight, 2.0);
    }

    #[test]
    fn parses_rime_dict_yaml_header_keys_with_space_before_colon() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
name : spaced_colon_primary
version : "0.1"
sort : original
columns : [code, text, weight]
import_tables : [secondary]
...

ba	八	1
"#,
            |name| {
                (name == "secondary").then(|| {
                    r#"
---
name : secondary
version : "0.1"
sort : original
columns : [code, text, weight]
...

ba	吧	2
"#
                    .to_owned()
                })
            },
        )
        .expect("yaml-cpp accepts whitespace before mapping-key colons");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[1].code, "ba");
    }

    #[test]
    fn parses_rime_dict_yaml_quoted_header_keys() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
"name": quoted_key_primary
'version': "0.1"
"sort": original
'columns': [code, text, weight]
"import_tables": [secondary]
...

ba	八	1
"#,
            |name| {
                (name == "secondary").then(|| {
                    r#"
---
'name': secondary
"version": "0.1"
"sort": original
'columns': [code, text, weight]
...

ba	吧	2
"#
                    .to_owned()
                })
            },
        )
        .expect("yaml-cpp accepts quoted dictionary header mapping keys");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[1].code, "ba");
    }

    #[test]
    fn parses_rime_dict_yaml_null_columns_as_default_columns() {
        for columns_header in ["columns:", "columns: null", "columns: ~"] {
            let dictionary = TableDictionary::parse_rime_dict_yaml(&format!(
                r#"
---
name: null_columns_sample
version: "0.1"
sort: original
{columns_header}
...

八	ba	10
"#
            ))
            .expect("null columns should use the default RIME column order");

            let entries = dictionary.entries();
            assert_eq!(entries.len(), 1);
            assert_eq!(entries[0].text, "八");
            assert_eq!(entries[0].code, "ba");
            assert_eq!(entries[0].weight, 10.0);
        }
    }

    #[test]
    fn parses_rime_dict_yaml_scalar_columns_as_explicit_empty_list() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: scalar_columns_sample
version: "0.1"
sort: original
columns: text
...

八	ba	10
"#,
        )
        .expect("scalar columns are non-null but not a ConfigList in librime");

        assert!(dictionary.entries().is_empty());
    }

    #[test]
    fn parses_rime_dict_yaml_null_column_items_as_placeholders() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: null_column_item_sample
version: "0.1"
sort: original
columns:
  -
  - text
  - code
  - ''
  - weight
...

ignored	八	ba	ignored	10
"#,
        )
        .expect("YAML-null column items should still occupy a column position");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 10.0);
    }

    #[test]
    fn parses_rime_dict_yaml_inline_null_column_items_as_placeholders() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: inline_null_column_item_sample
version: "0.1"
sort: original
columns: [, text, code, '', weight]
...

ignored	八	ba	ignored	10
"#,
        )
        .expect("inline YAML-null column items should still occupy column positions");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 10.0);
    }

    #[test]
    fn parses_rime_dict_yaml_inline_quoted_commas_as_single_column_items() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: inline_quoted_comma_column_sample
version: "0.1"
sort: original
columns: ['ignored,placeholder', text, code, weight]
...

ignored	八	ba	10
"#,
        )
        .expect("quoted commas in YAML flow lists should not split column items");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 10.0);
    }

    #[test]
    fn parses_rime_dict_yaml_header_without_document_start() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
name: no_document_start_sample
version: "0.1"
sort: original
...

八	ba	10
"#,
        )
        .expect("librime loads dictionary headers as YAML streams without requiring '---'");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 10.0);
    }

    #[test]
    fn parses_rime_dict_yaml_header_with_utf8_bom() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            "\u{feff}name: bom_header_sample\nversion: \"0.1\"\nsort: original\n...\n\n八\tba\t10\n",
        )
        .expect("yaml-cpp accepts a leading UTF-8 BOM before the dictionary header");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 10.0);
    }

    #[test]
    fn parses_rime_dict_yaml_import_tables_with_primary_sorting() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
name: primary
version: "0.1"
sort: by_weight
import_tables:
  - primary
  - secondary
...

八	ba	1
"#,
            |name| {
                (name == "secondary").then(|| {
                    r#"
---
name: secondary
version: "0.1"
sort: original
columns: [code, text, weight]
...

ba	爸	9
ba	吧	3
"#
                    .to_owned()
                })
            },
        )
        .expect("dictionary imports should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].text, "爸");
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[2].text, "八");
    }

    #[test]
    fn parses_rime_dict_yaml_schema_packs_as_optional_tables() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports_and_packs(
            r#"
---
name: primary
version: "0.1"
sort: by_weight
...

爸	ba	1
八	ba	2
"#,
            ["pack", "missing_pack", "broken_pack"],
            |name| match name {
                "pack" => Some(
                    r#"
---
name: pack
version: "0.1"
sort: original
columns: [code, text, weight]
...

ba	爸	9
ba	吧	3
"#
                    .to_owned(),
                ),
                "broken_pack" => Some("name: broken\n".to_owned()),
                _ => None,
            },
        )
        .expect("dictionary packs should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].text, "爸");
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[2].text, "八");
        assert_eq!(entries[3].text, "爸");
        assert_eq!(entries[3].weight, 1.0);
    }

    #[test]
    fn parses_rime_dict_yaml_preset_vocabulary_weights() {
        let mut requested_vocabulary = Vec::new();
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports_packs_and_vocabulary(
            r#"
---
name: primary
version: "0.1"
sort: by_weight
vocabulary: custom
import_tables:
  - secondary
...

八	ba
吧	ba	50%
白	bai	7
"#,
            std::iter::empty::<&str>(),
            |name| {
                (name == "secondary").then(|| {
                    r#"
---
name: secondary
version: "0.1"
sort: original
...

爸	ba
"#
                    .to_owned()
                })
            },
            |name| {
                requested_vocabulary.push(name.to_owned());
                (name == "custom").then(|| {
                    "\
八\t8
吧\t6
爸\t9
"
                    .to_owned()
                })
            },
        )
        .expect("dictionary with preset vocabulary weights should parse");

        let entries = dictionary.entries();
        assert_eq!(requested_vocabulary, ["custom"]);
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].text, "爸");
        assert_eq!(entries[0].weight, 9.0);
        assert_eq!(entries[1].text, "八");
        assert_eq!(entries[1].weight, 8.0);
        assert_eq!(entries[2].text, "吧");
        assert_eq!(entries[2].weight, 3.0);
        assert_eq!(entries[3].text, "白");
        assert_eq!(entries[3].weight, 7.0);
    }

    #[test]
    fn reverse_lookup_translator_uses_target_dictionary_comments() {
        let lookup_dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: stroke
version: "0.1"
sort: original
...

火	huo
水	shui
"#,
        )
        .expect("lookup dictionary should parse");
        let target_dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: luna
version: "0.1"
sort: original
...

火	ho
火	huo
"#,
        )
        .expect("target dictionary should parse");

        let translator =
            ReverseLookupTranslator::new(lookup_dictionary, Some(target_dictionary), "`", "");

        let unprefixed_candidates = translator.translate("huo");
        assert_eq!(unprefixed_candidates.len(), 1);
        assert_eq!(
            unprefixed_candidates[0].source,
            CandidateSource::ReverseLookup
        );
        assert_eq!(unprefixed_candidates[0].text, "火");
        assert_eq!(unprefixed_candidates[0].comment, "ho huo");

        let candidates = translator.translate("`huo");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].source, CandidateSource::ReverseLookup);
        assert_eq!(candidates[0].text, "火");
        assert_eq!(candidates[0].comment, "ho huo");
    }

    #[test]
    fn reverse_lookup_translator_completion_is_opt_in() {
        let lookup_dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: stroke
version: "0.1"
sort: original
...

火	huo
水	shui
"#,
        )
        .expect("lookup dictionary should parse");

        let exact_translator =
            ReverseLookupTranslator::new(lookup_dictionary.clone(), None, "`", "");
        assert!(exact_translator.translate("`hu").is_empty());

        let completion_translator =
            ReverseLookupTranslator::new(lookup_dictionary, None, "`", "").with_completion(true);
        let candidates = completion_translator.translate("`hu");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].text, "火");
        assert_eq!(candidates[0].comment, "huo");
    }

    #[test]
    fn reverse_lookup_translator_honors_librime_segment_tag() {
        let lookup_dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: stroke
version: "0.1"
sort: original
...

火	huo
"#,
        )
        .expect("lookup dictionary should parse");

        let mut engine = Engine::new();
        engine.add_translator(ReverseLookupTranslator::new(
            lookup_dictionary.clone(),
            None,
            "`",
            "",
        ));
        engine.set_input("`huo");
        assert!(engine
            .context()
            .candidates
            .iter()
            .all(|candidate| candidate.source != CandidateSource::ReverseLookup));

        engine.set_segment_tags(["abc", "reverse_lookup"]);
        let reverse_candidates = engine
            .context()
            .candidates
            .iter()
            .filter(|candidate| candidate.source == CandidateSource::ReverseLookup)
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(reverse_candidates, ["火"]);

        let mut tagged_engine = Engine::new();
        tagged_engine.add_translator(
            ReverseLookupTranslator::new(lookup_dictionary, None, "`", "").with_tag("custom"),
        );
        tagged_engine.set_segment_tags(["abc", "reverse_lookup"]);
        tagged_engine.set_input("`huo");
        assert!(tagged_engine
            .context()
            .candidates
            .iter()
            .all(|candidate| candidate.source != CandidateSource::ReverseLookup));

        tagged_engine.set_segment_tags(["abc", "custom"]);
        let reverse_candidates = tagged_engine
            .context()
            .candidates
            .iter()
            .filter(|candidate| candidate.source == CandidateSource::ReverseLookup)
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(reverse_candidates, ["火"]);
    }

    #[test]
    fn history_translator_returns_recent_commits_for_configured_input() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你"), ("hao", "好")]));
        engine.add_translator(HistoryTranslator::new("his").with_size(2));

        engine.set_input("ni");
        assert_eq!(engine.commit_highlighted(), Some("你".to_owned()));
        engine.set_input("hao");
        assert_eq!(engine.commit_highlighted(), Some("好".to_owned()));

        engine.set_input("hi");
        assert_eq!(engine.context().candidates[0].text, "hi");

        engine.set_input("his");
        let history_candidates = engine
            .context()
            .candidates
            .iter()
            .take(2)
            .map(|candidate| (candidate.text.as_str(), &candidate.source))
            .collect::<Vec<_>>();
        assert_eq!(
            history_candidates,
            [
                ("好", &CandidateSource::History),
                ("你", &CandidateSource::History)
            ]
        );

        let mut tagged_engine = Engine::new();
        tagged_engine.add_translator(StaticTableTranslator::new([("ni", "你")]));
        tagged_engine.add_translator(HistoryTranslator::new("his").with_tag("custom"));
        tagged_engine.set_input("ni");
        assert_eq!(tagged_engine.commit_highlighted(), Some("你".to_owned()));
        tagged_engine.set_input("his");
        assert!(tagged_engine
            .context()
            .candidates
            .iter()
            .all(|candidate| candidate.source != CandidateSource::History));

        tagged_engine.set_segment_tags(["abc", "custom"]);
        let history_candidates = tagged_engine
            .context()
            .candidates
            .iter()
            .filter(|candidate| candidate.source == CandidateSource::History)
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(history_candidates, ["你"]);
    }

    #[test]
    fn reverse_lookup_filter_updates_comments_like_librime() {
        let reverse_dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: stroke
version: "0.1"
sort: original
...

你	wq
好	vb
"#,
        )
        .expect("reverse lookup dictionary should parse");

        let default_filter = ReverseLookupFilter::new(reverse_dictionary.clone());
        let mut candidates = vec![
            Candidate {
                text: "你".to_owned(),
                comment: "ni".to_owned(),
                source: CandidateSource::Table,
                quality: 1.0,
            },
            Candidate {
                text: "好".to_owned(),
                comment: String::new(),
                source: CandidateSource::Table,
                quality: 1.0,
            },
            Candidate {
                text: "你".to_owned(),
                comment: String::new(),
                source: CandidateSource::Completion,
                quality: 0.5,
            },
            Candidate {
                text: "你好".to_owned(),
                comment: " ☯ ".to_owned(),
                source: CandidateSource::Sentence,
                quality: 2.0,
            },
        ];
        default_filter.apply(&mut candidates);
        assert_eq!(candidates[0].comment, "ni");
        assert_eq!(candidates[1].comment, "vb");
        assert_eq!(candidates[2].comment, "wq");
        assert_eq!(candidates[3].comment, " ☯ ");

        let mut sentence_candidates = vec![Candidate {
            text: "你好".to_owned(),
            comment: " ☯ ".to_owned(),
            source: CandidateSource::Sentence,
            quality: 2.0,
        }];
        ReverseLookupFilter::new(
            TableDictionary::parse_rime_dict_yaml(
                r#"
---
name: sentence_codes
version: "0.1"
sort: original
...

你好	wh
"#,
            )
            .expect("sentence reverse lookup dictionary should parse"),
        )
        .with_overwrite_comment(true)
        .apply(&mut sentence_candidates);
        assert_eq!(sentence_candidates[0].comment, "wh");

        let mut overwrite_engine = Engine::new();
        overwrite_engine.add_translator(StaticTableTranslator::new([("ni", "你")]));
        overwrite_engine.add_filter(
            ReverseLookupFilter::new(reverse_dictionary.clone()).with_overwrite_comment(true),
        );
        overwrite_engine
            .process_key_sequence("ni")
            .expect("keys should parse");
        assert_eq!(overwrite_engine.context().candidates[0].comment, "wq");

        let mut append_engine = Engine::new();
        append_engine.add_translator(StaticTableTranslator::new([("ni", "你")]));
        append_engine
            .add_filter(ReverseLookupFilter::new(reverse_dictionary).with_append_comment(true));
        append_engine
            .process_key_sequence("ni")
            .expect("keys should parse");
        assert_eq!(append_engine.context().candidates[0].comment, "ni wq");
    }

    #[test]
    fn uniquifier_filter_removes_later_duplicate_candidate_texts() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你"), ("ni", "呢")]));
        engine.add_translator(StaticTableTranslator::new([("ni", "你"), ("ni", "ni")]));
        engine.add_filter(UniquifierFilter);

        engine
            .process_key_sequence("ni")
            .expect("keys should parse");

        let texts = engine
            .context()
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, ["你", "呢", "ni"]);
    }

    #[test]
    fn single_char_filter_moves_table_single_characters_before_phrases() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([
            ("ni", "你好"),
            ("ni", "你"),
            ("ni", "呢"),
            ("ni", "你们"),
        ]));
        engine.add_filter(SingleCharFilter);

        engine
            .process_key_sequence("ni")
            .expect("keys should parse");

        let candidates = engine.context().candidates.iter().collect::<Vec<_>>();
        let texts = candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        let sources = candidates
            .iter()
            .map(|candidate| candidate.source.clone())
            .collect::<Vec<_>>();
        assert_eq!(texts, ["你", "呢", "你好", "你们", "ni"]);
        assert_eq!(
            sources,
            [
                CandidateSource::Table,
                CandidateSource::Table,
                CandidateSource::Table,
                CandidateSource::Table,
                CandidateSource::Echo,
            ]
        );
    }

    #[test]
    fn charset_filter_removes_extended_cjk_until_option_enabled() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([
            ("ni", "你"),
            ("ni", "㐀"),
            ("ni", "𠀀"),
            ("ni", "㍿"),
        ]));
        engine.add_filter(CharsetFilter);

        engine
            .process_key_sequence("ni")
            .expect("keys should parse");

        let texts = engine
            .context()
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, ["你", "ni"]);

        engine.set_option("extended_charset", true);
        let texts = engine
            .context()
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, ["你", "㐀", "𠀀", "㍿", "ni"]);
    }

    #[test]
    fn static_table_translator_charset_filter_matches_librime_option() {
        let mut engine = Engine::new();
        engine.add_translator(
            StaticTableTranslator::new([("ni", "你"), ("ni", "㐀"), ("ni", "𠀀"), ("ni", "㍿")])
                .with_charset_filter(true),
        );

        engine
            .process_key_sequence("ni")
            .expect("keys should parse");

        let texts = engine
            .context()
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, ["你", "ni"]);

        engine.set_option("extended_charset", true);
        let texts = engine
            .context()
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, ["你", "㐀", "𠀀", "㍿", "ni"]);
    }

    #[test]
    fn static_table_translator_trims_trailing_librime_delimiters() {
        let mut engine = Engine::new();
        engine.add_translator(
            StaticTableTranslator::new([("ba", "爸"), ("ban", "班")]).with_delimiters("'"),
        );

        engine.process_char('b');
        engine.process_char('a');
        engine.process_char('\'');

        let texts = engine
            .context()
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, ["爸", "ba'"]);
        assert_eq!(engine.context().composition.preedit, "ba'");
    }

    #[test]
    fn static_table_translator_completion_matches_librime_option() {
        let mut exact_engine = Engine::new();
        exact_engine.add_translator(StaticTableTranslator::new([("ba", "爸"), ("ban", "班")]));
        exact_engine.process_char('b');

        let exact_texts = exact_engine
            .context()
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(exact_texts, ["b"]);

        let mut completion_engine = Engine::new();
        completion_engine.add_translator(
            StaticTableTranslator::new([("ba", "爸"), ("ban", "班")]).with_completion(true),
        );
        completion_engine.process_char('b');

        let candidates = &completion_engine.context().candidates;
        let texts = candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        let sources = candidates
            .iter()
            .map(|candidate| candidate.source.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, ["爸", "班", "b"]);
        assert_eq!(sources, ["completion", "completion", "echo"]);
    }

    #[test]
    fn static_table_translator_honors_librime_segment_tags() {
        let mut custom_tag_engine = Engine::new();
        custom_tag_engine.add_translator(
            StaticTableTranslator::new([("ba", "爸"), ("ban", "班")])
                .with_completion(true)
                .with_tags(["custom"]),
        );
        custom_tag_engine.process_char('b');

        let texts = custom_tag_engine
            .context()
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, ["b"]);

        custom_tag_engine.set_segment_tags(["abc", "custom"]);
        let texts = custom_tag_engine
            .context()
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, ["爸", "班", "b"]);

        let mut abc_tag_engine = Engine::new();
        abc_tag_engine.add_translator(
            StaticTableTranslator::new([("ba", "爸"), ("ban", "班")])
                .with_completion(true)
                .with_tags(["custom", "abc"]),
        );
        abc_tag_engine.process_char('b');

        let texts = abc_tag_engine
            .context()
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, ["爸", "班", "b"]);
    }

    #[test]
    fn static_table_translator_sentence_fallback_matches_librime_option() {
        let mut disabled_engine = Engine::new();
        disabled_engine.add_translator(
            StaticTableTranslator::new([("ba", "爸"), ("bao", "包")]).with_sentence(false),
        );
        disabled_engine
            .process_key_sequence("babao")
            .expect("key sequence should parse");

        let texts = disabled_engine
            .context()
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, ["babao"]);

        let mut enabled_engine = Engine::new();
        enabled_engine.add_translator(
            StaticTableTranslator::new([("ba", "爸"), ("bao", "包")])
                .with_sentence(true)
                .with_delimiters("'"),
        );
        enabled_engine
            .process_key_sequence("ba'bao")
            .expect("key sequence should parse");

        let candidates = &enabled_engine.context().candidates;
        let texts = candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        let sources = candidates
            .iter()
            .map(|candidate| candidate.source.as_str())
            .collect::<Vec<_>>();
        let comments = candidates
            .iter()
            .map(|candidate| candidate.comment.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, ["爸包", "ba'bao"]);
        assert_eq!(sources, ["sentence", "echo"]);
        assert_eq!(comments[0], " ☯ ");
    }

    #[test]
    fn static_table_translator_sentence_over_completion_prioritizes_sentence() {
        let mut engine = Engine::new();
        engine.add_translator(
            StaticTableTranslator::new([("ba", "爸"), ("baban", "巴班")])
                .with_completion(true)
                .with_sentence_over_completion(true),
        );
        engine
            .process_key_sequence("baba")
            .expect("key sequence should parse");

        let candidates = &engine.context().candidates;
        let texts = candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        let sources = candidates
            .iter()
            .map(|candidate| candidate.source.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, ["爸爸", "巴班", "baba"]);
        assert_eq!(sources, ["sentence", "completion", "echo"]);
    }

    #[test]
    fn static_table_translator_initial_quality_participates_in_candidate_order() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ba", "低")]));
        engine
            .add_translator(StaticTableTranslator::new([("ba", "高")]).with_initial_quality(10.0));

        engine
            .process_key_sequence("ba")
            .expect("key sequence should parse");

        let candidates = &engine.context().candidates;
        let texts = candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, ["高", "低", "ba"]);
        assert_eq!(candidates[0].quality, 11.0);
        assert_eq!(candidates[1].quality, 1.0);
    }

    #[test]
    fn static_table_translator_applies_librime_comment_format() {
        let formulas = vec![
            "xlit/ab/AB/".to_owned(),
            "xform/^/[/".to_owned(),
            "xform/$/]/".to_owned(),
        ];
        let mut engine = Engine::new();
        engine.add_translator(
            StaticTableTranslator::new([("ba", "爸"), ("ban", "班")])
                .with_completion(true)
                .with_comment_format(&formulas),
        );

        engine.process_char('b');

        let comments = engine
            .context()
            .candidates
            .iter()
            .map(|candidate| candidate.comment.as_str())
            .collect::<Vec<_>>();
        assert_eq!(comments, ["[BA]", "[BAn]", "echo"]);
    }

    #[test]
    fn static_table_translator_applies_librime_dictionary_exclude() {
        let mut engine = Engine::new();
        engine.add_translator(
            StaticTableTranslator::new([("ba", "爸"), ("ban", "班"), ("bao", "包")])
                .with_completion(true)
                .with_dictionary_exclude(["爸", "班"]),
        );

        engine.process_char('b');

        let texts = engine
            .context()
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        let sources = engine
            .context()
            .candidates
            .iter()
            .map(|candidate| candidate.source.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, ["包", "b"]);
        assert_eq!(sources, ["completion", "echo"]);
    }

    #[test]
    fn static_table_translator_expands_librime_spelling_algebra() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: algebra
version: "0.1"
sort: original
...

略	lue	0
女	nv	0
病	bing	0
平	pin	0
长	chang	0
错	cuo	0
照	zyx	0
删	gone	0
"#,
        )
        .expect("dictionary should parse");
        let formulas = vec![
            "xlit/zyx/abc/".to_owned(),
            "xform/^lue$/lve/".to_owned(),
            "derive/^nv$/nu/".to_owned(),
            "fuzz/^bing$/pin/".to_owned(),
            "abbrev/^chang$/c/".to_owned(),
            "derive/^cuo$/cu/correction".to_owned(),
            "erase/^gone$/".to_owned(),
        ];
        let translator =
            StaticTableTranslator::from_dictionary(dictionary).with_spelling_algebra(&formulas);

        assert_eq!(translator.translate("lue").len(), 0);

        let lve = translator.translate("lve");
        assert_eq!(lve[0].text, "略");
        assert_eq!(lve[0].comment, "lue");

        let nv = translator.translate("nv");
        assert_eq!(nv[0].text, "女");
        assert_eq!(nv[0].comment, "nv");

        let nu = translator.translate("nu");
        assert_eq!(nu[0].text, "女");
        assert_eq!(nu[0].comment, "nv");

        let pin = translator.translate("pin");
        let exact = pin
            .iter()
            .find(|candidate| candidate.text == "平")
            .expect("normal spelling candidate should be present");
        let fuzzy = pin
            .iter()
            .find(|candidate| candidate.text == "病")
            .expect("fuzzy spelling candidate should be present");
        assert_eq!(exact.quality, 1.0);
        assert!((fuzzy.quality - 0.5).abs() < f32::EPSILON);
        assert!(exact.quality > fuzzy.quality);

        let abbreviation = translator.translate("c");
        assert_eq!(abbreviation[0].text, "长");
        assert!((abbreviation[0].quality - 0.5).abs() < f32::EPSILON);

        let correction = translator.translate("cu");
        assert_eq!(correction[0].text, "错");
        assert!((correction[0].quality - 0.01).abs() < 0.000_001);

        let transliterated = translator.translate("abc");
        assert_eq!(transliterated[0].text, "照");
        assert_eq!(transliterated[0].comment, "zyx");
        assert_eq!(translator.translate("zyx").len(), 0);
        assert_eq!(translator.translate("gone").len(), 0);
    }

    #[test]
    fn simplifier_filter_converts_text_when_option_enabled() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("tw", "臺灣"), ("tw", "龍馬")]));
        engine.add_filter(SimplifierFilter::new());

        engine
            .process_key_sequence("tw")
            .expect("keys should parse");

        let texts = engine
            .context()
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, ["臺灣", "龍馬", "tw"]);

        engine.set_option("simplification", true);
        let texts = engine
            .context()
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        let comments = engine
            .context()
            .candidates
            .iter()
            .map(|candidate| candidate.comment.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, ["台湾", "龙马", "tw"]);
        assert_eq!(comments, ["tw", "tw", "echo"]);
    }

    #[test]
    fn simplifier_filter_honors_custom_option_name() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("tw", "臺灣")]));
        engine.add_filter(SimplifierFilter::new().with_option_name("zh_simp"));

        engine
            .process_key_sequence("tw")
            .expect("keys should parse");

        engine.set_option("simplification", true);
        assert_eq!(engine.context().candidates[0].text, "臺灣");

        engine.set_option("zh_simp", true);
        assert_eq!(engine.context().candidates[0].text, "台湾");
    }

    #[test]
    fn tagged_filter_matches_librime_filter_tags() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("tw", "臺灣")]));
        engine.add_filter(TaggedFilter::new(SimplifierFilter::new(), ["custom"]));

        engine
            .process_key_sequence("tw")
            .expect("keys should parse");
        engine.set_option("simplification", true);

        assert_eq!(engine.context().candidates[0].text, "臺灣");

        engine.set_segment_tags(["abc", "custom"]);
        assert_eq!(engine.context().candidates[0].text, "台湾");
    }

    #[test]
    fn simplifier_filter_honors_librime_opencc_config() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("tw", "台灣"), ("tw", "裏")]));
        engine.add_filter(
            SimplifierFilter::new()
                .with_option_name("zh_tw")
                .with_opencc_config("t2tw.json"),
        );

        engine
            .process_key_sequence("tw")
            .expect("keys should parse");

        engine.set_option("simplification", true);
        assert_eq!(engine.context().candidates[0].text, "台灣");

        engine.set_option("zh_tw", true);
        let texts = engine
            .context()
            .candidates
            .iter()
            .map(|candidate| candidate.text.as_str())
            .collect::<Vec<_>>();
        assert_eq!(texts, ["臺灣", "裡", "tw"]);
    }

    #[test]
    fn simplifier_filter_shows_librime_tip_comments() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("tw", "臺"), ("tw", "臺灣")]));
        engine.add_filter(SimplifierFilter::new().with_tips("char"));

        engine
            .process_key_sequence("tw")
            .expect("keys should parse");
        engine.set_option("simplification", true);

        assert_eq!(engine.context().candidates[0].text, "台");
        assert_eq!(engine.context().candidates[0].comment, "〔臺〕");
        assert_eq!(engine.context().candidates[1].text, "台湾");
        assert_eq!(engine.context().candidates[1].comment, "tw");

        let mut all_tips_engine = Engine::new();
        let formulas = vec!["xform/^/[/".to_owned(), "xform/$/]/".to_owned()];
        all_tips_engine.add_translator(StaticTableTranslator::new([("tw", "臺灣")]));
        all_tips_engine.add_filter(
            SimplifierFilter::new()
                .with_tips("all")
                .with_comment_format(&formulas),
        );

        all_tips_engine
            .process_key_sequence("tw")
            .expect("keys should parse");
        all_tips_engine.set_option("simplification", true);

        assert_eq!(all_tips_engine.context().candidates[0].text, "台湾");
        assert_eq!(all_tips_engine.context().candidates[0].comment, "[臺灣]");
    }

    #[test]
    fn simplifier_filter_can_show_converted_text_in_comment() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("tw", "臺灣")]));
        engine.add_filter(
            SimplifierFilter::new()
                .with_tips("all")
                .with_show_in_comment(true),
        );

        engine
            .process_key_sequence("tw")
            .expect("keys should parse");
        engine.set_option("simplification", true);

        assert_eq!(engine.context().candidates[0].text, "臺灣");
        assert_eq!(engine.context().candidates[0].comment, "台湾");
    }

    #[test]
    fn simplifier_filter_honors_librime_excluded_types() {
        let filter = SimplifierFilter::new().with_excluded_types(["table".to_owned()]);
        let mut options = std::collections::HashMap::new();
        options.insert("simplification".to_owned(), true);
        let mut candidates = vec![
            Candidate {
                text: "臺灣".to_owned(),
                comment: "tw".to_owned(),
                source: CandidateSource::Table,
                quality: 1.0,
            },
            Candidate {
                text: "龍".to_owned(),
                comment: String::new(),
                source: CandidateSource::Punctuation,
                quality: 1.0,
            },
        ];

        filter.apply_with_options(&mut candidates, &options);

        assert_eq!(candidates[0].text, "臺灣");
        assert_eq!(candidates[0].comment, "tw");
        assert_eq!(candidates[1].text, "龙");
    }

    #[test]
    fn reverse_lookup_comment_format_applies_projection_formulas() {
        let lookup_dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: stroke
version: "0.1"
sort: original
...

你	wq
"#,
        )
        .expect("lookup dictionary should parse");
        let target_dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: luna
version: "0.1"
sort: original
...

你	ni
"#,
        )
        .expect("target dictionary should parse");
        let formulas = vec![
            "xlit/abcdefghijklmnopqrstuvwxyz/ABCDEFGHIJKLMNOPQRSTUVWXYZ/".to_owned(),
            "xform/^/〔/".to_owned(),
            "xform/$/〕/".to_owned(),
        ];

        let translator =
            ReverseLookupTranslator::new(lookup_dictionary, Some(target_dictionary), "", "")
                .with_comment_format(&formulas);
        let candidates = translator.translate("wq");
        assert_eq!(candidates[0].comment, "〔NI〕");

        let reverse_dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: stroke
version: "0.1"
sort: original
...

你	wq
"#,
        )
        .expect("reverse lookup dictionary should parse");
        let filter = ReverseLookupFilter::new(reverse_dictionary).with_comment_format(&formulas);
        let mut candidates = vec![Candidate {
            text: "你".to_owned(),
            comment: String::new(),
            source: CandidateSource::Table,
            quality: 1.0,
        }];
        filter.apply(&mut candidates);
        assert_eq!(candidates[0].comment, "〔WQ〕");
    }

    #[test]
    fn parses_rime_dict_yaml_skips_null_import_tables() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
name: primary
version: "0.1"
sort: original
import_tables: [null, ~, secondary, 'null']
...

八	ba	1
"#,
            |name| match name {
                "secondary" => Some(
                    r#"
---
name: secondary
version: "0.1"
sort: original
...

吧	ba	2
"#
                    .to_owned(),
                ),
                "null" => Some(
                    r#"
---
name: 'null'
version: "0.1"
sort: original
...

爸	ba	3
"#
                    .to_owned(),
                ),
                _ => None,
            },
        )
        .expect("YAML-null import tables should be skipped like librime config nodes");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[2].text, "爸");
    }

    #[test]
    fn parses_rime_dict_yaml_unescapes_quoted_import_table_names() {
        let mut requested_imports = Vec::new();
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
name: escaped_import_sample
version: "0.1"
sort: original
import_tables: ['sec''ondary', "third\"table", "hex\x5ftable", "unicode\u005ftable", "long\U0000005ftable"]
...

primary	pri	1
"#,
            |table| {
                requested_imports.push(table.to_owned());
                match table {
                    "sec'ondary" => Some(
                        r#"
---
name: "sec'ondary"
version: "0.1"
...

single quote	sq	2
"#
                        .to_owned(),
                    ),
                    "third\"table" => Some(
                        r#"
---
name: 'third"table'
version: "0.1"
...

double quote	dq	3
"#
                        .to_owned(),
                    ),
                    "hex_table" => Some(
                        r#"
---
name: hex_table
version: "0.1"
...

hex escape	he	4
"#
                        .to_owned(),
                    ),
                    "unicode_table" => Some(
                        r#"
---
name: unicode_table
version: "0.1"
...

unicode escape	ue	5
"#
                        .to_owned(),
                    ),
                    "long_table" => Some(
                        r#"
---
name: long_table
version: "0.1"
...

long unicode escape	le	6
"#
                        .to_owned(),
                    ),
                    _ => None,
                }
            },
        )
        .expect("quoted YAML import table names should be unescaped like yaml-cpp scalars");

        assert_eq!(
            requested_imports,
            [
                "sec'ondary",
                "third\"table",
                "hex_table",
                "unicode_table",
                "long_table"
            ]
        );
        let entries = dictionary.entries();
        assert_eq!(entries[0].text, "primary");
        assert_eq!(entries[1].text, "single quote");
        assert_eq!(entries[2].text, "double quote");
        assert_eq!(entries[3].text, "hex escape");
        assert_eq!(entries[4].text, "unicode escape");
        assert_eq!(entries[5].text, "long unicode escape");
    }

    #[test]
    fn parses_rime_dict_yaml_skips_collection_import_tables() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
name: primary
version: "0.1"
sort: original
import_tables: [[ignored, missing], {name: skipped}, secondary, '[literal]']
...

八	ba	1
"#,
            |name| match name {
                "secondary" => Some(
                    r#"
---
name: secondary
version: "0.1"
sort: original
...

吧	ba	2
"#
                    .to_owned(),
                ),
                "[literal]" => Some(
                    r#"
---
name: '[literal]'
version: "0.1"
sort: original
...

爸	ba	3
"#
                    .to_owned(),
                ),
                other => panic!("non-scalar import table should be skipped, got {other}"),
            },
        )
        .expect("non-scalar import table items should be skipped like librime config nodes");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[2].text, "爸");
    }

    #[test]
    fn parses_rime_dict_yaml_drops_duplicate_word_code_definitions() {
        let dictionary = TableDictionary::parse_rime_dict_yaml_with_imports(
            r#"
---
name: primary
version: "0.1"
sort: original
import_tables: [secondary]
...

八	ba	1
八	ba	99
"#,
            |name| {
                (name == "secondary").then(|| {
                    r#"
---
name: secondary
version: "0.1"
sort: original
...

八	ba	88
吧	ba	3
"#
                    .to_owned()
                })
            },
        )
        .expect("dictionary imports should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].text, "八");
        assert_eq!(entries[0].code, "ba");
        assert_eq!(entries[0].weight, 1.0);
        assert_eq!(entries[1].text, "吧");
        assert_eq!(entries[1].code, "ba");
    }

    #[test]
    fn parses_rime_dict_yaml_preserves_duplicate_phrase_code_definitions() {
        let dictionary = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: phrase_duplicate_sample
version: "0.1"
sort: original
...

你好	ni hao	1
你好	ni hao	2
你	ni	3
你	ni	4
"#,
        )
        .expect("dictionary with duplicate phrase code definitions should parse");

        let entries = dictionary.entries();
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].text, "你好");
        assert_eq!(entries[0].code, "nihao");
        assert_eq!(entries[0].weight, 1.0);
        assert_eq!(entries[1].text, "你好");
        assert_eq!(entries[1].code, "nihao");
        assert_eq!(entries[1].weight, 2.0);
        assert_eq!(entries[2].text, "你");
        assert_eq!(entries[2].code, "ni");
        assert_eq!(entries[2].weight, 3.0);
    }

    #[test]
    fn rejects_rime_dict_yaml_with_incomplete_header() {
        let missing_name = TableDictionary::parse_rime_dict_yaml(
            r#"
---
version: "0.1"
sort: by_weight
...

八	ba	1
"#,
        )
        .expect_err("dictionary without a name should be rejected");
        assert_eq!(
            missing_name.to_string(),
            "RIME dictionary header is missing required name or version"
        );

        let missing_version = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: incomplete_sample
sort: by_weight
...

八	ba	1
"#,
        )
        .expect_err("dictionary without a version should be rejected");
        assert_eq!(
            missing_version.to_string(),
            "RIME dictionary header is missing required name or version"
        );

        let commented_blank_version = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: incomplete_sample
version: # dictionary version is missing
sort: by_weight
...

八	ba	1
"#,
        )
        .expect_err("dictionary with a blank commented version should be rejected");
        assert_eq!(
            commented_blank_version.to_string(),
            "RIME dictionary header is missing required name or version"
        );

        let null_name = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: null
version: "0.1"
sort: by_weight
...

八	ba	1
"#,
        )
        .expect_err("dictionary with YAML null name should be rejected");
        assert_eq!(
            null_name.to_string(),
            "RIME dictionary header is missing required name or version"
        );

        let null_version = TableDictionary::parse_rime_dict_yaml(
            r#"
---
name: incomplete_sample
version: ~
sort: by_weight
...

八	ba	1
"#,
        )
        .expect_err("dictionary with YAML null version should be rejected");
        assert_eq!(
            null_version.to_string(),
            "RIME dictionary header is missing required name or version"
        );
    }

    #[test]
    fn table_translator_can_commit_rime_dictionary_phrase_codes() {
        let mut engine = Engine::new();
        let translator = StaticTableTranslator::parse_rime_dict_yaml(
            r#"
---
name: sample
version: "0.1"
sort: by_weight
...

你	ni	1
你好	ni hao	10
"#,
        )
        .expect("dictionary should parse");
        engine.add_translator(translator);

        let commits = engine
            .process_key_sequence("nihao{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, ["你好"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("你好"));
    }

    #[test]
    fn explicit_composition_control_commits_or_clears_active_input() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        engine
            .process_key_sequence("ni")
            .expect("key sequence should parse");
        assert_eq!(engine.commit_composition().as_deref(), Some("你"));
        assert!(!engine.status().is_composing);
        assert_eq!(engine.context().last_commit.as_deref(), Some("你"));

        engine
            .process_key_sequence("hao")
            .expect("key sequence should parse");
        engine.clear_composition();
        assert!(!engine.status().is_composing);
        assert!(engine.context().candidates.is_empty());
        assert_eq!(engine.context().last_commit.as_deref(), Some("你"));
        assert_eq!(engine.commit_composition(), None);
    }

    #[test]
    fn direct_input_control_rebuilds_candidates_and_clamps_caret() {
        let mut engine = Engine::new();
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        engine.set_input("ni");

        assert_eq!(engine.context().composition.input, "ni");
        assert_eq!(engine.context().composition.preedit, "ni");
        assert_eq!(engine.context().composition.caret, 2);
        assert_eq!(engine.context().candidates[0].text, "你");

        engine.set_caret_pos(1);
        assert_eq!(engine.context().composition.caret, 1);
        engine.set_caret_pos(10);
        assert_eq!(engine.context().composition.caret, 2);
    }

    #[test]
    fn runtime_options_update_status_flags_and_preserve_custom_values() {
        let mut engine = Engine::new();

        assert!(!engine.get_option("ascii_mode"));
        engine.set_option("ascii_mode", true);
        engine.set_option("custom_toggle", true);

        let status = engine.status();
        assert!(status.is_ascii_mode);
        assert!(engine.get_option("ascii_mode"));
        assert!(engine.get_option("custom_toggle"));

        engine.set_option("ascii_mode", false);
        assert!(!engine.status().is_ascii_mode);
        assert!(!engine.get_option("ascii_mode"));
        assert!(!engine.get_option("unknown_toggle"));
    }

    #[test]
    fn runtime_properties_store_session_strings() {
        let mut engine = Engine::new();

        assert_eq!(engine.get_property("client_app"), None);

        engine.set_property("client_app", "sample_console");
        engine.set_property("inline_preedit", "");

        assert_eq!(engine.get_property("client_app"), Some("sample_console"));
        assert_eq!(engine.get_property("inline_preedit"), Some(""));
    }

    #[test]
    fn mock_ai_ranker_can_reorder_ready_candidates() {
        let mut engine = Engine::new();
        let translator = StaticTableTranslator::parse_rime_dict_yaml(
            r#"
---
name: sample
version: "0.1"
sort: by_weight
...

把	ba	100
吧	ba	50
八	ba	10
"#,
        )
        .expect("dictionary should parse");
        engine.add_translator(translator);
        engine.add_ranker(MockAiRanker::new(["吧"]));

        engine
            .process_key_sequence("ba")
            .expect("keys should parse");

        assert_eq!(engine.context().candidates[0].text, "吧");
        assert_eq!(engine.context().candidates[1].text, "把");
        assert_eq!(engine.context().candidates[2].text, "八");
    }

    #[test]
    fn pending_ranker_keeps_classic_candidate_order() {
        struct PendingRanker;

        impl CandidateRanker for PendingRanker {
            fn name(&self) -> &'static str {
                "pending_ranker"
            }

            fn try_rerank(
                &self,
                _context: &Context,
                _candidates: &[super::Candidate],
            ) -> RerankResult {
                RerankResult::Pending
            }
        }

        let mut engine = Engine::new();
        let translator = StaticTableTranslator::parse_rime_dict_yaml(
            r#"
---
name: sample
version: "0.1"
sort: by_weight
...

把	ba	100
吧	ba	50
"#,
        )
        .expect("dictionary should parse");
        engine.add_translator(translator);
        engine.add_ranker(PendingRanker);

        engine
            .process_key_sequence("ba")
            .expect("keys should parse");

        assert_eq!(engine.context().candidates[0].text, "把");
        assert_eq!(engine.context().candidates[1].text, "吧");
    }

    #[test]
    fn punctuation_translator_offers_half_shape_candidates_before_echo() {
        let mut engine = Engine::new();
        engine.add_translator(PunctuationTranslator::default_half_shape());

        engine.process_char('.');

        assert_eq!(engine.context().composition.input, ".");
        assert_eq!(engine.context().candidates[0].text, "。");
        assert_eq!(
            engine.context().candidates[0].source,
            CandidateSource::Punctuation
        );
        assert_eq!(engine.context().candidates[1].text, ".");
    }

    #[test]
    fn punctuation_candidate_commits_through_selection_key() {
        let mut engine = Engine::new();
        engine.add_translator(PunctuationTranslator::default_half_shape());

        let commits = engine
            .process_key_sequence(".{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, ["。"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("。"));
        assert!(!engine.status().is_composing);
    }

    #[test]
    fn punctuation_translator_tracks_full_shape_option() {
        let mut engine = Engine::new();
        engine.add_translator(PunctuationTranslator::with_shape_entries(
            [("/", "、")],
            [("/", "／")],
        ));

        engine.process_char('/');
        assert_eq!(engine.context().candidates[0].text, "、");

        engine.set_option("full_shape", true);
        assert_eq!(engine.context().candidates[0].text, "／");

        engine.set_option("full_shape", false);
        assert_eq!(engine.context().candidates[0].text, "、");
    }

    #[test]
    fn punctuation_translator_uses_symbols_as_shape_fallback() {
        let mut engine = Engine::new();
        engine.add_translator(PunctuationTranslator::with_shape_and_symbol_entries(
            [("/", "、")],
            [("/", "／")],
            [("/", "symbol-slash"), ("/fh", "©")],
        ));

        engine
            .process_key_sequence("/fh")
            .expect("keys should parse");
        assert_eq!(engine.context().candidates[0].text, "©");
        assert_eq!(engine.context().candidates[1].text, "/fh");

        engine.clear_composition();
        engine.process_char('/');
        assert_eq!(engine.context().candidates[0].text, "、");
        assert_eq!(engine.context().candidates[1].text, "/");

        engine.set_option("full_shape", true);
        assert_eq!(engine.context().candidates[0].text, "／");
        assert_eq!(engine.context().candidates[1].text, "/");
    }

    #[test]
    fn punctuation_translator_uses_librime_shape_comments() {
        let mut engine = Engine::new();
        engine.add_translator(PunctuationTranslator::with_shape_and_symbol_entries(
            [("/", "/"), (",", "、")],
            [("/", "／")],
            [("/copyright", "©")],
        ));

        engine.process_char('/');
        assert_eq!(engine.context().candidates[0].comment, "〔半角〕");

        engine.clear_composition();
        engine.process_char(',');
        assert_eq!(engine.context().candidates[0].comment, "〔全角〕");

        engine.set_option("full_shape", true);
        engine.clear_composition();
        engine.process_char('/');
        assert_eq!(engine.context().candidates[0].comment, "〔全角〕");

        engine.clear_composition();
        engine
            .process_key_sequence("/copyright")
            .expect("keys should parse");
        assert_eq!(engine.context().candidates[0].comment, "");
    }

    #[test]
    fn punctuation_translator_keeps_digit_separator_literal_for_punct_number() {
        let mut engine = Engine::new();
        engine.add_translator(
            PunctuationTranslator::with_shape_entries([(".", "。")], [(".", "。")])
                .with_required_tags(["punct", "punct_number"]),
        );
        engine.set_segment_tags(["punct_number"]);

        engine.process_char('.');
        assert_eq!(engine.context().candidates[0].text, ".");
        assert_eq!(engine.context().candidates[0].comment, "〔半角〕");

        engine.set_option("full_shape", true);
        assert_eq!(engine.context().candidates[0].text, "．");
        assert_eq!(engine.context().candidates[0].comment, "〔全角〕");
    }

    #[test]
    fn backspace_rebuilds_candidates() {
        let mut engine = Engine::new();

        engine.process_char('a');
        engine.process_char('b');
        engine.process_char('\u{8}');

        assert_eq!(engine.context().composition.input, "a");
        assert_eq!(engine.context().candidates[0].source, CandidateSource::Echo);
    }

    #[test]
    fn sequence_collects_commits_and_snapshot_status() {
        let mut engine = Engine::new();
        engine.set_schema("sample", "Sample");
        engine.add_translator(StaticTableTranslator::new([("ni", "你")]));

        let commits = engine.process_sequence("ni ");
        let snapshot = engine.snapshot();

        assert_eq!(commits, ["你"]);
        assert_eq!(snapshot.context.last_commit.as_deref(), Some("你"));
        assert_eq!(snapshot.status.schema_id, "sample");
        assert!(!snapshot.status.is_composing);
    }

    #[test]
    fn key_sequence_processes_named_backspace_and_space() {
        let mut engine = Engine::new();

        let commits = engine
            .process_key_sequence("ni{BackSpace}{space}")
            .expect("key sequence should parse");

        assert_eq!(commits, ["n"]);
        assert_eq!(engine.context().last_commit.as_deref(), Some("n"));
        assert!(!engine.status().is_composing);
    }
}
