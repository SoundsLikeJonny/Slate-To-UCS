//! Build, version, changelog and verification tasks.
//!
//! One entry point per task, invoked through `just` or directly as
//! `cargo xtask <task>`. Everything here is deterministic and safe to run in CI.

#![deny(missing_docs)]
// A binary crate has no public API, so `unreachable_pub` cannot mean anything here.
#![allow(unreachable_pub)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod bump;
mod changelog;
mod docs_check;
mod gen_golden;
mod sync_taxonomy;
mod verify_versions;

use std::process::ExitCode;

/// The repository root, derived from this crate's location.
#[must_use]
pub fn repo_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .map(std::path::Path::to_path_buf)
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let Some(task) = args.first().map(String::as_str) else {
        eprintln!("{}", usage());
        return ExitCode::from(2);
    };
    let rest = &args[1..];

    let result = match task {
        "bump" => bump::run(rest),
        "verify-versions" => verify_versions::run(rest),
        "changelog" => changelog::run(rest),
        "docs-check" => docs_check::run(rest),
        "gen-golden" => gen_golden::run(rest),
        "sync-taxonomy" => sync_taxonomy::run(rest),
        "coverage" => run_tool(
            "cargo",
            &["llvm-cov", "--workspace", "--fail-under-lines", "85"],
        ),
        "fuzz" => run_tool(
            "cargo",
            &["fuzz", "run", "parse_wav", "--", "-max_total_time=60"],
        ),
        "-h" | "--help" | "help" => {
            println!("{}", usage());
            Ok(())
        }
        other => {
            eprintln!("unknown task: {other}\n\n{}", usage());
            return ExitCode::from(2);
        }
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("xtask: {message}");
            ExitCode::FAILURE
        }
    }
}

fn usage() -> String {
    [
        "usage: cargo xtask <task>",
        "",
        "tasks:",
        "  bump <X.Y.Z>          write the version into every manifest",
        "  verify-versions       fail if the manifests disagree",
        "  changelog --check     fail if [Unreleased] is missing",
        "  changelog --release X.Y.Z",
        "                        assemble changelog.d fragments into CHANGELOG.md",
        "  docs-check            fail on broken relative links in Markdown",
        "  gen-golden [--check]  regenerate or verify the conformance corpus",
        "  sync-taxonomy         refresh the bundled UCS CSV and its pinned hash",
        "  coverage              cargo llvm-cov with the enforced floor",
        "  fuzz                  60 s fuzz smoke on the WAV parser",
    ]
    .join("\n")
}

/// Run an external tool, reporting a missing tool as an actionable message.
fn run_tool(program: &str, args: &[&str]) -> Result<(), String> {
    let status = std::process::Command::new(program)
        .args(args)
        .status()
        .map_err(|e| format!("{program} is not installed or not on PATH: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} {} failed", args.join(" ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_root_contains_the_version_file() {
        assert!(repo_root().join("version.toml").is_file());
    }

    #[test]
    fn usage_lists_every_task() {
        let text = usage();

        for task in [
            "bump",
            "verify-versions",
            "changelog",
            "docs-check",
            "gen-golden",
            "sync-taxonomy",
            "coverage",
            "fuzz",
        ] {
            assert!(text.contains(task), "usage omits {task}");
        }
    }
}
