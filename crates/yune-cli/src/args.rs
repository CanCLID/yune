use std::path::PathBuf;

use crate::default_sequence;

// Owns CLI command-shape parsing for core run/check and ABI frontend replay;
// runtime path and schema flags are the librime-visible comparison inputs.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Command {
    Run {
        sequence: String,
        ai_provider: AiProviderMode,
    },
    Check {
        fixture: PathBuf,
    },
    Frontend {
        shared_data_dir: PathBuf,
        user_data_dir: PathBuf,
        schema_id: String,
        sequence: String,
        output_mode: FrontendOutputMode,
    },
    FrontendCheck {
        fixture: PathBuf,
        shared_data_dir: PathBuf,
        user_data_dir: PathBuf,
    },
    Help,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FrontendOutputMode {
    Json,
    Human,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AiProviderMode {
    None,
    Mock,
    Local,
}

impl Command {
    pub(crate) fn parse(args: &[String]) -> Result<Self, String> {
        match args.first().map(String::as_str) {
            None => Ok(Self::Run {
                sequence: default_sequence().to_owned(),
                ai_provider: AiProviderMode::None,
            }),
            Some("run") => parse_run(&args[1..]),
            Some("check") => {
                let fixture = args
                    .get(1)
                    .ok_or_else(|| "usage: yune-cli check <fixture.json>".to_owned())?;
                Ok(Self::Check {
                    fixture: PathBuf::from(fixture),
                })
            }
            Some("frontend") => parse_frontend(&args[1..]),
            Some("frontend-check") => parse_frontend_check(&args[1..]),
            Some("-h" | "--help" | "help") => Ok(Self::Help),
            Some(command) => Err(format!("unknown command: {command}\n\n{}", help_text())),
        }
    }
}

fn parse_run(args: &[String]) -> Result<Command, String> {
    let mut sequence = None;
    let mut ai_provider = AiProviderMode::None;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--ai-provider" => {
                index += 1;
                ai_provider = parse_ai_provider(flag_value(args, index, "ai-provider")?)?;
            }
            flag if flag.starts_with("--") => {
                return Err(format!(
                    "error: unknown run flag {flag}. next: run yune-cli --help."
                ));
            }
            value if sequence.is_none() => {
                sequence = Some(value.to_owned());
            }
            value => {
                return Err(format!(
                    "error: unexpected run argument {value}. next: pass at most one key sequence."
                ));
            }
        }
        index += 1;
    }

    Ok(Command::Run {
        sequence: sequence.unwrap_or_else(|| default_sequence().to_owned()),
        ai_provider,
    })
}

fn parse_frontend(args: &[String]) -> Result<Command, String> {
    let (shared_data_dir, user_data_dir, schema_id, sequence, fixture, output_mode) =
        parse_frontend_flags(args)?;
    if fixture.is_some() {
        return Err(
            "error: unexpected frontend fixture. next: pass --sequence <keys> for frontend runs."
                .to_owned(),
        );
    }

    Ok(Command::Frontend {
        shared_data_dir: required_path(shared_data_dir, "shared-data-dir")?,
        user_data_dir: required_path(user_data_dir, "user-data-dir")?,
        schema_id: validate_schema_id(schema_id.ok_or_else(|| {
            "error: missing --schema. next: pass --schema <schema-id>.".to_owned()
        })?)?,
        sequence: sequence
            .ok_or_else(|| "error: missing --sequence. next: pass --sequence <keys>.".to_owned())?,
        output_mode,
    })
}

fn parse_frontend_check(args: &[String]) -> Result<Command, String> {
    let (shared_data_dir, user_data_dir, schema_id, sequence, fixture, output_mode) =
        parse_frontend_flags(args)?;
    if schema_id.is_some() || sequence.is_some() || output_mode != FrontendOutputMode::Json {
        return Err(
            "error: unexpected frontend-check replay fields. next: read schema_id and sequence from the fixture."
                .to_owned(),
        );
    }

    Ok(Command::FrontendCheck {
        fixture: fixture.ok_or_else(|| {
            "usage: yune-cli frontend-check <fixture.json> --shared-data-dir <path> --user-data-dir <path>"
                .to_owned()
        })?,
        shared_data_dir: required_path(shared_data_dir, "shared-data-dir")?,
        user_data_dir: required_path(user_data_dir, "user-data-dir")?,
    })
}

type FrontendFlagValues = (
    Option<PathBuf>,
    Option<PathBuf>,
    Option<String>,
    Option<String>,
    Option<PathBuf>,
    FrontendOutputMode,
);

