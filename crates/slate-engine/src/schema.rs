//! The runtime contract version gate.

use crate::error::EngineError;

/// The `schema_version` this client implements.
///
/// The engine reports its own in `/health` and in every response envelope. A client
/// refuses an unknown version rather than guessing at the shape.
pub const SCHEMA_VERSION: u32 = 1;

/// Accept `version` or fail with a message the UI can show.
///
/// # Errors
///
/// Returns [`EngineError::SchemaMismatch`] when `version` is not
/// [`SCHEMA_VERSION`].
pub fn check_schema_version(version: u32) -> Result<(), EngineError> {
    if version == SCHEMA_VERSION {
        Ok(())
    } else {
        Err(EngineError::SchemaMismatch {
            found: version,
            expected: SCHEMA_VERSION,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_supported_version() {
        assert!(check_schema_version(SCHEMA_VERSION).is_ok());
    }

    #[test]
    fn rejects_a_newer_version_with_both_numbers() {
        let err = check_schema_version(SCHEMA_VERSION + 1).expect_err("newer");

        match err {
            EngineError::SchemaMismatch { found, expected } => {
                assert_eq!(found, SCHEMA_VERSION + 1);
                assert_eq!(expected, SCHEMA_VERSION);
            }
            other => panic!("wrong variant: {other:?}"),
        }
    }

    #[test]
    fn rejects_version_zero() {
        assert!(check_schema_version(0).is_err());
    }
}
