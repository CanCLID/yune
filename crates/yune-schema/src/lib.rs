use serde::Deserialize;
use std::{error::Error, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Schema {
    pub id: String,
    pub name: String,
    pub engine: EngineSpec,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct EngineSpec {
    pub processors: Vec<String>,
    pub segmentors: Vec<String>,
    pub translators: Vec<String>,
    pub filters: Vec<String>,
}

impl Schema {
    #[must_use]
    pub fn minimal(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            engine: EngineSpec {
                processors: vec!["speller".to_owned(), "selector".to_owned()],
                segmentors: vec!["abc_segmentor".to_owned(), "fallback_segmentor".to_owned()],
                translators: vec!["echo_translator".to_owned()],
                filters: Vec::new(),
            },
        }
    }

    pub fn parse_rime_yaml(input: &str) -> Result<Self, SchemaParseError> {
        let document: RimeSchemaDocument = serde_yaml::from_str(input)?;
        Ok(Self {
            id: required_field(document.schema.schema_id, "schema.schema_id")?,
            name: required_field(document.schema.name, "schema.name")?,
            engine: document.engine.into(),
        })
    }
}

#[derive(Debug)]
pub struct SchemaParseError {
    message: String,
}

impl SchemaParseError {
    fn missing_field(path: &'static str) -> Self {
        Self {
            message: format!("missing required RIME schema field: {path}"),
        }
    }
}

impl fmt::Display for SchemaParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for SchemaParseError {}

impl From<serde_yaml::Error> for SchemaParseError {
    fn from(error: serde_yaml::Error) -> Self {
        Self {
            message: format!("invalid RIME schema YAML: {error}"),
        }
    }
}

#[derive(Debug, Default, Deserialize)]
struct RimeSchemaDocument {
    #[serde(default)]
    schema: RimeSchemaMetadata,
    #[serde(default)]
    engine: RimeEngineSpec,
}

#[derive(Debug, Default, Deserialize)]
struct RimeSchemaMetadata {
    schema_id: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct RimeEngineSpec {
    #[serde(default)]
    processors: Vec<String>,
    #[serde(default)]
    segmentors: Vec<String>,
    #[serde(default)]
    translators: Vec<String>,
    #[serde(default)]
    filters: Vec<String>,
}

impl From<RimeEngineSpec> for EngineSpec {
    fn from(engine: RimeEngineSpec) -> Self {
        Self {
            processors: engine.processors,
            segmentors: engine.segmentors,
            translators: engine.translators,
            filters: engine.filters,
        }
    }
}

fn required_field(value: Option<String>, path: &'static str) -> Result<String, SchemaParseError> {
    value.ok_or_else(|| SchemaParseError::missing_field(path))
}

#[cfg(test)]
mod tests {
    use super::Schema;

    #[test]
    fn creates_minimal_schema() {
        let schema = Schema::minimal("sample", "Sample");

        assert_eq!(schema.id, "sample");
        assert_eq!(schema.engine.translators, ["echo_translator"]);
    }

    #[test]
    fn parses_rime_schema_subset() {
        let schema = Schema::parse_rime_yaml(
            r#"
schema:
  schema_id: sample
  name: Sample
  version: "0.1.sample"
engine:
  processors:
    - speller
    - punctuator
    - selector
    - navigator
    - express_editor
  segmentors:
    - abc_segmentor
    - punct_segmentor
    - fallback_segmentor
  translators:
    - punct_translator
    - trivial_translator
    - echo_translator
"#,
        )
        .expect("schema should parse");

        assert_eq!(schema.id, "sample");
        assert_eq!(schema.name, "Sample");
        assert_eq!(
            schema.engine.processors,
            [
                "speller",
                "punctuator",
                "selector",
                "navigator",
                "express_editor"
            ]
        );
        assert_eq!(
            schema.engine.segmentors,
            ["abc_segmentor", "punct_segmentor", "fallback_segmentor"]
        );
        assert_eq!(
            schema.engine.translators,
            ["punct_translator", "trivial_translator", "echo_translator"]
        );
        assert!(schema.engine.filters.is_empty());
    }

    #[test]
    fn reports_missing_required_schema_metadata() {
        let error = Schema::parse_rime_yaml(
            r#"
schema:
  name: Sample
engine:
  translators:
    - echo_translator
"#,
        )
        .expect_err("schema_id should be required");

        assert_eq!(
            error.to_string(),
            "missing required RIME schema field: schema.schema_id"
        );
    }
}
