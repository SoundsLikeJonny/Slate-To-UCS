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
    /// Report engine, model and integration health.
    Doctor,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let code = match cli.command {
        Command::Predict(args) => cmd::predict::run(&args),
        Command::Doctor => cmd::doctor::run(),
    };
    ExitCode::from(code)
}
