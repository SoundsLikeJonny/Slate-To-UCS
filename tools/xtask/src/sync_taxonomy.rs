//! `cargo xtask sync-taxonomy`: refresh the bundled UCS CSV and its pinned hash.

use std::path::Path;

use sha2::{Digest, Sha256};

use crate::repo_root;

/// Where the upstream CSV lives, relative to this repository.
pub const UPSTREAM: &str = "../ttucs/src/ttucs/resources/ucs_v8.2.1.csv";

/// Where the bundled copy lives, relative to this repository.
pub const BUNDLED: &str = "crates/slate-ucs/resources/ucs_v8.2.1.csv";

/// Where the pinned hash lives, relative to this repository.
pub const RESOURCES_RS: &str = "crates/slate-ucs/src/resources.rs";

/// Run the taxonomy sync.
///
/// # Errors
///
/// Returns a message when the upstream file is missing or a write fails.
pub fn run(_args: &[String]) -> Result<(), String> {
    sync_in(&repo_root())
}

/// Copy the upstream CSV into the crate and update the pinned hash.
///
/// # Errors
///
/// Returns a message when the upstream file is missing or a write fails.
pub fn sync_in(root: &Path) -> Result<(), String> {
    let upstream = root.join(UPSTREAM);
    if !upstream.is_file() {
        return Err(format!(
            "{} is missing; check out the ttucs repository next to this one",
            upstream.display()
        ));
    }
    let bytes = std::fs::read(&upstream).map_err(|e| format!("{}: {e}", upstream.display()))?;
    let bundled = root.join(BUNDLED);
    if let Some(parent) = bundled.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
    }
    std::fs::write(&bundled, &bytes).map_err(|e| format!("{}: {e}", bundled.display()))?;

    let hash = sha256_hex(&bytes);
    let resources = root.join(RESOURCES_RS);
    let text =
        std::fs::read_to_string(&resources).map_err(|e| format!("{}: {e}", resources.display()))?;
    let updated = replace_hash(&text, &hash);
    std::fs::write(&resources, updated).map_err(|e| format!("{}: {e}", resources.display()))?;

    println!("synced {} bytes, sha256 {hash}", bytes.len());
    Ok(())
}

/// Lower-case hex SHA-256 of a byte slice.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// Replace the `UCS_SHA256` constant value.
#[must_use]
pub fn replace_hash(text: &str, hash: &str) -> String {
    let needle = "pub const UCS_SHA256: &str = \"";
    let Some(start) = text.find(needle) else {
        return text.to_string();
    };
    let value_start = start + needle.len();
    let Some(end_rel) = text[value_start..].find('"') else {
        return text.to_string();
    };
    format!(
        "{}{}{}",
        &text[..value_start],
        hash,
        &text[value_start + end_rel..]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_a_known_value() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn replaces_the_pinned_hash() {
        let text = "pub const UCS_SHA256: &str = \"old\";\n";

        let updated = replace_hash(text, "new");

        assert_eq!(updated, "pub const UCS_SHA256: &str = \"new\";\n");
    }

    #[test]
    fn leaves_text_without_the_constant_alone() {
        let text = "fn main() {}\n";

        assert_eq!(replace_hash(text, "new"), text);
    }

    #[test]
    fn a_missing_upstream_is_reported() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().join("repo");
        std::fs::create_dir_all(&root).expect("dirs");

        let err = sync_in(&root).expect_err("missing upstream");

        assert!(err.contains("ttucs"), "got {err}");
    }

    #[test]
    fn sync_copies_the_file_and_updates_the_hash() {
        let dir = tempfile::tempdir().expect("temp dir");
        // `UPSTREAM` starts with `..`, so the repository root must sit one level below
        // the temporary directory for the sibling checkout to stay inside it.
        let root = dir.path().join("repo");
        std::fs::create_dir_all(&root).expect("dirs");
        let upstream = root.join(UPSTREAM);
        std::fs::create_dir_all(upstream.parent().expect("parent")).expect("dirs");
        std::fs::write(&upstream, "Category,SubCategory,CatID,CatShort\n").expect("write");
        let resources = root.join(RESOURCES_RS);
        std::fs::create_dir_all(resources.parent().expect("parent")).expect("dirs");
        std::fs::write(&resources, "pub const UCS_SHA256: &str = \"old\";\n").expect("write");

        sync_in(&root).expect("synced");

        let bundled = std::fs::read(root.join(BUNDLED)).expect("read");
        let expected = sha256_hex(&bundled);
        let text = std::fs::read_to_string(&resources).expect("read");
        assert!(text.contains(&expected), "hash not updated: {text}");
    }
}
