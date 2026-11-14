//! `slate predict`: text in, prediction out.

use clap::Args;
use slate_engine::{Engine, EngineError};

use crate::output;
use crate::{EXIT_ENGINE, EXIT_USER};

/// Arguments for `slate predict`.
#[derive(Debug, Args)]
pub struct PredictArgs {
    /// One or more descriptions or transcripts.
    #[arg(required = true)]
    pub texts: Vec<String>,

    /// Emit JSON instead of a table. Always a list, even for one input.
    #[arg(long)]
    pub json: bool,

    /// Number of alternatives to request.
    #[arg(long, value_name = "N")]
    pub top_k: Option<u32>,
}

/// Run `slate predict` and return the process exit code.
pub fn run(args: &PredictArgs) -> u8 {
    match predict(args) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("slate: {error}");
            exit_code_for(&error)
        }
    }
}

fn predict(args: &PredictArgs) -> Result<u8, EngineError> {
    let engine = Engine::connect_or_spawn()?;
    let predictions = engine.predict(&args.texts, args.top_k)?;

    if args.json {
        println!("{}", output::predictions_json(&predictions));
    } else {
        print!("{}", output::predictions_table(&predictions));
    }

    let _ = engine.shutdown_if_owned();
    Ok(0)
}

/// Map an engine error to the documented exit code.
fn exit_code_for(error: &EngineError) -> u8 {
    match error {
        EngineError::NotInstalled(_)
        | EngineError::StartupTimeout(_)
        | EngineError::WarmTimeout(_)
        | EngineError::Unhealthy(_)
        | EngineError::Transport(_) => EXIT_ENGINE,
        _ => EXIT_USER,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_failures_map_to_exit_two() {
        assert_eq!(
            exit_code_for(&EngineError::NotInstalled("x".into())),
            EXIT_ENGINE
        );
        assert_eq!(exit_code_for(&EngineError::StartupTimeout(30)), EXIT_ENGINE);
        assert_eq!(exit_code_for(&EngineError::WarmTimeout(60)), EXIT_ENGINE);
        assert_eq!(
            exit_code_for(&EngineError::Transport("x".into())),
            EXIT_ENGINE
        );
    }

    #[test]
    fn user_errors_map_to_exit_one() {
        assert_eq!(exit_code_for(&EngineError::Unauthorised), EXIT_USER);
        assert_eq!(
            exit_code_for(&EngineError::RequestTooLarge("x".into())),
            EXIT_USER
        );
        assert_eq!(
            exit_code_for(&EngineError::SchemaMismatch {
                found: 2,
                expected: 1
            }),
            EXIT_USER
        );
    }
}
