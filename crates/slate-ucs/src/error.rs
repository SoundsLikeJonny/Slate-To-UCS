//! Errors raised by the UCS layer.

use thiserror::Error;

/// Everything that can go wrong while resolving a CatID, building a filename or
/// loading the taxonomy.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum UcsError {
    /// The CatID is not in the UCS list and is not a known alias.
    #[error("unknown UCS CatID: {0}")]
    UnknownCatId(String),

    /// The CatID is syntactically invalid before any lookup is attempted.
    #[error("invalid UCS CatID: {0}")]
    InvalidCatId(String),

    /// A filename was built from an empty FX name.
    #[error("FX name must not be empty")]
    EmptyFxName,

    /// A filename block contains a character UCS or the filesystem forbids.
    #[error("forbidden character {ch:?} in {field}")]
    ForbiddenCharacter {
        /// Which block the character appeared in.
        field: &'static str,
        /// The offending character.
        ch: char,
    },

    /// A filename block exceeds its length budget.
    #[error("{field} is {len} characters, maximum is {max}")]
    TooLong {
        /// Which block was too long.
        field: &'static str,
        /// Its actual length in characters.
        len: usize,
        /// The budget it exceeded.
        max: usize,
    },

    /// A filename does not have the four required blocks.
    #[error("UCS filenames need at least 4 '_' separated blocks: {0}")]
    TooFewBlocks(String),

    /// The taxonomy CSV is empty or malformed.
    #[error("taxonomy is malformed: {0}")]
    MalformedTaxonomy(String),
}