fn parse_frontend_flags(args: &[String]) -> Result<FrontendFlagValues, String> {
    let mut shared_data_dir = None;
    let mut user_data_dir = None;
    let mut schema_id = None;
    let mut sequence = None;
    let mut fixture = None;
    let mut output_mode = FrontendOutputMode::Json;
    let mut index = 0;

    while index < args.len() {
        match args[index].as_str() {
            "--shared-data-dir" => {
                index += 1;
                shared_data_dir = Some(PathBuf::from(flag_value(args, index, "shared-data-dir")?));
            }
            "--user-data-dir" => {
                index += 1;
                user_data_dir = Some(PathBuf::from(flag_value(args, index, "user-data-dir")?));
            }
            "--schema" => {
                index += 1;
                schema_id = Some(flag_value(args, index, "schema")?.to_owned());
            }
            "--sequence" => {
                index += 1;
                sequence = Some(flag_value(args, index, "sequence")?.to_owned());
            }
            "--output" => {
                index += 1;
                output_mode = parse_output_mode(flag_value(args, index, "output")?)?;
            }
            flag if flag.starts_with("--") => {
                return Err(format!(
                    "error: unknown frontend flag {flag}. next: run yune-cli --help."
                ));
            }
            value if fixture.is_none() => {
                fixture = Some(PathBuf::from(value));
            }
            value => {
                return Err(format!(
                    "error: unexpected frontend argument {value}. next: pass supported frontend flags."
                ));
            }
        }
        index += 1;
    }

    Ok((
        shared_data_dir,
        user_data_dir,
        schema_id,
        sequence,
        fixture,
        output_mode,
    ))
}

fn parse_output_mode(value: &str) -> Result<FrontendOutputMode, String> {
    match value {
        "json" => Ok(FrontendOutputMode::Json),
        "human" => Ok(FrontendOutputMode::Human),
        _ => Err(
            "error: unknown frontend output mode. next: pass --output json or --output human."
                .to_owned(),
        ),
    }
}

fn parse_ai_provider(value: &str) -> Result<AiProviderMode, String> {
    match value {
        "none" => Ok(AiProviderMode::None),
        "mock" => Ok(AiProviderMode::Mock),
        "local" | "local-model" => Ok(AiProviderMode::Local),
        _ => Err(
            "error: unknown AI provider. next: pass --ai-provider none, mock, or local.".to_owned(),
        ),
    }
}

pub(crate) fn validate_schema_id(schema_id: String) -> Result<String, String> {
    let valid = !schema_id.is_empty()
        && schema_id != "."
        && schema_id != ".."
        && schema_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-');
    if valid {
        Ok(schema_id)
    } else {
        Err(
            "error: invalid --schema. next: pass a logical schema id such as luna_pinyin."
                .to_owned(),
        )
    }
}

fn required_path(value: Option<PathBuf>, name: &str) -> Result<PathBuf, String> {
    value.ok_or_else(|| format!("error: missing --{name}. next: pass --{name} <path>."))
}

fn flag_value<'args>(
    args: &'args [String],
    index: usize,
    name: &str,
) -> Result<&'args str, String> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| format!("error: missing --{name}. next: pass --{name} <path>."))
}

pub(crate) fn help_text() -> &'static str {
    "usage:\n  yune-cli run [--ai-provider none|mock|local] [key-sequence]\n  yune-cli check <fixture.json>\n  yune-cli frontend --shared-data-dir <path> --user-data-dir <path> --schema <schema-id> --sequence <keys> [--output json|human]\n  yune-cli frontend-check <fixture.json> --shared-data-dir <path> --user-data-dir <path>"
}

#[cfg(test)]
mod tests {
    use super::{AiProviderMode, Command, FrontendOutputMode};

    #[test]
    fn default_command_runs_default_sequence() {
        assert_eq!(
            Command::parse(&[]),
            Ok(Command::Run {
                sequence: "nihao ".to_owned(),
                ai_provider: AiProviderMode::None
            })
        );
    }

    #[test]
    fn parses_run_command_with_mock_ai_provider() {
        assert_eq!(
            Command::parse(&[
                "run".to_owned(),
                "--ai-provider".to_owned(),
                "mock".to_owned(),
                "nihao".to_owned()
            ]),
            Ok(Command::Run {
                sequence: "nihao".to_owned(),
                ai_provider: AiProviderMode::Mock
            })
        );
    }

    #[test]
    fn parses_run_command_with_local_ai_provider() {
        assert_eq!(
            Command::parse(&[
                "run".to_owned(),
                "--ai-provider".to_owned(),
                "local".to_owned(),
                "nihao".to_owned()
            ]),
            Ok(Command::Run {
                sequence: "nihao".to_owned(),
                ai_provider: AiProviderMode::Local
            })
        );
    }

    #[test]
    fn rejects_unknown_run_ai_provider() {
        assert_eq!(
            Command::parse(&[
                "run".to_owned(),
                "--ai-provider".to_owned(),
                "remote".to_owned()
            ]),
            Err(
                "error: unknown AI provider. next: pass --ai-provider none, mock, or local."
                    .to_owned()
            )
        );
    }

    #[test]
    fn parses_check_command() {
        assert_eq!(
            Command::parse(&["check".to_owned(), "fixture.json".to_owned()]),
            Ok(Command::Check {
                fixture: "fixture.json".into()
            })
        );
    }

