//! `slate tag`: plan, apply and undo renames of slate files.
//!
//! A dry run is the default. `--apply` writes a journal and renames; `--undo` replays a
//! journal. The desired names come from a plan file until the engine wiring lands, so the
//! command is usable and testable without a running server.

use std::path::{Path, PathBuf};

use clap::Args;
use serde::{Deserialize, Serialize};
use slate_tagger::{apply, plan_renames, undo, Plan, PlanOptions, RenameRequest, SkipReason};

use crate::EXIT_USER;

/// Arguments for `slate tag`.
#[derive(Debug, Args)]
pub struct TagArgs {
    /// The directory whose files are renamed.
    pub directory: PathBuf,

    /// A JSONL plan file: one `{"file": "...", "name": "..."}` object per line.
    #[arg(long, value_name = "FILE")]
    pub names: Option<PathBuf>,

    /// Rename for real. Without this the command only prints the plan.
    #[arg(long)]
    pub apply: bool,

    /// Undo a journal written by a previous `--apply`.
    #[arg(long, value_name = "JOURNAL")]
    pub undo: Option<PathBuf>,

    /// Print JSON instead of a table.
    #[arg(long)]
    pub json: bool,
}

/// One line of a plan file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NameRequest {
    /// The file name as it is now, relative to the directory.
    pub file: String,
    /// The file name it should get.
    pub name: String,
}

/// Run `slate tag` and return the process exit code.
pub fn run(args: &TagArgs) -> u8 {
    if let Some(journal) = &args.undo {
        return run_undo(journal, args.json);
    }
    let Some(names) = &args.names else {
        eprintln!("slate tag: --names is required unless --undo is given");
        return EXIT_USER;
    };
    let requests = match read_requests(names) {
        Ok(requests) => requests,
        Err(message) => {
            eprintln!("slate tag: {message}");
            return EXIT_USER;
        }
    };
    let plan = plan_renames(&args.directory, &requests, &PlanOptions::default());
    if args.apply {
        run_apply(&plan, args.json)
    } else {
        print_plan(&plan, args.json);
        if plan.is_actionable() {
            0
        } else {
            EXIT_USER
        }
    }
}

/// Read a plan file into requests.
///
/// # Errors
///
/// Returns a message naming the line that is not a `{"file", "name"}` object.
pub fn read_requests(path: &Path) -> Result<Vec<RenameRequest>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    parse_requests(&text).map_err(|e| format!("{}: {e}", path.display()))
}

/// Parse plan-file text into requests, skipping blank lines.
///
/// # Errors
///
/// Returns a message naming the first line that is not a valid object.
pub fn parse_requests(text: &str) -> Result<Vec<RenameRequest>, String> {
    let mut out = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let request: NameRequest =
            serde_json::from_str(line).map_err(|e| format!("line {}: {e}", index + 1))?;
        out.push(RenameRequest::new(&request.file, request.name));
    }
    Ok(out)
}

/// Apply a plan and report the journal path.
fn run_apply(plan: &Plan, json: bool) -> u8 {
    let created = timestamp();
    match apply(plan, &created) {
        Ok(outcome) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "renamed": outcome.renamed,
                        "journal": outcome.journal_path.display().to_string(),
                    })
                );
            } else {
                println!("renamed {} files", outcome.renamed);
                println!("journal {}", outcome.journal_path.display());
            }
            0
        }
        Err(error) => {
            eprintln!("slate tag: {error}");
            EXIT_USER
        }
    }
}

/// Undo a journal and report the counts.
fn run_undo(journal: &Path, json: bool) -> u8 {
    match undo(journal) {
        Ok(outcome) => {
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "restored": outcome.restored,
                        "already_restored": outcome.already_restored,
                    })
                );
            } else {
                println!(
                    "restored {} files, {} already restored",
                    outcome.restored, outcome.already_restored
                );
            }
            0
        }
        Err(error) => {
            eprintln!("slate tag: {error}");
            EXIT_USER
        }
    }
}

/// Print a plan as a table or as JSON.
fn print_plan(plan: &Plan, json: bool) {
    if json {
        let entries: Vec<serde_json::Value> = plan
            .entries
            .iter()
            .map(|entry| {
                serde_json::json!({
                    "from": entry.from.display().to_string(),
                    "to": entry.to.display().to_string(),
                    "skip": entry.skip.as_ref().map(SkipReason::kind),
                    "reason": entry.skip.as_ref().map(SkipReason::describe),
                })
            })
            .collect();
        println!(
            "{}",
            serde_json::json!({
                "directory": plan.directory.display().to_string(),
                "actionable": plan.is_actionable(),
                "blocked": plan.blocked_count(),
                "entries": entries,
            })
        );
        return;
    }
    for entry in &plan.entries {
        let from = entry
            .from
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let to = entry
            .to
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        match &entry.skip {
            None => println!("{from} -> {to}"),
            Some(reason) => println!("{from} -> {to}  [{}: {}]", reason.kind(), reason.describe()),
        }
    }
    println!(
        "{} actionable, {} blocked",
        plan.entries.len() - plan.blocked_count(),
        plan.blocked_count()
    );
}

