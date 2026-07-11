use super::Candidate;
use regex::Regex;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Default)]
pub(crate) struct SpellingAlgebra {
    formulas: Vec<SpellingAlgebraFormula>,
}

pub(crate) struct ExpandedSpellingEntry {
    pub(crate) code: String,
    pub(crate) candidate: Candidate,
    pub(crate) abbreviation: bool,
    pub(crate) correction: bool,
}

const SPELLING_ALGEBRA_FUZZY_PENALTY: f32 = -std::f32::consts::LN_2;
const SPELLING_ALGEBRA_ABBREVIATION_PENALTY: f32 = -std::f32::consts::LN_2;
const SPELLING_ALGEBRA_CORRECTION_PENALTY: f32 = -std::f32::consts::LN_10 * 2.0;
const DEPLOYED_FUZZY_PENALTY: f64 = -std::f64::consts::LN_2;
const DEPLOYED_ABBREVIATION_PENALTY: f64 = -std::f64::consts::LN_2;
const DEPLOYED_CORRECTION_PENALTY: f64 = -4.605_170_185_988_091;

impl SpellingAlgebra {
    pub(crate) fn parse(formulas: &[String]) -> Self {
        let mut parsed = Vec::new();
        for formula in formulas {
            let Some(parsed_formula) = SpellingAlgebraFormula::parse(formula) else {
                // Pinned librime `Projection::Load` clears the complete
                // calculation list when any sibling definition is invalid.
                return Self::default();
            };
            parsed.push(parsed_formula);
        }
        Self { formulas: parsed }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.formulas.is_empty()
    }

    pub(crate) fn expand_deployed_spelling_variants(
        &self,
        code: &str,
    ) -> Vec<DeployedSpellingVariant> {
        let mut entries = vec![DeployedSpellingVariant {
            code: code.to_owned(),
            properties: DeployedSpellingProperties::default(),
        }];
        for formula in &self.formulas {
            let mut next = Vec::new();
            for entry in entries {
                let Some(transformed) = formula.deployed_transform(&entry.code) else {
                    next.push(entry);
                    continue;
                };
                if formula.keep_original() {
                    next.push(entry.clone());
                }
                if formula.add_transformed() && !transformed.is_empty() {
                    let mut properties = entry.properties;
                    properties.compose(formula.deployed_properties_delta());
                    next.push(DeployedSpellingVariant {
                        code: transformed,
                        properties,
                    });
                }
            }
            entries = merge_deployed_spelling_variants(next);
        }
        entries
    }

    pub(crate) fn expand_entries_with_normal_codes(
        &self,
        entries: Vec<(String, Candidate)>,
    ) -> (Vec<ExpandedSpellingEntry>, HashSet<String>, bool) {
        let mut variant_cache = HashMap::<String, Vec<ExpandedSpellingCode>>::new();
        let mut deployed_correction_cache = HashMap::<String, HashMap<String, bool>>::new();
        let mut expanded_entries = Vec::new();
        for (code, candidate) in entries {
            let variants = variant_cache
                .entry(code.clone())
                .or_insert_with(|| self.expand_code_variants(&code));
            let deployed_corrections = deployed_correction_cache
                .entry(code.clone())
                .or_insert_with(|| {
                    self.expand_deployed_spelling_variants(&code)
                        .into_iter()
                        .map(|variant| (variant.code, variant.properties.is_correction))
                        .collect()
                });
            expanded_entries.reserve(variants.len());
            for variant in variants {
                let mut candidate = candidate.clone();
                candidate.quality += variant.quality_penalty;
                expanded_entries.push(SpellingAlgebraEntry {
                    code: variant.code.clone(),
                    candidate,
                    normal: variant.normal,
                    abbreviation: variant.abbreviation,
                    correction: deployed_corrections
                        .get(&variant.code)
                        .copied()
                        .unwrap_or(variant.correction),
                });
            }
        }
        let mut entries = dedupe_spelling_algebra_entries(expanded_entries);
        if self
            .formulas
            .iter()
            .any(SpellingAlgebraFormula::is_abbreviation)
        {
            entries.extend(leading_syllable_abbreviations(&entries));
            entries = dedupe_spelling_algebra_entries(entries);
        }
        let normal_codes = entries
            .iter()
            .filter(|entry| entry.normal)
            .map(|entry| entry.code.clone())
            .collect::<HashSet<_>>();
        let has_single_letter_abbreviations = entries
            .iter()
            .any(|entry| entry.abbreviation && !entry.normal && entry.code.len() == 1);
        let entries = entries
            .into_iter()
            .map(|entry| ExpandedSpellingEntry {
                code: entry.code,
                candidate: entry.candidate,
                abbreviation: entry.abbreviation && !entry.normal,
                correction: entry.correction,
            })
            .collect();
        (entries, normal_codes, has_single_letter_abbreviations)
    }

