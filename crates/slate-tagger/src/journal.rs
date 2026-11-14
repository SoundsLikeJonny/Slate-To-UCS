//! The journal that makes a rename undoable after a restart.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::TaggerError;

/// Prefix of a journal file, followed by the timestamp the caller supplied.
pub const JOURNAL_PREFIX: &str = "slate-undo-";

/// The journal format version. A reader refuses anything else rather than guessing.
pub const JOURNAL_VERSION: u32 = 1;

/// One completed rename.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalEntry {
    /// The name before the rename, relative to the journal's directory.
    pub from: String,
    /// The name after the rename, relative to the journal's directory.
    pub to: String,
}

/// A record of the renames one apply performed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Journal {
    /// The format version, always [`JOURNAL_VERSION`] for a journal this crate writes.
    pub version: u32,
    /// When the apply ran, as the caller supplied it. Never read from the clock here.
    pub created: String,
    /// The directory the renames happened in, as an absolute path.
    pub directory: PathBuf,
    /// The renames, in the order they were applied.
    pub entries: Vec<JournalEntry>,
}

impl Journal {
    /// A journal for a directory and a timestamp.
    #[must_use]
    pub fn new(directory: impl Into<PathBuf>, created: impl Into<String>) -> Self {
        Self {
            version: JOURNAL_VERSION,
            created: created.into(),
            directory: directory.into(),
            entries: Vec::new(),
        }
    }

    /// Record one completed rename.
    pub fn push(&mut self, from: impl Into<String>, to: impl Into<String>) {
        self.entries.push(JournalEntry {
            from: from.into(),
            to: to.into(),
        });
    }

    /// The file name a journal with this timestamp gets.
    ///
    /// Colons are replaced because a timestamp such as `2026-11-14T09:00:00Z` is not a
    /// legal file name on Windows.
    #[must_use]
    pub fn file_name(created: &str) -> String {
        let safe: String = created
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect();
        format!("{JOURNAL_PREFIX}{safe}.json")
    }

    /// Write the journal into its directory, returning the path written.
    ///
    /// # Errors
    ///
    /// Returns [`TaggerError::Io`] when the file cannot be written.
    pub fn write_to(&self, directory: &Path) -> Result<PathBuf, TaggerError> {
        let path = directory.join(Self::file_name(&self.created));
        let json = serde_json::to_string_pretty(self).map_err(|e| TaggerError::Journal {
            path: path.clone(),
            reason: e.to_string(),
        })?;
        std::fs::write(&path, json).map_err(|e| TaggerError::io(&path, e))?;
        Ok(path)
    }

    /// Read a journal from disk, refusing a version this build does not understand.
    ///
    /// # Errors
    ///
    /// Returns [`TaggerError::Io`] when the file cannot be read and
    /// [`TaggerError::Journal`] when it is not a journal of a supported version.
    pub fn read_from(path: &Path) -> Result<Self, TaggerError> {
        let text = std::fs::read_to_string(path).map_err(|e| TaggerError::io(path, e))?;
        let journal: Self = serde_json::from_str(&text).map_err(|e| TaggerError::Journal {
            path: path.to_path_buf(),
            reason: e.to_string(),
        })?;
        if journal.version != JOURNAL_VERSION {
            return Err(TaggerError::Journal {
                path: path.to_path_buf(),
                reason: format!(
                    "version {} is not supported, this build reads {JOURNAL_VERSION}",
                    journal.version
                ),
            });
        }
        Ok(journal)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_journal_is_empty_and_versioned() {
        let journal = Journal::new("/slates", "2026-11-14T09:00:00Z");

        assert_eq!(journal.version, JOURNAL_VERSION);
        assert!(journal.entries.is_empty());
        assert_eq!(journal.directory, PathBuf::from("/slates"));
    }

    #[test]
    fn push_keeps_the_order() {
        let mut journal = Journal::new("/slates", "t");
        journal.push("a.wav", "b.wav");
        journal.push("c.wav", "d.wav");

        assert_eq!(journal.entries[0].from, "a.wav");
        assert_eq!(journal.entries[1].to, "d.wav");
    }

    #[test]
    fn the_file_name_replaces_every_non_alphanumeric() {
        assert_eq!(
            Journal::file_name("2026-11-14T09:00:00Z"),
            "slate-undo-2026-11-14T09-00-00Z.json"
        );
    }

    #[test]
    fn a_journal_round_trips_through_disk() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut journal = Journal::new(dir.path(), "2026-11-14T09:00:00Z");
        journal.push("a.wav", "b.wav");

        let path = journal.write_to(dir.path()).expect("write");
        let read = Journal::read_from(&path).expect("read");

        assert_eq!(read, journal);
    }

    #[test]
    fn a_future_version_is_refused() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("j.json");
        std::fs::write(
            &path,
            r#"{"version":99,"created":"t","directory":"/s","entries":[]}"#,
        )
        .expect("write");

        let err = Journal::read_from(&path).expect_err("version");

        assert_eq!(err.kind(), "journal");
        assert!(err.to_string().contains("99"), "got {err}");
    }

    #[test]
    fn a_non_journal_file_is_refused() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("j.json");
        std::fs::write(&path, "not json").expect("write");

        let err = Journal::read_from(&path).expect_err("json");

        assert_eq!(err.kind(), "journal");
    }

    #[test]
    fn a_missing_journal_is_an_io_error() {
        let dir = tempfile::tempdir().expect("temp dir");

        let err = Journal::read_from(&dir.path().join("nope.json")).expect_err("missing");

        assert_eq!(err.kind(), "io");
    }
}
