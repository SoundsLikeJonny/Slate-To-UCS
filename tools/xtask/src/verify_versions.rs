//! `cargo xtask verify-versions`: fail if the manifests disagree.
//!
//! This is the check that stops a release shipping a stale version in one surface.

use std::path::Path;

use crate::repo_root;

/// Run the verification task.
///
/// # Errors
///
/// Returns a message listing every manifest whose version differs from
/// `version.toml`.
pub fn run(_args: &[String]) -> Result<(), String> {
    verify_in(&repo_root())
}

/// Compare every manifest under `root` against `version.toml`.
///
/// # Errors
///
/// Returns a message naming each disagreement.
pub fn verify_in(root: &Path) -> Result<(), String> {
    let expected = read_version_toml(&root.join("version.toml"))?;
    let mut problems: Vec<String> = Vec::new();

    match read_cargo_workspace(&root.join("Cargo.toml")) {
        Ok(found) if found == expected => {}
        Ok(found) => problems.push(format!("Cargo.toml has {found}, expected {expected}")),
        Err(e) => problems.push(e),
    }

    let package_json = root.join("package.json");
    if package_json.is_file() {
        match read_package_json(&package_json) {
            Ok(found) if found == expected => {}
            Ok(found) => problems.push(format!("package.json has {found}, expected {expected}")),
            Err(e) => problems.push(e),
        }
    }

    if problems.is_empty() {
        println!("versions agree on {expected}");
        Ok(())
    } else {
        Err(problems.join("\n"))
    }
}

/// Read `version` from a `version.toml`.
///
/// # Errors
///
/// Returns a message when the file is missing, unparseable, or has no version.
pub fn read_version_toml(path: &Path) -> Result<String, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let value: toml::Value = text
        .parse()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    value
        .get("version")
        .and_then(toml::Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{}: no version key", path.display()))
}

/// Read `workspace.package.version` from the root `Cargo.toml`.
///
/// # Errors
///
/// Returns a message when the file is missing, unparseable, or has no such key.
pub fn read_cargo_workspace(path: &Path) -> Result<String, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let value: toml::Value = text
        .parse()
        .map_err(|e| format!("{}: {e}", path.display()))?;
    value
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(toml::Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{}: no workspace.package.version", path.display()))
}

/// Read the top-level `version` from a `package.json`.
///
/// # Errors
///
/// Returns a message when the file is unreadable, unparseable, or has no version.
pub fn read_package_json(path: &Path) -> Result<String, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    value
        .get("version")
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| format!("{}: no version key", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, body: &str) {
        std::fs::write(dir.join(name), body).expect("write");
    }

    #[test]
    fn agrees_when_every_manifest_matches() {
        let dir = tempfile::tempdir().expect("temp dir");
        write(dir.path(), "version.toml", "version = \"0.1.0\"\n");
        write(
            dir.path(),
            "Cargo.toml",
            "[workspace.package]\nversion = \"0.1.0\"\n",
        );
        write(dir.path(), "package.json", "{\"version\": \"0.1.0\"}");

        assert!(verify_in(dir.path()).is_ok());
    }

    #[test]
    fn reports_a_cargo_mismatch() {
        let dir = tempfile::tempdir().expect("temp dir");
        write(dir.path(), "version.toml", "version = \"0.1.0\"\n");
        write(
            dir.path(),
            "Cargo.toml",
            "[workspace.package]\nversion = \"0.0.9\"\n",
        );

        let err = verify_in(dir.path()).expect_err("mismatch");

        assert!(err.contains("Cargo.toml has 0.0.9"), "got {err}");
    }

    #[test]
    fn reports_a_package_json_mismatch() {
        let dir = tempfile::tempdir().expect("temp dir");
        write(dir.path(), "version.toml", "version = \"0.1.0\"\n");
        write(
            dir.path(),
            "Cargo.toml",
            "[workspace.package]\nversion = \"0.1.0\"\n",
        );
        write(dir.path(), "package.json", "{\"version\": \"9.9.9\"}");

        let err = verify_in(dir.path()).expect_err("mismatch");

        assert!(err.contains("package.json has 9.9.9"), "got {err}");
    }

    #[test]
    fn a_missing_version_file_is_an_error() {
        let dir = tempfile::tempdir().expect("temp dir");

        assert!(verify_in(dir.path()).is_err());
    }

    #[test]
    fn a_version_file_without_the_key_is_an_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        write(dir.path(), "version.toml", "name = \"x\"\n");

        let err = verify_in(dir.path()).expect_err("no key");

        assert!(err.contains("no version key"), "got {err}");
    }

    #[test]
    fn the_repository_itself_agrees() {
        assert!(verify_in(&repo_root()).is_ok());
    }
}
