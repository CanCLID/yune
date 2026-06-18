use std::{env, process::ExitCode};

mod args;
mod fixture;
mod render;
mod rime_frontend;
mod sample_core;
mod transcript;

use args::{Command, FrontendOutputMode};
use fixture::{check_fixture, check_frontend_fixture};
use sample_core::{run_sequence_with_ai_provider, DEFAULT_SEQUENCE};

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: Vec<String>) -> Result<(), String> {
    match Command::parse(&args)? {
        Command::Run {
            sequence,
            ai_provider,
        } => {
            let output = run_sequence_with_ai_provider(&sequence, ai_provider)?;
            println!("{}", output.to_json());
            Ok(())
        }
        Command::Check { fixture } => check_fixture(&fixture),
        Command::FrontendCheck {
            fixture,
            shared_data_dir,
            user_data_dir,
        } => check_frontend_fixture(&fixture, &shared_data_dir, &user_data_dir),
        Command::Frontend {
            shared_data_dir,
            user_data_dir,
            schema_id,
            sequence,
            output_mode,
        } => {
            let output = rime_frontend::run_frontend(rime_frontend::FrontendOptions {
                shared_data_dir,
                user_data_dir,
                schema_id,
                sequence,
            })?;
            match output_mode {
                FrontendOutputMode::Json => println!("{}", output.to_json()),
                FrontendOutputMode::Human => print!("{}", render::render_frontend_human(&output)),
            }
            Ok(())
        }
        Command::Help => {
            render::print_help();
            Ok(())
        }
    }
}

pub(crate) fn default_sequence() -> &'static str {
    DEFAULT_SEQUENCE
}