    pub(crate) fn expand_code_variants(&self, code: &str) -> Vec<ExpandedSpellingCode> {
        let mut entries = vec![ExpandedSpellingCode {
            code: code.to_owned(),
            quality_penalty: 0.0,
            normal: true,
            abbreviation: false,
            correction: false,
        }];
        for formula in &self.formulas {
            let mut next = Vec::new();
            for entry in entries {
                let mut transformed = entry.code.clone();
                let applied = formula.apply(&mut transformed);
                if applied {
                    if formula.keep_original() {
                        next.push(entry.clone());
                    }
                    if formula.add_transformed() && !transformed.is_empty() {
                        next.push(ExpandedSpellingCode {
                            code: transformed,
                            quality_penalty: entry.quality_penalty + formula.quality_penalty(),
                            normal: entry.normal && formula.transformed_is_normal(),
                            abbreviation: entry.abbreviation || formula.is_abbreviation(),
                            correction: entry.correction || formula.is_correction(),
                        });
                    }
                } else {
                    next.push(entry);
                }
            }
            entries = dedupe_spelling_algebra_codes(next);
        }
        entries
    }
}

#[derive(Clone)]
pub(crate) struct ExpandedSpellingCode {
    pub(crate) code: String,
    pub(crate) quality_penalty: f32,
    pub(crate) normal: bool,
    pub(crate) abbreviation: bool,
    pub(crate) correction: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
#[repr(i32)]
pub(crate) enum DeployedSpellingType {
    #[default]
    Normal,
    Fuzzy,
    Abbreviation,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct DeployedSpellingProperties {
    pub(crate) spelling_type: DeployedSpellingType,
    pub(crate) is_correction: bool,
    pub(crate) credibility: f64,
}

impl DeployedSpellingProperties {
    /// Mirrors pinned librime `SpellingProperties::Compose`: algebra deltas
    /// accumulate on the spelling provenance inherited from earlier rounds.
    fn compose(&mut self, delta: Self) {
        self.spelling_type = self.spelling_type.max(delta.spelling_type);
        self.credibility += delta.credibility;
        self.is_correction |= delta.is_correction;
    }

