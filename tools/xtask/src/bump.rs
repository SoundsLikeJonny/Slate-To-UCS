//! `cargo xtask bump X.Y.Z`: write the product version into every manifest.

use std::path::Path;

use crate::repo_root;

/// Run the bump task.
///
/// # Errors
///
/// Returns a message when the version is missing, malformed, or a manifest cannot be
/// read or written.
pub fn run(args: &[String]) -> Result<(), String> {
    let Some(version) = args.first() else {
        return Err("usage: cargo xtask bump <X.Y.Z>".to_string());
    };
    if !is_semver(version) {
        return Err(format!("{version} is not a SemVer version"));
    }
    let root = repo_root();
    bump_in(&root, version)
}

/// Write `version` into every manifest under `root`.
///
/// # Errors
///
/// Returns a message when a manifest cannot be read or written.
pub fn bump_in(root: &Path, version: &str) -> Result<(), String> {
    write_version_file(&root.join("version.toml"), version)?;
    write_cargo_workspace(&root.join("Cargo.toml"), version)?;
    write_package_json(&root.join("package.json"), version)?;
    Ok(())
}

/// Whether a string is a plain `MAJOR.MINOR.PATCH` version.
#[must_use]
pub fn is_semver(value: &str) -> bool {
    let parts: Vec<&str> = value.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

fn write_version_file(path: &Path, version: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let updated = replace_assignment(&text, "version", version);
    std::fs::write(path, updated).map_err(|e| format!("{}: {e}", path.display()))
}

fn write_cargo_workspace(path: &Path, version: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let updated = replace_assignment(&text, "version", version);
    std::fs::write(path, updated).map_err(|e| format!("{}: {e}", path.display()))
}

fn write_package_json(path: &Path, version: &str) -> Result<(), String> {
    if !path.is_file() {
        return Ok(());
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let updated = replace_json_version(&text, version);
    std::fs::write(path, updated).map_err(|e| format!("{}: {e}", path.display()))
}

/// Replace the first `key = "..."` assignment, preserving formatting.
#[must_use]
pub fn replace_assignment(text: &str, key: &str, value: &str) -> String {
    let needle = format!("{key} = \"");
    let mut out = String::with_capacity(text.len());
    let mut replaced = false;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if !replaced && trimmed.starts_with(&needle) {
            let indent = &line[..line.len() - trimmed.len()];
            let tail = line.trim_end().rsplit('"').next().unwrap_or("");
            out.push_str(&format!("{indent}{key} = \"{value}\"{tail}"));
            if line.ends_with('\n') {
                out.push('\n');
            }
            replaced = true;
        } else {
            out.push_str(line);
        }
    }
    out
}

/// Replace the top-level `"version": "..."` value in a JSON document.
#[must_use]
pub fn replace_json_version(text: &str, value: &str) -> String {
    let needle = "\"version\": \"";
    let Some(start) = text.find(needle) else {
        return text.to_string();
    };
    let value_start = start + needle.len();
    let Some(end_rel) = text[value_start..].find('"') else {
        return text.to_string();
    };
    let end = value_start + end_rel;
    format!("{}{}{}", &text[..value_start], value, &text[end..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_semver() {
        assert!(is_semver("0.1.0"));
        assert!(is_semver("12.34.56"));
    }

    #[test]
    fn rejects_partial_or_decorated_versions() {
        assert!(!is_semver("1.0"));
        assert!(!is_semver("1.0.0-beta"));
        assert!(!is_semver("v1.0.0"));
        assert!(!is_semver(""));
    }

    #[test]
    fn replaces_a_toml_assignment_and_keeps_the_comment() {
        let text = "# header\nversion = \"0.1.0\"  # trailing\nother = 1\n";

        let updated = replace_assignment(text, "version", "0.2.0");

        assert!(
            updated.contains("version = \"0.2.0\"  # trailing"),
            "got {updated}"
        );
        assert!(updated.contains("other = 1"));
    }

    #[test]
    fn replaces_only_the_first_assignment() {
        let text = "version = \"0.1.0\"\nversion = \"9.9.9\"\n";

        let updated = replace_assignment(text, "version", "0.2.0");

        assert!(updated.contains("version = \"0.2.0\""));
        assert!(updated.contains("version = \"9.9.9\""));
    }

    #[test]
    fn leaves_text_without_the_key_alone() {
        let text = "name = \"x\"\n";

        assert_eq!(replace_assignment(text, "version", "0.2.0"), text);
    }

    #[test]
    fn replaces_the_json_version() {
        let text = "{\n  \"name\": \"x\",\n  \"version\": \"0.1.0\"\n}\n";

        let updated = replace_json_version(text, "0.2.0");

        assert!(updated.contains("\"version\": \"0.2.0\""), "got {updated}");
        assert!(updated.contains("\"name\": \"x\""));
    }

    #[test]
    fn leaves_json_without_a_version_alone() {
        let text = "{\"name\": \"x\"}";

        assert_eq!(replace_json_version(text, "0.2.0"), text);
    }

    #[test]
    fn bump_writes_every_manifest() {
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join("version.toml"), "version = \"0.1.0\"\n").expect("write");
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace.package]\nversion = \"0.1.0\"\n",
        )
        .expect("write");
        std::fs::write(
            dir.path().join("package.json"),
            "{\n  \"version\": \"0.1.0\"\n}\n",
        )
        .expect("write");

        bump_in(dir.path(), "0.2.0").expect("bumped");

        assert!(std::fs::read_to_string(dir.path().join("version.toml"))
            .expect("read")
            .contains("0.2.0"));
        assert!(std::fs::read_to_string(dir.path().join("Cargo.toml"))
            .expect("read")
            .contains("0.2.0"));
        assert!(std::fs::read_to_string(dir.path().join("package.json"))
            .expect("read")
            .contains("0.2.0"));
    }

    #[test]
    fn bump_rejects_a_bad_version() {
        assert!(run(&["1.0".to_string()]).is_err());
        assert!(run(&[]).is_err());
    }
}
