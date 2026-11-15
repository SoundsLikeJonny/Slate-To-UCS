//! Applying a plan, and undoing what an apply did.

use std::path::{Path, PathBuf};

use crate::journal::Journal;
use crate::plan::Plan;
use crate::TaggerError;

/// What an apply did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApplyOutcome {
    /// The journal that was written, with one entry per completed rename.
    pub journal: Journal,
    /// Where the journal was written.
    pub journal_path: PathBuf,
    /// How many files were renamed.
    pub renamed: usize,
}

/// What an undo did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UndoOutcome {
    /// How many files were moved back.
    pub restored: usize,
    /// How many journal entries needed no action because the file was already back.
    pub already_restored: usize,
}

/// Apply a plan, writing the journal before each rename so a crash is recoverable.
///
/// Each rename is a single `rename` call inside one directory, which is atomic on every
/// supported platform; there is no window where a file has neither name. The journal is
/// rewritten after every completed rename, so a crash leaves a journal that lists exactly
/// the renames that happened.
///
/// # Errors
///
/// Returns [`TaggerError::NotActionable`] when the plan has blocked entries,
/// [`TaggerError::WouldOverwrite`] when a destination appeared after planning, and
/// [`TaggerError::Io`] when a rename or a journal write fails.
pub fn apply(plan: &Plan, created: &str) -> Result<ApplyOutcome, TaggerError> {
    if !plan.is_actionable() {
        return Err(TaggerError::NotActionable(plan.blocked_count()));
    }

    let mut journal = Journal::new(absolute(&plan.directory), created);
    let journal_path = journal.write_to(&plan.directory)?;
    let mut renamed = 0;

    for entry in plan.actionable() {
        if entry.to.exists() {
            return Err(TaggerError::WouldOverwrite {
                path: entry.to.clone(),
            });
        }
        std::fs::rename(&entry.from, &entry.to).map_err(|e| TaggerError::io(&entry.from, e))?;
        journal.push(
            file_name(&entry.from).unwrap_or_default(),
            file_name(&entry.to).unwrap_or_default(),
        );
        journal.write_to(&plan.directory)?;
        renamed += 1;
    }

    Ok(ApplyOutcome {
        journal,
        journal_path,
        renamed,
    })
}

/// Undo a journal, restoring every name in reverse order.
///
/// An entry whose renamed file is already gone is counted as already restored rather than
/// treated as an error, so running undo twice is safe.
///
/// # Errors
///
/// Returns [`TaggerError::UndoBlocked`] when a file already occupies the name that would
/// be restored, and [`TaggerError::Io`] when a rename fails.
pub fn undo(journal_path: &Path) -> Result<UndoOutcome, TaggerError> {
    let journal = Journal::read_from(journal_path)?;
    let mut restored = 0;
    let mut already_restored = 0;

    for entry in journal.entries.iter().rev() {
        let from = journal.directory.join(&entry.from);
        let to = journal.directory.join(&entry.to);
        if !to.exists() {
            already_restored += 1;
            continue;
        }
        if from.exists() {
            return Err(TaggerError::UndoBlocked { path: from });
        }
        std::fs::rename(&to, &from).map_err(|e| TaggerError::io(&to, e))?;
        restored += 1;
    }

    Ok(UndoOutcome {
        restored,
        already_restored,
    })
}

/// The final component of a path, as a string, for the journal.
fn file_name(path: &Path) -> Option<String> {
    path.file_name().map(|n| n.to_string_lossy().into_owned())
}

