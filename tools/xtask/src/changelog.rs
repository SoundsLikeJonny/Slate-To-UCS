//! `cargo xtask changelog`: check or assemble the changelog.
//!
//! `--check` fails when `[Unreleased]` is missing, which is the gate CI runs.
//! `--release X.Y.Z` moves the `[Unreleased]` entries under a dated heading, adds the
//! compare links, and clears the section.

use std::path::Path;

use crate::repo_root;

/// The heading every changelog must carry.
pub const UNRELEASED: &str = "## [Unreleased]";

/// Run the changelog task.
///
/// # Errors
///
/// Returns a message when the file is missing, `[Unreleased]` is absent, or the
/// release version is malformed.
pub fn run(args: &[String]) -> Result<(), String> {
    let root = repo_root();
    let path = root.join("CHANGELOG.md");
    match args.first().map(String::as_str) {
        Some("--check") | None => check(&path),
        Some("--release") => {
            let Some(version) = args.get(1) else {
                return Err("usage: cargo xtask changelog --release <X.Y.Z>".to_string());
            };
            if !crate::bump::is_semver(version) {
                return Err(format!("{version} is not a SemVer version"));
            }
            release(&path, version, &today())
        }
        Some(other) => Err(format!("unknown changelog option: {other}")),
    }
}

/// Fail when the changelog has no `[Unreleased]` section.
///
/// # Errors
///
/// Returns a message when the file cannot be read or the heading is missing.
pub fn check(path: &Path) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if text.contains(UNRELEASED) {
        println!("changelog has an [Unreleased] section");
        Ok(())
    } else {
        Err(format!("{}: no {UNRELEASED} section", path.display()))
    }
}

/// Move `[Unreleased]` entries under `## [version] - date`.
///
/// # Errors
///
/// Returns a message when the file cannot be read or written, or the heading is
/// missing.
pub fn release(path: &Path, version: &str, date: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let updated = release_text(&text, version, date)?;
    std::fs::write(path, updated).map_err(|e| format!("{}: {e}", path.display()))?;
    println!("released {version} in {}", path.display());
    Ok(())
}

/// Pure transformation behind [`release`], so it can be tested without a file.
///
/// # Errors
///
/// Returns a message when the `[Unreleased]` heading is missing.
pub fn release_text(text: &str, version: &str, date: &str) -> Result<String, String> {
    let Some(start) = text.find(UNRELEASED) else {
        return Err(format!("no {UNRELEASED} section"));
    };
    let body_start = start + UNRELEASED.len();
    let rest = &text[body_start..];
    let body_end = rest.find("\n## [").unwrap_or(rest.len());
    let body = rest[..body_end].trim_end();

    let mut out = String::with_capacity(text.len() + 64);
    out.push_str(&text[..start]);
    out.push_str(UNRELEASED);
    out.push_str("\n\n");
    out.push_str(&format!("## [{version}] - {date}\n"));
    out.push_str(body);
    out.push_str("\n\n");
    out.push_str(rest[body_end..].trim_start_matches('\n'));

    Ok(refresh_links(&out, version))
}

/// Point `[Unreleased]` at the new tag and add a compare link for it.
fn refresh_links(text: &str, version: &str) -> String {
    let mut out = String::with_capacity(text.len() + 128);
    let mut added = false;
    for line in text.lines() {
        if line.starts_with("[Unreleased]: ") {
            out.push_str(&format!(
                "[Unreleased]: https://github.com/SoundsLikeJonny/Slate-To-UCS/compare/v{version}...HEAD\n"
            ));
            out.push_str(&format!(
                "[{version}]: https://github.com/SoundsLikeJonny/Slate-To-UCS/releases/tag/v{version}\n"
            ));
            added = true;
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if !added {
        out.push_str(&format!(
            "\n[Unreleased]: https://github.com/SoundsLikeJonny/Slate-To-UCS/compare/v{version}...HEAD\n"
        ));
        out.push_str(&format!(
            "[{version}]: https://github.com/SoundsLikeJonny/Slate-To-UCS/releases/tag/v{version}\n"
        ));
    }
    out
}

/// Today's date in `YYYY-MM-DD`, from the system clock.
fn today() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let days = now / 86_400;
    let (year, month, day) = civil_from_days(i64::try_from(days).unwrap_or(0));
    format!("{year:04}-{month:02}-{day:02}")
}

/// Convert days since the Unix epoch to a civil date.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (
        if m <= 2 { y + 1 } else { y },
        u32::try_from(m).unwrap_or(1),
        u32::try_from(d).unwrap_or(1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "# Changelog\n\n## [Unreleased]\n\n### Added\n- a thing\n\n## [0.1.0] - 2026-01-01\n\n### Added\n- old thing\n\n[Unreleased]: https://example/compare/v0.1.0...HEAD\n[0.1.0]: https://example/tag/v0.1.0\n";

    #[test]
    fn check_accepts_a_changelog_with_unreleased() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("CHANGELOG.md");
        std::fs::write(&path, SAMPLE).expect("write");

        assert!(check(&path).is_ok());
    }

    #[test]
    fn check_rejects_a_changelog_without_unreleased() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("CHANGELOG.md");
        std::fs::write(&path, "# Changelog\n").expect("write");

        assert!(check(&path).is_err());
    }

    #[test]
    fn release_moves_entries_under_a_dated_heading() {
        let updated = release_text(SAMPLE, "0.2.0", "2026-11-14").expect("released");

        assert!(updated.contains("## [0.2.0] - 2026-11-14"), "got {updated}");
        assert!(updated.contains("- a thing"));
        assert!(updated.contains("## [0.1.0] - 2026-01-01"));
    }

    #[test]
    fn release_leaves_an_empty_unreleased_section() {
        let updated = release_text(SAMPLE, "0.2.0", "2026-11-14").expect("released");

        let unreleased_at = updated.find(UNRELEASED).expect("heading");
        let next = updated[unreleased_at + UNRELEASED.len()..]
            .find("## [")
            .expect("next heading");
        let between =
            &updated[unreleased_at + UNRELEASED.len()..unreleased_at + UNRELEASED.len() + next];

        assert!(between.trim().is_empty(), "unreleased body was {between:?}");
    }

    #[test]
    fn release_refreshes_the_compare_links() {
        let updated = release_text(SAMPLE, "0.2.0", "2026-11-14").expect("released");

        assert!(updated.contains("compare/v0.2.0...HEAD"), "got {updated}");
        assert!(updated.contains(
            "[0.2.0]: https://github.com/SoundsLikeJonny/Slate-To-UCS/releases/tag/v0.2.0"
        ));
    }

    #[test]
    fn release_without_unreleased_is_an_error() {
        assert!(release_text("# Changelog\n", "0.2.0", "2026-11-14").is_err());
    }

    #[test]
    fn civil_dates_are_correct_at_known_points() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_000), (2022, 1, 8));
    }

    #[test]
    fn today_looks_like_a_date() {
        let date = today();

        assert_eq!(date.len(), 10);
        assert_eq!(date.chars().filter(|c| *c == '-').count(), 2);
    }

    #[test]
    fn the_repository_changelog_passes_the_check() {
        assert!(check(&repo_root().join("CHANGELOG.md")).is_ok());
    }
}
