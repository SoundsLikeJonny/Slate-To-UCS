//! `cargo xtask gen-golden`: regenerate or verify the conformance corpus.

use std::path::Path;

use crate::repo_root;

/// Run the golden-corpus task.
///
/// # Errors
///
/// Returns a message when Python is unavailable or the corpus is stale.
pub fn run(args: &[String]) -> Result<(), String> {
    let check = args.iter().any(|a| a == "--check");
    run_in(&repo_root(), check)
}

/// Invoke `tools/gen_golden.py` under `root`.
///
/// # Errors
///
/// Returns a message when the script is missing, Python cannot be started, or the
/// script reports a stale corpus.
pub fn run_in(root: &Path, check: bool) -> Result<(), String> {
    let script = root.join("tools").join("gen_golden.py");
    if !script.is_file() {
        return Err(format!("{} is missing", script.display()));
    }
    let python = std::env::var("SLATE_PYTHON").unwrap_or_else(|_| "python".to_string());
    let mut command = std::process::Command::new(&python);
    command.arg(&script).current_dir(root);
    if check {
        command.arg("--check");
    }
    let status = command
        .status()
        .map_err(|e| format!("{python} is not installed or not on PATH: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "golden corpus generation failed with {}",
            status
                .code()
                .map_or_else(|| "a signal".to_string(), |c| format!("exit code {c}"))
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_script_is_reported() {
        let dir = tempfile::tempdir().expect("temp dir");

        let err = run_in(dir.path(), true).expect_err("missing script");

        assert!(err.contains("gen_golden.py"), "got {err}");
    }

    #[test]
    fn a_refused_reference_reports_its_exit_code() {
        let dir = tempfile::tempdir().expect("temp dir");
        let tools = dir.path().join("tools");
        std::fs::create_dir(&tools).expect("tools dir");
        std::fs::write(tools.join("gen_golden.py"), "raise SystemExit(3)\n").expect("script");

        let err = run_in(dir.path(), true).expect_err("non-zero exit");

        assert!(err.contains("exit code 3"), "got {err}");
    }

    #[test]
    fn the_repository_has_the_script() {
        assert!(repo_root().join("tools/gen_golden.py").is_file());
    }
}
