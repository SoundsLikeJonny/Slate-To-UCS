//! Rename planning, journalling and undo for slate files.
//!
//! This is the only destructive part of the product, so it is built as three separate
//! steps that can each be tested on their own:
//!
//! 1. [`plan_renames`] decides what would move, and refuses anything unsafe before a
//!    single file is touched.
//! 2. [`apply`] writes a [`Journal`] first, then renames. A crash between the two leaves
//!    a journal that [`undo`] can replay.
//! 3. [`undo`] restores every name byte for byte, in reverse order.
//!
//! Nothing here reads the wall clock: the caller passes the timestamp, so a test can
//! assert an exact journal.
//!
//! ```
//! use slate_tagger::{plan_renames, PlanOptions, RenameRequest};
//!
//! let plan = plan_renames(
//!     std::path::Path::new("/slates"),
//!     &[RenameRequest::new("/slates/a.wav", "DOORCreak_A_NONE_NONE.wav")],
//!     &PlanOptions::default(),
//! );
//!
//! assert_eq!(plan.entries.len(), 1);
//! assert!(plan.is_actionable());
//! ```

#![deny(missing_docs)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod error;
mod journal;
mod plan;
mod rename;

pub use error::TaggerError;
pub use journal::{Journal, JournalEntry, JOURNAL_PREFIX, JOURNAL_VERSION};
pub use plan::{plan_renames, Plan, PlanEntry, PlanOptions, RenameRequest, SkipReason};
pub use rename::{apply, undo, ApplyOutcome, UndoOutcome};

/// Longest file name, in bytes, that the tool will produce on any supported platform.
///
/// Windows counts UTF-16 units against 255 and Linux counts bytes; staying at 240 leaves
/// room for a suffix without crossing either limit.
pub const MAX_FILENAME_BYTES: usize = 240;
