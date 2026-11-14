//! The bundled UCS list: version, entry count and content hash.

/// UCS list version shipped with this crate.
pub const UCS_VERSION: &str = "8.2.1";

/// Number of Category/SubCategory rows in the bundled list.
pub const UCS_ENTRY_COUNT: usize = 753;

/// SHA-256 of the bundled CSV, lower-case hex.
///
/// A test asserts this, so an accidental edit to the resource is a failure rather
/// than a silent change to every filename the product produces.
pub const UCS_SHA256: &str = "8d913464119cc2f5ead3a9b2324b7a9ccc8cc0f0eec4494fb4d4d40d22dd5de3";

/// The bundled CSV, compiled into the binary so inference needs no file access.
pub(crate) const UCS_CSV: &str = include_str!("../resources/ucs_v8.2.1.csv");

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::{UCS_CSV, UCS_ENTRY_COUNT, UCS_SHA256};

    #[test]
    fn bundled_csv_matches_its_pinned_hash() {
        let digest = Sha256::digest(UCS_CSV.as_bytes());
        let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();

        assert_eq!(
            hex, UCS_SHA256,
            "bundled UCS CSV changed without a hash update"
        );
    }

    #[test]
    fn bundled_csv_has_the_expected_row_count() {
        let rows = UCS_CSV.lines().count().saturating_sub(1);

        assert_eq!(rows, UCS_ENTRY_COUNT);
    }
}
