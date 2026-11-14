//! The CatID newtype: syntax validation and rendering.

use std::fmt;

use crate::error::UcsError;
use crate::taxonomy::{Taxonomy, UcsEntry};

/// A UCS CatID, validated for syntax and optionally for membership in the list.
///
/// Construction never touches the network or the filesystem. [`CatId::parse`] checks
/// syntax only; [`CatId::resolve`] additionally requires the taxonomy.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CatId(String);

impl CatId {
    /// Longest CatID the UCS list contains, with headroom.
    const MAX_LEN: usize = 24;

    /// Validate CatID syntax without consulting a taxonomy.
    ///
    /// A CatID is ASCII alphanumeric, starts with an uppercase letter, and is at most
    /// [`CatId::MAX_LEN`] characters.
    ///
    /// # Errors
    ///
    /// Returns [`UcsError::InvalidCatId`] for an empty value, a non-ASCII character,
    /// a leading non-letter, or an over-long value.
    pub fn parse(value: &str) -> Result<Self, UcsError> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(UcsError::InvalidCatId(value.to_string()));
        }
        if trimmed.len() > Self::MAX_LEN {
            return Err(UcsError::InvalidCatId(value.to_string()));
        }
        let mut chars = trimmed.chars();
        let first = chars.next().unwrap_or(' ');
        if !first.is_ascii_uppercase() {
            return Err(UcsError::InvalidCatId(value.to_string()));
        }
        if !trimmed.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(UcsError::InvalidCatId(value.to_string()));
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Resolve a value against a taxonomy, returning the canonical spelling.
    ///
    /// # Errors
    ///
    /// Returns [`UcsError::UnknownCatId`] when the value is not in the list.
    pub fn resolve(value: &str, taxonomy: &Taxonomy) -> Result<Self, UcsError> {
        let entry: &UcsEntry = taxonomy.get(value)?;
        Ok(Self(entry.cat_id.clone()))
    }

    /// The canonical CatID string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CatId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_canonical_cat_id() {
        assert_eq!(
            CatId::parse("DOORCreak").expect("valid").as_str(),
            "DOORCreak"
        );
    }

    #[test]
    fn trims_surrounding_whitespace() {
        assert_eq!(
            CatId::parse("  AIRBlow ").expect("valid").as_str(),
            "AIRBlow"
        );
    }

    #[test]
    fn rejects_empty_and_whitespace_only() {
        assert!(matches!(CatId::parse(""), Err(UcsError::InvalidCatId(_))));
        assert!(matches!(
            CatId::parse("   "),
            Err(UcsError::InvalidCatId(_))
        ));
    }

    #[test]
    fn rejects_lowercase_leading_character() {
        assert!(matches!(
            CatId::parse("doorCreak"),
            Err(UcsError::InvalidCatId(_))
        ));
    }

    #[test]
    fn rejects_non_alphanumeric_characters() {
        assert!(matches!(
            CatId::parse("DOOR-Creak"),
            Err(UcsError::InvalidCatId(_))
        ));
        assert!(matches!(
            CatId::parse("DOOR Creak"),
            Err(UcsError::InvalidCatId(_))
        ));
    }

    #[test]
    fn rejects_over_long_values() {
        let long = format!("A{}", "B".repeat(CatId::MAX_LEN));

        assert!(matches!(
            CatId::parse(&long),
            Err(UcsError::InvalidCatId(_))
        ));
    }

    #[test]
    fn resolve_returns_the_canonical_spelling() {
        let tax = Taxonomy::bundled().expect("bundled");

        let id = CatId::resolve("doorcreak", &tax).expect("resolved");

        assert_eq!(id.as_str(), "DOORCreak");
    }

    #[test]
    fn resolve_rejects_unknown_values() {
        let tax = Taxonomy::bundled().expect("bundled");

        assert!(matches!(
            CatId::resolve("NOPE", &tax),
            Err(UcsError::UnknownCatId(_))
        ));
    }

    #[test]
    fn display_prints_the_value() {
        assert_eq!(
            CatId::parse("AIRBlow").expect("valid").to_string(),
            "AIRBlow"
        );
    }
}
