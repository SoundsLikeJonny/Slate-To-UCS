//! The error type for planning, applying and undoing a rename.

use std::path::PathBuf;

use thiserror::Error;

/// Everything that can go wrong in this crate.
#[derive(Debug, Error)]
pub enum TaggerError {
    /// A file could not be read or written.
    #[error("{path}: {source}")]
    Io {
        /// The path involved.
        path: PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },

    /// A journal file was not the JSON this crate writes.
    #[error("{path}: not a slate journal: {reason}")]
    Journal {
        /// The journal path.
        path: PathBuf,
        /// Why it was rejected.
        reason: String,
    },

    /// A rename would have overwritten an existing file.
    #[error("{path} already exists; refusing to overwrite")]
    WouldOverwrite {
        /// The destination that is already taken.
        path: PathBuf,
    },

    /// The plan contained at least one entry that cannot be applied.
    #[error("the plan has {0} entries that cannot be applied; run a dry run first")]
    NotActionable(usize),

    /// An undo found a file it expected to move back already occupied.
    #[error("cannot restore {path}: it already exists")]
    UndoBlocked {
        /// The path that blocked the restore.
        path: PathBuf,
    },
}

impl TaggerError {
    /// A stable, lower-snake-case identifier for the front end and the CLI.
    ///
    /// The strings are part of the CLI contract, so a test asserts they are unique and
    /// that every variant has one.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Io { .. } => "io",
            Self::Journal { .. } => "journal",
            Self::WouldOverwrite { .. } => "would_overwrite",
            Self::NotActionable(_) => "not_actionable",
            Self::UndoBlocked { .. } => "undo_blocked",
        }
    }

    /// Wrap an [`std::io::Error`] with the path that produced it.
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn every_variant() -> Vec<TaggerError> {
        vec![
            TaggerError::io("/a", std::io::Error::other("boom")),
            TaggerError::Journal {
                path: PathBuf::from("/a"),
                reason: "bad".to_string(),
            },
            TaggerError::WouldOverwrite {
                path: PathBuf::from("/a"),
            },
            TaggerError::NotActionable(2),
            TaggerError::UndoBlocked {
                path: PathBuf::from("/a"),
            },
        ]
    }

    #[test]
    fn every_variant_has_a_distinct_kind() {
        let kinds: Vec<&str> = every_variant().iter().map(TaggerError::kind).collect();
        let mut unique = kinds.clone();
        unique.sort_unstable();
        unique.dedup();

        assert_eq!(kinds.len(), unique.len(), "kinds: {kinds:?}");
    }

    #[test]
    fn kinds_are_lower_snake_case() {
        for kind in every_variant().iter().map(TaggerError::kind) {
            assert!(
                kind.chars()
                    .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit()),
                "kind {kind} is not lower snake case"
            );
        }
    }

    #[test]
    fn messages_name_the_path() {
        let err = TaggerError::io("/slates/a.wav", std::io::Error::other("denied"));

        assert!(err.to_string().contains("/slates/a.wav"), "got {err}");
        assert!(err.to_string().contains("denied"), "got {err}");
    }

    #[test]
    fn the_overwrite_message_says_it_refused() {
        let err = TaggerError::WouldOverwrite {
            path: PathBuf::from("/slates/b.wav"),
        };

        assert!(
            err.to_string().contains("refusing to overwrite"),
            "got {err}"
        );
    }
}