/// The current UTC time as `YYYY-MM-DDTHH:MM:SSZ`, from the system clock.
///
/// The journal takes the timestamp as a parameter so tests never read the clock; this is
/// the one place the CLI supplies it.
fn timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    format_utc(now)
}

/// Format seconds since the Unix epoch as `YYYY-MM-DDTHH:MM:SSZ`.
///
/// A civil-date conversion rather than a date library, because the only requirement is a
/// sortable, file-name-safe stamp.
#[must_use]
pub fn format_utc(seconds: u64) -> String {
    let days = seconds / 86_400;
    let rem = seconds % 86_400;
    let (year, month, day) = civil_from_days(days as i64);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Convert days since 1970-01-01 to a civil date, by Howard Hinnant's algorithm.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_one_request_per_line() {
        let text =
            "{\"file\":\"a.wav\",\"name\":\"b.wav\"}\n\n{\"file\":\"c.wav\",\"name\":\"d.wav\"}\n";

        let requests = parse_requests(text).expect("parsed");

        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].from, PathBuf::from("a.wav"));
        assert_eq!(requests[1].to, "d.wav");
    }

    #[test]
    fn a_bad_line_names_its_number() {
        let text = "{\"file\":\"a.wav\",\"name\":\"b.wav\"}\nnot json\n";

        let err = parse_requests(text).expect_err("bad");

        assert!(err.contains("line 2"), "got {err}");
    }

    #[test]
    fn a_missing_field_is_rejected() {
        let err = parse_requests("{\"file\":\"a.wav\"}\n").expect_err("missing name");

        assert!(err.contains("line 1"), "got {err}");
    }

    #[test]
    fn an_empty_plan_file_is_empty() {
        assert!(parse_requests("\n\n").expect("parsed").is_empty());
    }

    #[test]
    fn a_missing_plan_file_is_reported() {
        let dir = tempfile::tempdir().expect("temp dir");

        let err = read_requests(&dir.path().join("nope.jsonl")).expect_err("missing");

        assert!(err.contains("nope.jsonl"), "got {err}");
    }

    #[test]
    fn formats_the_epoch() {
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn formats_a_known_instant() {
        assert_eq!(format_utc(1_765_000_000), "2025-12-06T05:46:40Z");
    }

    #[test]
    fn formats_a_leap_day() {
        assert_eq!(format_utc(1_709_164_800), "2024-02-29T00:00:00Z");
    }

    #[test]
    fn the_journal_file_name_is_safe() {
        let stamp = format_utc(1_765_000_000);

        let name = slate_tagger::Journal::file_name(&stamp);

        assert_eq!(name, "slate-undo-2025-12-06T05-46-40Z.json");
        assert!(
            name.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_')),
            "got {name}"
        );
    }

    #[test]
    fn a_dry_run_on_a_missing_names_file_is_a_user_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        let args = TagArgs {
            directory: dir.path().to_path_buf(),
            names: Some(dir.path().join("nope.jsonl")),
            apply: false,
            undo: None,
            json: false,
        };

        assert_eq!(run(&args), EXIT_USER);
    }

    #[test]
    fn a_dry_run_without_names_is_a_user_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        let args = TagArgs {
            directory: dir.path().to_path_buf(),
            names: None,
            apply: false,
            undo: None,
            json: false,
        };

        assert_eq!(run(&args), EXIT_USER);
    }

    #[test]
    fn apply_then_undo_round_trips_through_the_cli() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("a.wav"), b"x").expect("write");
        let names = dir.path().join("plan.jsonl");
        std::fs::write(&names, "{\"file\":\"a.wav\",\"name\":\"b.wav\"}\n").expect("write");
        let apply_args = TagArgs {
            directory: dir.path().to_path_buf(),
            names: Some(names),
            apply: true,
            undo: None,
            json: true,
        };

        assert_eq!(run(&apply_args), 0);
        assert!(dir.path().join("b.wav").exists());

        let journal = std::fs::read_dir(dir.path())
            .expect("read dir")
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                p.file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with("slate-undo-"))
            })
            .expect("journal");
        let undo_args = TagArgs {
            directory: dir.path().to_path_buf(),
            names: None,
            apply: false,
            undo: Some(journal),
            json: true,
        };

        assert_eq!(run(&undo_args), 0);
        assert!(dir.path().join("a.wav").exists());
        assert!(!dir.path().join("b.wav").exists());
    }

    #[test]
    fn a_blocked_dry_run_exits_one() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("a.wav"), b"x").expect("write");
        std::fs::write(dir.path().join("b.wav"), b"y").expect("write");
        let names = dir.path().join("plan.jsonl");
        std::fs::write(&names, "{\"file\":\"a.wav\",\"name\":\"b.wav\"}\n").expect("write");
        let args = TagArgs {
            directory: dir.path().to_path_buf(),
            names: Some(names),
            apply: false,
            undo: None,
            json: false,
        };

        assert_eq!(run(&args), EXIT_USER);
    }
}
