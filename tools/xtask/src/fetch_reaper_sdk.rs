//! `cargo xtask fetch-reaper-sdk`: download the pinned REAPER extension headers.
//!
//! The headers are not vendored because they are Cockos's, not ours. They are fetched
//! at a pinned SHA-256 so a build either uses exactly the reviewed bytes or fails, and
//! the destination is gitignored so a checkout never carries them.

use std::path::Path;

use sha2::{Digest, Sha256};

use crate::repo_root;

/// Where the headers are written, relative to this repository.
pub const VENDOR: &str = "apps/reaper/extension/vendor";

/// The two headers an extension needs, with the SHA-256 each must have.
///
/// The hashes were taken from `https://www.reaper.fm/sdk/plugin/` on 2026-11-14. A
/// mismatch means the upstream file changed, which is a review event, not a retry.
pub const HEADERS: &[(&str, &str)] = &[
    (
        "reaper_plugin.h",
        "e94b69bf3dd5573da6c34ac8f35af6911b302c4f79a33be424d2a8e49ba2651b",
    ),
    (
        "reaper_plugin_functions.h",
        "93fadf37e37793a139902d1623e0a07087f433adac13b4c8c5421e3248a42c20",
    ),
];

/// The base URL the headers come from.
pub const BASE_URL: &str = "https://www.reaper.fm/sdk/plugin";

/// Run the fetch.
///
/// # Errors
///
/// Returns a message when a download fails, a hash does not match, or a write fails.
pub fn run(_args: &[String]) -> Result<(), String> {
    fetch_in(&repo_root(), &|url| download(url))
}

/// Fetch every header into `root/VENDOR`, verifying each hash before writing it.
///
/// The downloader is a parameter so the tests can prove the hash gate and the write
/// without a network call.
///
/// # Errors
///
/// Returns a message when a download fails, a hash does not match, or a write fails.
pub fn fetch_in(
    root: &Path,
    downloader: &dyn Fn(&str) -> Result<Vec<u8>, String>,
) -> Result<(), String> {
    let vendor = root.join(VENDOR);
    std::fs::create_dir_all(&vendor).map_err(|e| format!("{}: {e}", vendor.display()))?;

    for (name, expected) in HEADERS {
        let url = format!("{BASE_URL}/{name}");
        let bytes = downloader(&url)?;
        verify_and_write(&vendor, name, &bytes, expected)?;
    }

    let licence = vendor.join("README.md");
    std::fs::write(&licence, LICENCE_NOTE).map_err(|e| format!("{}: {e}", licence.display()))?;
    println!("wrote {}", licence.display());
    Ok(())
}

/// Check one header against its pinned hash and write it only if they agree.
///
/// # Errors
///
/// Returns a message when the hash does not match or the write fails.
pub fn verify_and_write(
    vendor: &Path,
    name: &str,
    bytes: &[u8],
    expected: &str,
) -> Result<(), String> {
    let actual = sha256_hex(bytes);
    if actual != expected {
        return Err(format!(
            "{name}: sha256 {actual} does not match the pinned {expected}; \
             the upstream file changed, so review it before updating HEADERS"
        ));
    }
    let path = vendor.join(name);
    std::fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("{name}: {} bytes, sha256 {actual}", bytes.len());
    Ok(())
}

/// What the vendor directory says about itself.
pub const LICENCE_NOTE: &str = "\
# REAPER extension SDK headers

These files are fetched by `cargo xtask fetch-reaper-sdk` and are not part of this
repository's source. They are Cockos Incorporated's, distributed under the terms in
`reaper_plugin.h` itself; read that header before redistributing anything built with it.

Do not commit this directory. `apps/reaper/extension/vendor/` is gitignored, and the
fetch task refuses a file whose SHA-256 does not match the pinned value.
";

/// Lower-case hex SHA-256 of a byte slice.
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Download a URL into memory.
///
/// # Errors
///
/// Returns a message when the request fails or the status is not success.
pub fn download(url: &str) -> Result<Vec<u8>, String> {
    let output = std::process::Command::new("curl")
        .args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--max-time",
            "60",
        ])
        .arg(url)
        .output()
        .map_err(|e| format!("curl is not installed or not on PATH: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "{url}: curl exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(output.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_matching_hash_is_written() {
        let dir = tempfile::tempdir().expect("temp dir");
        let body = b"header".to_vec();
        let expected = sha256_hex(&body);

        verify_and_write(dir.path(), "reaper_plugin.h", &body, &expected).expect("written");

        let written = std::fs::read(dir.path().join("reaper_plugin.h")).expect("read back");
        assert_eq!(written, body);
    }

    #[test]
    fn a_mismatched_hash_is_refused() {
        let dir = tempfile::tempdir().expect("temp dir");
        let downloader = |_url: &str| Ok(b"tampered".to_vec());

        let error = fetch_in(dir.path(), &downloader).expect_err("hash mismatch");

        assert!(error.contains("does not match the pinned"), "got {error}");
        assert!(
            !dir.path().join(VENDOR).join("reaper_plugin.h").exists(),
            "a refused file must not be written"
        );
    }

    #[test]
    fn verify_and_write_refuses_a_tampered_body() {
        let dir = tempfile::tempdir().expect("temp dir");
        let expected = sha256_hex(b"original");

        let error = verify_and_write(dir.path(), "reaper_plugin.h", b"tampered", &expected)
            .expect_err("tampered");

        assert!(error.contains("does not match the pinned"), "got {error}");
        assert!(!dir.path().join("reaper_plugin.h").exists());
    }

    #[test]
    fn a_download_failure_is_reported() {
        let dir = tempfile::tempdir().expect("temp dir");
        let downloader = |url: &str| Err(format!("{url}: offline"));

        let error = fetch_in(dir.path(), &downloader).expect_err("offline");

        assert!(error.contains("offline"), "got {error}");
    }

    #[test]
    fn the_pinned_hashes_are_well_formed() {
        for (name, hash) in HEADERS {
            assert_eq!(hash.len(), 64, "{name} hash must be 64 hex characters");
            assert!(
                hash.chars()
                    .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                "{name} hash must be lower-case hex"
            );
        }
    }

    #[test]
    fn the_header_list_covers_both_headers() {
        let names: Vec<&str> = HEADERS.iter().map(|(name, _)| *name).collect();

        assert_eq!(names, vec!["reaper_plugin.h", "reaper_plugin_functions.h"]);
    }

    #[test]
    fn the_licence_note_names_the_fetch_task_and_the_directory() {
        assert!(LICENCE_NOTE.contains("fetch-reaper-sdk"));
        assert!(LICENCE_NOTE.contains("apps/reaper/extension/vendor/"));
        assert!(LICENCE_NOTE.contains("Cockos"));
    }

    #[test]
    fn sha256_hex_matches_a_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }
}
