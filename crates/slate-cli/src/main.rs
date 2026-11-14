//! The `slate` command line interface.
//!
//! Exit codes are part of the contract:
//!
//! | code | meaning |
//! |---|---|
//! | 0 | success |
//! | 1 | user error: bad arguments, unknown CatID |
//! | 2 | the engine is unavailable or unhealthy |
//! | 3 | a batch completed with some items failed |

#![deny(missing_docs)]
// A binary crate has no public API, so `unreachable_pub` cannot mean anything here.
#![allow(unreachable_pub)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod cmd;
mod output;
mod whisper;

use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// Exit code for a user error.
pub const EXIT_USER: u8 = 1;
/// Exit code for an unavailable engine.
pub const EXIT_ENGINE: u8 = 2;
/// Exit code for a partially failed batch.
pub const EXIT_PARTIAL: u8 = 3;

/// Turn the slates from your recordings into UCS filenames and metadata.
#[derive(Debug, Parser)]
#[command(name = "slate", version, about, long_about = None)]
pub struct Cli {
    /// The subcommand to run.
    #[command(subcommand)]
    pub command: Command,
}

/// The available subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Predict a CatID, filename and keywords for one or more descriptions.
    Predict(cmd::predict::PredictArgs),
    /// Transcribe an audio file to timed text with Whisper.
    Transcribe(cmd::transcribe::TranscribeArgs),
    /// Report engine, model and integration health.
    Doctor,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    ExitCode::from(dispatch(&cli))
}

/// Run the parsed command and return its exit code.
fn dispatch(cli: &Cli) -> u8 {
    match &cli.command {
        Command::Predict(args) => cmd::predict::run(args),
        Command::Transcribe(args) => cmd::transcribe::run(args),
        Command::Doctor => cmd::doctor::run(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_predict_with_a_description() {
        let cli = Cli::try_parse_from(["slate", "predict", "door creak"]).expect("parsed");

        match cli.command {
            Command::Predict(args) => assert_eq!(args.texts, vec!["door creak".to_string()]),
            other => panic!("expected predict, got {other:?}"),
        }
    }

    #[test]
    fn parses_transcribe_with_a_path() {
        let cli = Cli::try_parse_from(["slate", "transcribe", "take.wav"]).expect("parsed");

        match cli.command {
            Command::Transcribe(args) => {
                assert_eq!(args.path, std::path::PathBuf::from("take.wav"));
                assert!(!args.json);
            }
            other => panic!("expected transcribe, got {other:?}"),
        }
    }

    #[test]
    fn parses_transcribe_with_json() {
        let cli =
            Cli::try_parse_from(["slate", "transcribe", "take.wav", "--json"]).expect("parsed");

        match cli.command {
            Command::Transcribe(args) => assert!(args.json),
            other => panic!("expected transcribe, got {other:?}"),
        }
    }

    #[test]
    fn transcribe_requires_a_path() {
        let err = Cli::try_parse_from(["slate", "transcribe"]).expect_err("rejected");

        assert_eq!(err.kind(), clap::error::ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn parses_doctor_without_arguments() {
        let cli = Cli::try_parse_from(["slate", "doctor"]).expect("parsed");

        assert!(matches!(cli.command, Command::Doctor));
    }

    #[test]
    fn rejects_an_unknown_subcommand() {
        let err = Cli::try_parse_from(["slate", "frobnicate"]).expect_err("rejected");

        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidSubcommand);
    }

    #[test]
    fn rejects_a_missing_subcommand() {
        let err = Cli::try_parse_from(["slate"]).expect_err("rejected");

        assert_eq!(
            err.kind(),
            clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
    }

    #[test]
    fn exit_codes_are_distinct_and_documented() {
        assert_eq!((EXIT_USER, EXIT_ENGINE, EXIT_PARTIAL), (1, 2, 3));
    }
}
