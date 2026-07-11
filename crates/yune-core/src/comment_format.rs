use regex::Regex;

#[derive(Clone, Default)]
pub(crate) struct CommentFormat {
    formulas: Vec<CommentFormatFormula>,
}

impl CommentFormat {
    pub(crate) fn parse(formulas: &[String]) -> Self {
        let mut parsed = Vec::new();
        for formula in formulas {
            let Some(parsed_formula) = CommentFormatFormula::parse(formula) else {
                return Self::default();
            };
            parsed.push(parsed_formula);
        }
        Self { formulas: parsed }
    }

    pub(crate) fn apply(&self, value: &str) -> String {
        self.apply_with_modified(value).0
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.formulas.is_empty()
    }

    pub(crate) fn apply_with_modified(&self, value: &str) -> (String, bool) {
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
    pub(crate) fn parse(definition: &str) -> Option<Self> {
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
            replacement: decode_rime_replacement_escapes(args[2]),
        })
    }

    fn parse_erase(args: &[&str]) -> Option<Self> {
        if args.len() < 2 || args[1].is_empty() {
            return None;
        }
        Some(Self::Erase(Regex::new(args[1]).ok()?))
    }

    pub(crate) fn apply(&self, value: &mut String) {
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

fn decode_rime_replacement_escapes(value: &str) -> String {
    let mut decoded = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            decoded.push(ch);
            continue;
        }
        match chars.next() {
            Some('f') => decoded.push('\u{000c}'),
            Some('r') => decoded.push('\r'),
            Some('n') => decoded.push('\n'),
            Some('t') => decoded.push('\t'),
            Some('v') => decoded.push('\u{000b}'),
            Some('\\') => decoded.push('\\'),
            Some(other) => {
                decoded.push('\\');
                decoded.push(other);
            }
            None => decoded.push('\\'),
        }
    }
    decoded
}