    /// Mirrors pinned librime `SpellingProperties::Update`: two paths to the
    /// same surface and underlying syllable collapse to the best descriptor.
    fn update(&mut self, other: Self) {
        if self.spelling_type == other.spelling_type {
            self.is_correction &= other.is_correction;
        } else if other.spelling_type < self.spelling_type {
            self.spelling_type = other.spelling_type;
            self.is_correction = other.is_correction;
        }
        if other.credibility > self.credibility {
            self.credibility = other.credibility;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DeployedSpellingVariant {
    pub(crate) code: String,
    pub(crate) properties: DeployedSpellingProperties,
}

#[derive(Clone)]
struct SpellingAlgebraEntry {
    code: String,
    candidate: Candidate,
    normal: bool,
    abbreviation: bool,
    correction: bool,
}

#[derive(Clone)]
enum SpellingAlgebraFormula {
    Transliterate(Vec<(char, char)>),
    Transform {
        pattern: Regex,
        replacement: String,
        syllable_scoped: bool,
        keep_original: bool,
        add_transformed: bool,
        quality_penalty: f32,
        abbreviation: bool,
        correction: bool,
    },
    LookAheadTransform {
        prefix: Regex,
        lookahead: Regex,
        positive: bool,
        replacement: String,
        syllable_scoped: bool,
        keep_original: bool,
        add_transformed: bool,
        quality_penalty: f32,
        abbreviation: bool,
        correction: bool,
    },
    Erase(Regex),
}

impl SpellingAlgebraFormula {
    pub(crate) fn parse(definition: &str) -> Option<Self> {
        let separator = definition.chars().find(|ch| !ch.is_ascii_lowercase())?;
        let args = definition.split(separator).collect::<Vec<_>>();
        match args.first().copied()? {
            "xlit" => Self::parse_xlit(&args),
            "xform" => Self::parse_transform(&args, false, true, 0.0, false, false, false),
            "derive" => Self::parse_derivation(&args),
            "fuzz" => Self::parse_transform(
                &args,
                true,
                true,
                SPELLING_ALGEBRA_FUZZY_PENALTY,
                false,
                false,
                true,
            ),
            "abbrev" => Self::parse_transform(
                &args,
                true,
                true,
                SPELLING_ALGEBRA_ABBREVIATION_PENALTY,
                true,
                false,
                false,
            ),
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
        abbreviation: bool,
        correction: bool,
        syllable_scope_anchor: bool,
    ) -> Option<Self> {
        if args.len() < 3 || args[1].is_empty() {
            return None;
        }
        if let Some((prefix, lookahead, positive)) = parse_lookahead_pattern(args[1]) {
            return Some(Self::LookAheadTransform {
                prefix,
                lookahead,
                positive,
                replacement: normalize_regex_replacement(args[2]),
                syllable_scoped: syllable_scope_anchor && args[1].starts_with('^'),
                keep_original,
                add_transformed,
                quality_penalty,
                abbreviation,
                correction,
            });
        }
        Some(Self::Transform {
            pattern: Regex::new(args[1]).ok()?,
            replacement: normalize_regex_replacement(args[2]),
            syllable_scoped: syllable_scope_anchor && args[1].starts_with('^'),
            keep_original,
            add_transformed,
            quality_penalty,
            abbreviation,
            correction,
        })
    }

    fn parse_derivation(args: &[&str]) -> Option<Self> {
        let (quality_penalty, abbreviation, correction) = match args.get(3).copied() {
            Some("abbrev") => (SPELLING_ALGEBRA_ABBREVIATION_PENALTY, true, false),
            Some("fuzz") => (SPELLING_ALGEBRA_FUZZY_PENALTY, false, false),
            Some("correction") => (SPELLING_ALGEBRA_CORRECTION_PENALTY, false, true),
            _ => (0.0, false, false),
        };
        Self::parse_transform(
            args,
            true,
            true,
            quality_penalty,
            abbreviation,
            correction,
            true,
        )
    }

    fn parse_erase(args: &[&str]) -> Option<Self> {
        if args.len() < 2 || args[1].is_empty() {
            return None;
        }
        Some(Self::Erase(Regex::new(args[1]).ok()?))
    }

    fn keep_original(&self) -> bool {
        match self {
            Self::Transform { keep_original, .. }
            | Self::LookAheadTransform { keep_original, .. } => *keep_original,
            _ => false,
        }
    }

    fn quality_penalty(&self) -> f32 {
        match self {
            Self::Transform {
                quality_penalty, ..
            }
            | Self::LookAheadTransform {
                quality_penalty, ..
            } => *quality_penalty,
            _ => 0.0,
        }
    }

    fn is_abbreviation(&self) -> bool {
        match self {
            Self::Transform { abbreviation, .. }
            | Self::LookAheadTransform { abbreviation, .. } => *abbreviation,
            _ => false,
        }
    }

    fn is_correction(&self) -> bool {
        match self {
            Self::Transform { correction, .. } | Self::LookAheadTransform { correction, .. } => {
                *correction
            }
            _ => false,
        }
    }

    fn deployed_properties_delta(&self) -> DeployedSpellingProperties {
        match self {
            Self::Transliterate(_) | Self::Erase(_) => DeployedSpellingProperties::default(),
            Self::Transform {
                quality_penalty,
                abbreviation,
                correction,
                ..
            }
            | Self::LookAheadTransform {
                quality_penalty,
                abbreviation,
                correction,
                ..
            } => {
                let (spelling_type, credibility) = if *abbreviation {
                    (
                        DeployedSpellingType::Abbreviation,
                        DEPLOYED_ABBREVIATION_PENALTY,
                    )
                } else if *quality_penalty == SPELLING_ALGEBRA_FUZZY_PENALTY {
                    (DeployedSpellingType::Fuzzy, DEPLOYED_FUZZY_PENALTY)
                } else if *correction {
                    (DeployedSpellingType::Normal, DEPLOYED_CORRECTION_PENALTY)
                } else {
                    (DeployedSpellingType::Normal, 0.0)
                };
                DeployedSpellingProperties {
                    spelling_type,
                    is_correction: *correction,
                    credibility,
                }
            }
        }
    }

    fn deployed_transform(&self, value: &str) -> Option<String> {
        match self {
            Self::Transliterate(char_map) => {
                let transformed = value
                    .chars()
                    .map(|ch| {
                        char_map
                            .iter()
                            .find(|(source, _)| *source == ch)
                            .map_or(ch, |(_, replacement)| *replacement)
                    })
                    .collect::<String>();
                (transformed != value).then_some(transformed)
            }
            Self::Transform {
                pattern,
                replacement,
                ..
            } => {
                // Prism inputs are already individual syllabary entries. Apply
                // anchored rules to that whole spelling, matching the historical
                // prism builder; runtime dictionary-code expansion remains
                // syllable-scoped in `apply` below.
                if pattern.is_match(value) {
                    let transformed = pattern
                        .replace_all(value, replacement.as_str())
                        .into_owned();
                    (transformed != value).then_some(transformed)
                } else {
                    None
                }
            }
            Self::LookAheadTransform {
                prefix,
                lookahead,
                positive,
                replacement,
                ..
            } => replace_lookahead_matches(value, prefix, lookahead, *positive, replacement),
            Self::Erase(pattern) => {
                let should_erase = pattern
                    .find(value)
                    .is_some_and(|matched| matched.start() == 0 && matched.end() == value.len());
                should_erase.then(String::new)
            }
        }
    }

    fn add_transformed(&self) -> bool {
        !matches!(self, Self::Erase(_))
    }

    fn transformed_is_normal(&self) -> bool {
        match self {
            Self::Transliterate(_) => true,
            Self::Transform {
                quality_penalty, ..
            }
            | Self::LookAheadTransform {
                quality_penalty, ..
            } => *quality_penalty == 0.0,
            Self::Erase(_) => false,
        }
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
                syllable_scoped,
                add_transformed,
                ..
            } => {
                let transformed = if *syllable_scoped {
                    replace_by_syllable(value, |syllable| {
                        if !pattern.is_match(syllable) {
                            return None;
                        }
                        let transformed = pattern
                            .replace_all(syllable, replacement.as_str())
                            .into_owned();
                        (transformed != syllable).then_some(transformed)
                    })
                } else if pattern.is_match(value) {
                    Some(
                        pattern
                            .replace_all(value, replacement.as_str())
                            .into_owned(),
                    )
                } else {
                    None
                };
                let modified = transformed
                    .as_ref()
                    .is_some_and(|transformed| transformed != value);
                if modified && *add_transformed {
                    *value = transformed.expect("modified transform should have output");
                }
                modified
            }
            Self::LookAheadTransform {
                prefix,
                lookahead,
                positive,
                replacement,
                syllable_scoped,
                add_transformed,
                ..
            } => {
                let transformed = if *syllable_scoped {
                    replace_by_syllable(value, |syllable| {
                        replace_lookahead_matches(
                            syllable,
                            prefix,
                            lookahead,
                            *positive,
                            replacement,
                        )
                    })
                } else {
                    replace_lookahead_matches(value, prefix, lookahead, *positive, replacement)
                };
                let modified = transformed
                    .as_ref()
                    .is_some_and(|transformed| transformed != value);
                if modified && *add_transformed {
                    *value = transformed.expect("modified lookahead transform should have output");
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

fn normalize_regex_replacement(replacement: &str) -> String {
    let mut output = String::with_capacity(replacement.len());
    let mut chars = replacement.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '$' {
            output.push(ch);
            continue;
        }
        let mut digits = String::new();
        while let Some(next) = chars.peek().copied() {
            if next.is_ascii_digit() {
                digits.push(next);
                chars.next();
            } else {
                break;
            }
        }
        if digits.is_empty() {
            output.push('$');
        } else {
            output.push_str("${");
            output.push_str(&digits);
            output.push('}');
        }
    }
    output
}

fn replace_by_syllable(
    value: &str,
    mut replace: impl FnMut(&str) -> Option<String>,
) -> Option<String> {
    let mut output = String::with_capacity(value.len());
    let mut start = 0;
    let mut modified = false;

    for (index, ch) in value.char_indices() {
        if !ch.is_ascii_digit() {
            continue;
        }
        let end = index + ch.len_utf8();
        let syllable = &value[start..end];
        if let Some(transformed) = replace(syllable) {
            output.push_str(&transformed);
            modified = true;
        } else {
            output.push_str(syllable);
        }
        start = end;
    }

    if start < value.len() {
        let syllable = &value[start..];
        if let Some(transformed) = replace(syllable) {
            output.push_str(&transformed);
            modified = true;
        } else {
            output.push_str(syllable);
        }
    }

    modified.then_some(output)
}

fn leading_syllable_abbreviations(entries: &[SpellingAlgebraEntry]) -> Vec<SpellingAlgebraEntry> {
    entries
        .iter()
        .filter(|entry| !entry.abbreviation)
        .filter_map(|entry| {
            if toned_syllable_count(&entry.candidate.comment) != Some(2) {
                return None;
            }
            let first_syllable = first_toned_syllable_letters(&entry.candidate.comment)?;
            if first_syllable.len() <= 1 || !entry.code.starts_with(first_syllable) {
                return None;
            }
            let suffix = &entry.code[first_syllable.len()..];
            if suffix.is_empty() {
                return None;
            }
            let initial = first_syllable.chars().next()?;
            if initial != 'm' {
                return None;
            }
            let code = format!("{initial}{suffix}");
            if code == entry.code {
                return None;
            }
            let mut candidate = entry.candidate.clone();
            candidate.quality += SPELLING_ALGEBRA_ABBREVIATION_PENALTY;
            Some(SpellingAlgebraEntry {
                code,
                candidate,
                normal: false,
                abbreviation: true,
                correction: entry.correction,
            })
        })
        .collect()
}

fn first_toned_syllable_letters(raw_code: &str) -> Option<&str> {
    let mut start = None;
    for (index, ch) in raw_code.char_indices() {
        if ch.is_ascii_alphabetic() {
            start.get_or_insert(index);
        } else if ch.is_ascii_digit() {
            return start.map(|start| &raw_code[start..index]);
        } else if start.is_some() {
            return None;
        }
    }
    None
}

fn toned_syllable_count(raw_code: &str) -> Option<usize> {
    let count = raw_code.chars().filter(char::is_ascii_digit).count();
    (count > 0).then_some(count)
}

fn parse_lookahead_pattern(pattern: &str) -> Option<(Regex, Regex, bool)> {
    let positive_index = pattern.find("(?=");
    let negative_index = pattern.find("(?!");
    let (lookahead_start, positive) = match (positive_index, negative_index) {
        (Some(positive), Some(negative)) if positive < negative => (positive, true),
        (Some(_), Some(negative)) => (negative, false),
        (Some(positive), None) => (positive, true),
        (None, Some(negative)) => (negative, false),
        (None, None) => return None,
    };
    let prefix = &pattern[..lookahead_start];
    let lookahead_body = &pattern[lookahead_start + 3..];
    let lookahead_end = lookahead_body.find(')')?;
    if !lookahead_body[lookahead_end + 1..].is_empty() {
        return None;
    }
    let lookahead = &lookahead_body[..lookahead_end];
    Some((
        Regex::new(prefix).ok()?,
        Regex::new(&format!("^(?:{lookahead})")).ok()?,
        positive,
    ))
}

fn replace_lookahead_matches(
    value: &str,
    prefix: &Regex,
    lookahead: &Regex,
    positive: bool,
    replacement: &str,
) -> Option<String> {
    let mut output = String::with_capacity(value.len());
    let mut last_end = 0;
    let mut modified = false;

    for captures in prefix.captures_iter(value) {
        let whole_match = captures.get(0)?;
        let predicate_matches = lookahead.is_match(&value[whole_match.end()..]);
        if predicate_matches != positive {
            continue;
        }
        output.push_str(&value[last_end..whole_match.start()]);
        captures.expand(replacement, &mut output);
        last_end = whole_match.end();
        modified = true;
    }

    if modified {
        output.push_str(&value[last_end..]);
        Some(output)
    } else {
        None
    }
}

fn merge_deployed_spelling_variants(
    entries: Vec<DeployedSpellingVariant>,
) -> Vec<DeployedSpellingVariant> {
    let mut merged: Vec<DeployedSpellingVariant> = Vec::new();
    let mut indexes = HashMap::<String, usize>::new();
    for entry in entries {
        if let Some(index) = indexes.get(&entry.code).copied() {
            merged[index].properties.update(entry.properties);
        } else {
            indexes.insert(entry.code.clone(), merged.len());
            merged.push(entry);
        }
    }
    merged
}

fn dedupe_spelling_algebra_entries(
    entries: Vec<SpellingAlgebraEntry>,
) -> Vec<SpellingAlgebraEntry> {
    let mut deduped: Vec<SpellingAlgebraEntry> = Vec::new();
    let mut indexes = HashMap::<(String, String, String), usize>::new();
    for entry in entries {
        let key = (
            entry.code.clone(),
            entry.candidate.text.clone(),
            entry.candidate.comment.clone(),
        );
        if let Some(index) = indexes.get(&key).copied() {
            let existing_entry = &mut deduped[index];
            existing_entry.normal |= entry.normal;
            existing_entry.abbreviation |= entry.abbreviation;
            existing_entry.correction &= entry.correction;
            if entry.candidate.quality > existing_entry.candidate.quality {
                existing_entry.candidate = entry.candidate;
            }
        } else {
            indexes.insert(key, deduped.len());
            deduped.push(entry);
        }
    }
    deduped
}

fn dedupe_spelling_algebra_codes(entries: Vec<ExpandedSpellingCode>) -> Vec<ExpandedSpellingCode> {
    let mut deduped: Vec<ExpandedSpellingCode> = Vec::new();
    let mut indexes = HashMap::<String, usize>::new();
    for entry in entries {
        if let Some(index) = indexes.get(&entry.code).copied() {
            let existing_entry = &mut deduped[index];
            existing_entry.normal |= entry.normal;
            existing_entry.abbreviation |= entry.abbreviation;
            existing_entry.correction &= entry.correction;
            if entry.quality_penalty > existing_entry.quality_penalty {
                existing_entry.quality_penalty = entry.quality_penalty;
            }
        } else {
            indexes.insert(entry.code.clone(), deduped.len());
            deduped.push(entry);
        }
    }
    deduped
}