    #[test]
    fn parses_frontend_command_with_explicit_runtime_inputs() {
        assert_eq!(
            Command::parse(&[
                "frontend".to_owned(),
                "--shared-data-dir".to_owned(),
                "shared".to_owned(),
                "--user-data-dir".to_owned(),
                "user".to_owned(),
                "--schema".to_owned(),
                "luna_pinyin".to_owned(),
                "--sequence".to_owned(),
                "nihao ".to_owned(),
            ]),
            Ok(Command::Frontend {
                shared_data_dir: "shared".into(),
                user_data_dir: "user".into(),
                schema_id: "luna_pinyin".to_owned(),
                sequence: "nihao ".to_owned(),
                output_mode: FrontendOutputMode::Json,
            })
        );
    }

    #[test]
    fn parses_frontend_command_with_human_output_mode() {
        assert_eq!(
            Command::parse(&[
                "frontend".to_owned(),
                "--shared-data-dir".to_owned(),
                "shared".to_owned(),
                "--user-data-dir".to_owned(),
                "user".to_owned(),
                "--schema".to_owned(),
                "luna_pinyin".to_owned(),
                "--sequence".to_owned(),
                "nihao ".to_owned(),
                "--output".to_owned(),
                "human".to_owned(),
            ]),
            Ok(Command::Frontend {
                shared_data_dir: "shared".into(),
                user_data_dir: "user".into(),
                schema_id: "luna_pinyin".to_owned(),
                sequence: "nihao ".to_owned(),
                output_mode: FrontendOutputMode::Human,
            })
        );
    }

    #[test]
    fn frontend_requires_explicit_shared_data_dir() {
        assert_eq!(
            Command::parse(&[
                "frontend".to_owned(),
                "--user-data-dir".to_owned(),
                "user".to_owned(),
                "--schema".to_owned(),
                "luna_pinyin".to_owned(),
                "--sequence".to_owned(),
                "nihao ".to_owned(),
            ]),
            Err(
                "error: missing --shared-data-dir. next: pass --shared-data-dir <path>.".to_owned()
            )
        );
    }

    #[test]
    fn frontend_requires_explicit_user_data_dir() {
        assert_eq!(
            Command::parse(&[
                "frontend".to_owned(),
                "--shared-data-dir".to_owned(),
                "shared".to_owned(),
                "--schema".to_owned(),
                "luna_pinyin".to_owned(),
                "--sequence".to_owned(),
                "nihao ".to_owned(),
            ]),
            Err("error: missing --user-data-dir. next: pass --user-data-dir <path>.".to_owned())
        );
    }

    #[test]
    fn frontend_requires_schema_and_sequence_without_changing_run_check() {
        assert_eq!(
            Command::parse(&[
                "frontend".to_owned(),
                "--shared-data-dir".to_owned(),
                "shared".to_owned(),
                "--user-data-dir".to_owned(),
                "user".to_owned(),
                "--sequence".to_owned(),
                "nihao ".to_owned(),
            ]),
            Err("error: missing --schema. next: pass --schema <schema-id>.".to_owned())
        );
        assert_eq!(
            Command::parse(&[
                "frontend".to_owned(),
                "--shared-data-dir".to_owned(),
                "shared".to_owned(),
                "--user-data-dir".to_owned(),
                "user".to_owned(),
                "--schema".to_owned(),
                "luna_pinyin".to_owned(),
            ]),
            Err("error: missing --sequence. next: pass --sequence <keys>.".to_owned())
        );
        assert_eq!(
            Command::parse(&["run".to_owned(), "hao".to_owned()]),
            Ok(Command::Run {
                sequence: "hao".to_owned(),
                ai_provider: AiProviderMode::None
            })
        );
    }

    #[test]
    fn rejects_invalid_frontend_schema_id() {
        assert_eq!(
            Command::parse(&[
                "frontend".to_owned(),
                "--shared-data-dir".to_owned(),
                "shared".to_owned(),
                "--user-data-dir".to_owned(),
                "user".to_owned(),
                "--schema".to_owned(),
                "../default".to_owned(),
                "--sequence".to_owned(),
                "nihao ".to_owned(),
            ]),
            Err(
                "error: invalid --schema. next: pass a logical schema id such as luna_pinyin."
                    .to_owned()
            )
        );
    }

    #[test]
    fn parses_frontend_check_command_with_explicit_runtime_inputs() {
        assert_eq!(
            Command::parse(&[
                "frontend-check".to_owned(),
                "fixture.json".to_owned(),
                "--shared-data-dir".to_owned(),
                "shared".to_owned(),
                "--user-data-dir".to_owned(),
                "user".to_owned(),
            ]),
            Ok(Command::FrontendCheck {
                fixture: "fixture.json".into(),
                shared_data_dir: "shared".into(),
                user_data_dir: "user".into(),
            })
        );
    }

    #[test]
    fn help_lists_frontend_command() {
        assert!(super::help_text().starts_with("usage:"));
        assert!(super::help_text().contains(
            "yune-cli frontend --shared-data-dir <path> --user-data-dir <path> --schema <schema-id> --sequence <keys>"
        ));
        assert!(super::help_text().contains(
            "yune-cli frontend-check <fixture.json> --shared-data-dir <path> --user-data-dir <path>"
        ));
    }
}
