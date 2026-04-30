use std::fmt;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct UserDbValue {
    pub(crate) commits: i32,
    pub(crate) dee: f64,
    pub(crate) tick: u64,
}

impl Default for UserDbValue {
    fn default() -> Self {
        Self {
            commits: 0,
            dee: 0.0,
            tick: 0,
        }
    }
}

impl UserDbValue {
    pub(crate) fn new(commits: i32, dee: f64, tick: u64) -> Self {
        Self { commits, dee, tick }
    }

    pub(crate) fn pack(&self) -> String {
        format!(
            "c={} d={} t={}",
            self.commits,
            format_dee(self.dee),
            self.tick
        )
    }

    pub(crate) fn parse(value: &str) -> Result<Self, UserDbRecordError> {
        let mut parsed = Self::default();
        let mut saw_packed_field = false;
        for field in value.split_whitespace() {
            let Some((key, value)) = field.split_once('=') else {
                continue;
            };
            saw_packed_field = true;
            match key {
                "c" => {
                    parsed.commits = value.parse().map_err(|_| UserDbRecordError::InvalidValue)?;
                }
                "d" => {
                    parsed.dee = value
                        .parse::<f64>()
                        .map_err(|_| UserDbRecordError::InvalidValue)?
                        .min(10_000.0);
                }
                "t" => {
                    parsed.tick = value.parse().map_err(|_| UserDbRecordError::InvalidValue)?;
                }
                _ => {}
            }
        }
        if !saw_packed_field && !value.trim().is_empty() {
            let commits = value
                .trim()
                .parse::<i32>()
                .map_err(|_| UserDbRecordError::InvalidValue)?;
            parsed.commits = commits;
            parsed.dee = f64::from(commits.unsigned_abs());
            parsed.tick = 1;
        }
        Ok(parsed)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct UserDbRecord {
    pub(crate) key: String,
    pub(crate) code: String,
    pub(crate) phrase: String,
    pub(crate) value: UserDbValue,
}

impl UserDbRecord {
    pub(crate) fn from_code_phrase(code: &str, phrase: &str, value: UserDbValue) -> Option<Self> {
        if code.is_empty() || phrase.is_empty() {
            return None;
        }
        let normalized_code = normalize_code(code);
        let key = format!("{normalized_code}\t{phrase}");
        Some(Self {
            key,
            code: normalized_code,
            phrase: phrase.to_owned(),
            value,
        })
    }

    pub(crate) fn from_key_value(key: &str, value: UserDbValue) -> Result<Self, UserDbRecordError> {
        if let Some((code, phrase)) = key.split_once('\t') {
            if code.is_empty() || phrase.is_empty() {
                return Err(UserDbRecordError::InvalidKey);
            }
            let normalized_code = normalize_code(code);
            return Ok(Self {
                key: format!("{normalized_code}\t{phrase}"),
                code: normalized_code,
                phrase: phrase.to_owned(),
                value,
            });
        }
        let columns = key.split_whitespace().collect::<Vec<_>>();
        if columns.len() < 2 {
            return Err(UserDbRecordError::InvalidKey);
        }
        let commits = columns
            .last()
            .and_then(|value| value.parse::<i32>().ok())
            .unwrap_or(value.commits);
        let phrase = columns[columns.len().saturating_sub(2)].to_owned();
        let code = columns[..columns.len().saturating_sub(2)].join(" ");
        let mut parsed_value = value;
        if parsed_value.commits == 0 {
            parsed_value.commits = commits;
            parsed_value.dee = f64::from(commits.unsigned_abs());
            parsed_value.tick = parsed_value.tick.max(1);
        }
        Self::from_code_phrase(&code, &phrase, parsed_value).ok_or(UserDbRecordError::InvalidKey)
    }

    pub(crate) fn from_table_row(row: &str) -> Result<Self, UserDbRecordError> {
        let columns = row.split('\t').collect::<Vec<_>>();
        if columns.len() < 2 || columns[0].is_empty() || columns[1].is_empty() {
            return Err(UserDbRecordError::InvalidTableRow);
        }
        let commits: i32 = columns
            .get(2)
            .filter(|value| !value.is_empty())
            .map(|value| value.parse().map_err(|_| UserDbRecordError::InvalidValue))
            .transpose()?
            .unwrap_or(0);
        let dee = f64::from(commits.unsigned_abs());
        Self::from_code_phrase(columns[1], columns[0], UserDbValue::new(commits, dee, 1))
            .ok_or(UserDbRecordError::InvalidTableRow)
    }

    pub(crate) fn to_table_row(&self) -> Option<String> {
        if self.value.commits < 0 {
            return None;
        }
        Some(format!(
            "{}\t{}\t{}\n",
            self.phrase,
            self.code.trim_end(),
            self.value.commits
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct UserDbMetadata {
    pub(crate) db_name: String,
    pub(crate) db_type: String,
    pub(crate) tick: u64,
    pub(crate) user_id: String,
    pub(crate) rime_version: String,
}

impl UserDbMetadata {
    pub(crate) fn new(db_name: String, user_id: String) -> Self {
        Self {
            db_name,
            db_type: "userdb".to_owned(),
            tick: 1,
            user_id,
            rime_version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum UserDbRecordError {
    InvalidKey,
    InvalidTableRow,
    InvalidValue,
}

impl fmt::Display for UserDbRecordError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidKey => formatter.write_str("invalid userdb key"),
            Self::InvalidTableRow => formatter.write_str("invalid userdb table row"),
            Self::InvalidValue => formatter.write_str("invalid userdb value"),
        }
    }
}

impl std::error::Error for UserDbRecordError {}

pub(crate) fn formula_d(d: f64, t: f64, da: f64, ta: f64) -> f64 {
    d + da * ((ta - t) / 200.0).exp()
}

fn normalize_code(code: &str) -> String {
    let mut normalized = code.trim_end().to_owned();
    normalized.push(' ');
    normalized
}

fn format_dee(dee: f64) -> String {
    if dee.fract().abs() < f64::EPSILON {
        format!("{dee:.0}")
    } else {
        dee.to_string()
    }
}