/// The absolute form of a directory, so an undo works from any working directory.
///
/// Falls back to the path as given when it cannot be canonicalised, which keeps the
/// journal writable on a filesystem that does not support it.
fn absolute(directory: &Path) -> PathBuf {
    std::fs::canonicalize(directory).unwrap_or_else(|_| directory.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan::{plan_renames, PlanOptions, RenameRequest};

    fn dir_with(names: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("temp dir");
        for name in names {
            std::fs::write(dir.path().join(name), name.as_bytes()).expect("write");
        }
        dir
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut out: Vec<String> = std::fs::read_dir(dir)
            .expect("read dir")
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| !n.starts_with(crate::JOURNAL_PREFIX))
            .collect();
        out.sort();
        out
    }

    fn plan_for(dir: &Path, pairs: &[(&str, &str)]) -> Plan {
        let requests: Vec<RenameRequest> = pairs
            .iter()
            .map(|(from, to)| RenameRequest::new(dir.join(from), *to))
            .collect();
        plan_renames(dir, &requests, &PlanOptions::default())
    }

    #[test]
    fn apply_renames_every_actionable_entry() {
        let dir = dir_with(&["a.wav", "b.wav"]);
        let plan = plan_for(dir.path(), &[("a.wav", "x.wav"), ("b.wav", "y.wav")]);

        let outcome = apply(&plan, "2026-11-14T09:00:00Z").expect("apply");

        assert_eq!(outcome.renamed, 2);
        assert_eq!(names(dir.path()), vec!["x.wav", "y.wav"]);
    }

    #[test]
    fn apply_writes_a_journal_beside_the_files() {
        let dir = dir_with(&["a.wav"]);
        let plan = plan_for(dir.path(), &[("a.wav", "x.wav")]);

        let outcome = apply(&plan, "2026-11-14T09:00:00Z").expect("apply");

        assert!(outcome.journal_path.exists());
        assert_eq!(
            outcome.journal_path.file_name().unwrap().to_string_lossy(),
            "slate-undo-2026-11-14T09-00-00Z.json"
        );
        assert_eq!(outcome.journal.entries.len(), 1);
        assert_eq!(outcome.journal.entries[0].from, "a.wav");
        assert_eq!(outcome.journal.entries[0].to, "x.wav");
    }

    #[test]
    fn apply_refuses_a_plan_with_a_blocked_entry() {
        let dir = dir_with(&["a.wav", "b.wav"]);
        let plan = plan_for(dir.path(), &[("a.wav", "b.wav")]);

        let err = apply(&plan, "t").expect_err("blocked");

        assert_eq!(err.kind(), "not_actionable");
        assert_eq!(names(dir.path()), vec!["a.wav", "b.wav"]);
    }

    #[test]
    fn apply_refuses_a_destination_that_appeared_after_planning() {
        let dir = dir_with(&["a.wav"]);
        let plan = plan_for(dir.path(), &[("a.wav", "x.wav")]);
        std::fs::write(dir.path().join("x.wav"), b"late").expect("write");

        let err = apply(&plan, "t").expect_err("race");

        assert_eq!(err.kind(), "would_overwrite");
        assert!(dir.path().join("a.wav").exists());
        assert_eq!(
            std::fs::read(dir.path().join("x.wav")).expect("read"),
            b"late"
        );
    }

    #[test]
    fn an_empty_plan_applies_and_writes_an_empty_journal() {
        let dir = dir_with(&[]);
        let plan = plan_for(dir.path(), &[]);

        let outcome = apply(&plan, "t").expect("apply");

        assert_eq!(outcome.renamed, 0);
        assert!(outcome.journal.entries.is_empty());
    }

    #[test]
    fn undo_restores_every_name() {
        let dir = dir_with(&["a.wav", "b.wav"]);
        let before = names(dir.path());
        let plan = plan_for(dir.path(), &[("a.wav", "x.wav"), ("b.wav", "y.wav")]);
        let outcome = apply(&plan, "t").expect("apply");

        let undo = undo(&outcome.journal_path).expect("undo");

        assert_eq!(undo.restored, 2);
        assert_eq!(undo.already_restored, 0);
        assert_eq!(names(dir.path()), before);
    }

    #[test]
    fn undo_restores_the_bytes_of_each_file() {
        let dir = dir_with(&["a.wav"]);
        let plan = plan_for(dir.path(), &[("a.wav", "x.wav")]);
        let outcome = apply(&plan, "t").expect("apply");

        undo(&outcome.journal_path).expect("undo");

        assert_eq!(
            std::fs::read(dir.path().join("a.wav")).expect("read"),
            b"a.wav"
        );
    }

    #[test]
    fn undo_twice_is_safe() {
        let dir = dir_with(&["a.wav"]);
        let plan = plan_for(dir.path(), &[("a.wav", "x.wav")]);
        let outcome = apply(&plan, "t").expect("apply");
        undo(&outcome.journal_path).expect("first undo");

        let second = undo(&outcome.journal_path).expect("second undo");

        assert_eq!(second.restored, 0);
        assert_eq!(second.already_restored, 1);
        assert_eq!(names(dir.path()), vec!["a.wav"]);
    }

    #[test]
    fn undo_refuses_when_the_original_name_is_taken() {
        let dir = dir_with(&["a.wav"]);
        let plan = plan_for(dir.path(), &[("a.wav", "x.wav")]);
        let outcome = apply(&plan, "t").expect("apply");
        std::fs::write(dir.path().join("a.wav"), b"new").expect("write");

        let err = undo(&outcome.journal_path).expect_err("blocked");

        assert_eq!(err.kind(), "undo_blocked");
        assert_eq!(
            std::fs::read(dir.path().join("a.wav")).expect("read"),
            b"new"
        );
        assert!(dir.path().join("x.wav").exists());
    }

    #[test]
    fn a_swap_is_refused_because_the_target_still_exists() {
        let dir = dir_with(&["a.wav", "b.wav"]);

        let plan = plan_for(dir.path(), &[("a.wav", "b.wav"), ("b.wav", "a.wav")]);

        assert_eq!(
            plan.entries[0].skip,
            Some(crate::plan::SkipReason::CollisionWithExisting(
                dir.path().join("b.wav")
            ))
        );
        assert_eq!(plan.blocked_count(), 2);
    }

    #[test]
    fn undo_reverses_in_order() {
        let dir = dir_with(&["a.wav", "b.wav"]);
        let plan = plan_for(dir.path(), &[("a.wav", "c.wav"), ("b.wav", "d.wav")]);
        let outcome = apply(&plan, "t").expect("apply");
        assert_eq!(names(dir.path()), vec!["c.wav", "d.wav"]);

        undo(&outcome.journal_path).expect("undo");

        assert_eq!(names(dir.path()), vec!["a.wav", "b.wav"]);
    }

    #[test]
    fn the_journal_records_an_absolute_directory() {
        let dir = dir_with(&["a.wav"]);
        let plan = plan_for(dir.path(), &[("a.wav", "x.wav")]);

        let outcome = apply(&plan, "t").expect("apply");

        assert!(outcome.journal.directory.is_absolute());
        assert_eq!(
            outcome.journal.directory,
            std::fs::canonicalize(dir.path()).expect("canonical")
        );
    }

    #[test]
    fn apply_leaves_no_temporary_files() {
        let dir = dir_with(&["a.wav", "b.wav"]);
        let plan = plan_for(dir.path(), &[("a.wav", "x.wav"), ("b.wav", "y.wav")]);

        apply(&plan, "t").expect("apply");

        let leftovers: Vec<String> = std::fs::read_dir(dir.path())
            .expect("read dir")
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.contains("tmp"))
            .collect();
        assert!(leftovers.is_empty(), "leftovers: {leftovers:?}");
    }
}
