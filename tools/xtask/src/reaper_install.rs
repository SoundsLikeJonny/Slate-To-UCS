//! Install and remove the REAPER integration, reversibly.
//!
//! The installer copies the Lua scripts into REAPER's resource directory and writes a
//! manifest listing exactly what it wrote. Removal reads that manifest and deletes only
//! those files, so a user's own scripts in the same tree are never touched.
//!
//! Every path decision takes its environment as a parameter rather than reading the
//! process globals, so the detection rules are testable on any platform.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// The directory name the scripts are installed under, inside REAPER's resource path.
pub const INSTALL_DIR: &str = "Slate-To-UCS";

/// The manifest file name, written inside [`INSTALL_DIR`].
pub const MANIFEST_NAME: &str = "install-manifest.json";

/// The scripts copied into REAPER, relative to `apps/reaper/scripts`.
pub const SCRIPTS: &[&str] = &[
    "slate_tag_selected.lua",
    "slate_tag_project.lua",
    "slate_render_pattern.lua",
    "slate_open_app.lua",
    "lib/slate_api.lua",
];

/// The environment the detection rules read.
#[derive(Debug, Clone, Default)]
pub struct Env {
    /// `%APPDATA%` on Windows.
    pub appdata: Option<PathBuf>,
    /// `$HOME` on macOS and Linux.
    pub home: Option<PathBuf>,
    /// `$XDG_CONFIG_HOME`, which overrides the Linux default when set.
    pub xdg_config_home: Option<PathBuf>,
    /// The operating system, as `std::env::consts::OS` spells it.
    pub os: String,
    /// A directory holding a portable `reaper.ini`, when one was found.
    pub portable: Option<PathBuf>,
}

/// Where the scripts go, and whether the install is portable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// REAPER's resource directory.
    pub resource_dir: PathBuf,
    /// Whether this is a portable install, which keeps everything beside the executable.
    pub portable: bool,
}

/// One file the installer will write.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopyOp {
    /// The source path on disk.
    pub source: PathBuf,
    /// The destination path on disk.
    pub destination: PathBuf,
    /// The SHA-256 of the source bytes, recorded in the manifest.
    pub sha256: String,
}

/// What the manifest records, so removal is exact.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// The product version that wrote the manifest.
    pub version: String,
    /// The resource directory the files were written under.
    pub resource_dir: PathBuf,
    /// Every file written, relative to `resource_dir`, with its hash.
    pub files: BTreeMap<String, String>,
}

/// Resolve REAPER's resource directory from an environment.
///
/// A portable install wins over the per-user one, because a recordist who runs REAPER
/// from a USB stick expects the scripts to travel with it.
///
/// # Errors
///
/// Returns a message naming the missing variable when the platform's variable is unset.
pub fn detect_target(env: &Env) -> Result<Target, String> {
    if let Some(portable) = &env.portable {
        return Ok(Target {
            resource_dir: portable.clone(),
            portable: true,
        });
    }

    let resource_dir = match env.os.as_str() {
        "windows" => env
            .appdata
            .as_ref()
            .map(|base| base.join("REAPER"))
            .ok_or_else(|| {
                "APPDATA is not set, so REAPER's resource directory is unknown".to_string()
            })?,
        "macos" => env
            .home
            .as_ref()
            .map(|home| home.join("Library/Application Support/REAPER"))
            .ok_or_else(|| {
                "HOME is not set, so REAPER's resource directory is unknown".to_string()
            })?,
        _ => {
            let base = env
                .xdg_config_home
                .clone()
                .or_else(|| env.home.as_ref().map(|home| home.join(".config")))
                .ok_or_else(|| {
                    "neither XDG_CONFIG_HOME nor HOME is set, so REAPER's resource directory is unknown"
                        .to_string()
                })?;
            base.join("REAPER")
        }
    };

    Ok(Target {
        resource_dir,
        portable: false,
    })
}

/// Whether a directory looks like a portable REAPER install.
///
/// A portable install is identified by `reaper.ini` sitting beside the executable, which
/// is what REAPER itself writes when it is told to keep its settings local.
#[must_use]
pub fn is_portable(candidate: &Path) -> bool {
    candidate.join("reaper.ini").is_file()
}

/// Find a portable install among the given candidates.
#[must_use]
pub fn find_portable(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|path| is_portable(path)).cloned()
}

/// The destination of one script inside the resource directory.
#[must_use]
pub fn destination_for(resource_dir: &Path, relative: &str) -> PathBuf {
    let mut path = resource_dir.join("Scripts").join(INSTALL_DIR);
    for part in relative.split('/') {
        path = path.join(part);
    }
    path
}

/// Plan the copies for a set of scripts.
///
/// # Errors
///
/// Returns a message when a source cannot be read.
pub fn plan(scripts_root: &Path, target: &Target) -> Result<Vec<CopyOp>, String> {
    let mut ops = Vec::with_capacity(SCRIPTS.len());
    for relative in SCRIPTS {
        let source = scripts_root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
        let bytes =
            std::fs::read(&source).map_err(|e| format!("cannot read {}: {e}", source.display()))?;
        ops.push(CopyOp {
            source,
            destination: destination_for(&target.resource_dir, relative),
            sha256: hex(&Sha256::digest(&bytes)),
        });
    }
    Ok(ops)
}

/// Write the planned files and the manifest.
///
/// # Errors
///
/// Returns a message when a directory cannot be created or a file cannot be written.
pub fn install(ops: &[CopyOp], target: &Target, version: &str) -> Result<Manifest, String> {
    let mut files = BTreeMap::new();
    for op in ops {
        if let Some(parent) = op.destination.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("cannot create {}: {e}", parent.display()))?;
        }
        std::fs::copy(&op.source, &op.destination).map_err(|e| {
            format!(
                "cannot copy {} to {}: {e}",
                op.source.display(),
                op.destination.display()
            )
        })?;
        let relative = op
            .destination
            .strip_prefix(&target.resource_dir)
            .map_err(|_| {
                format!(
                    "{} is outside {}",
                    op.destination.display(),
                    target.resource_dir.display()
                )
            })?
            .to_string_lossy()
            .replace('\\', "/");
        files.insert(relative, op.sha256.clone());
    }

    let manifest = Manifest {
        version: version.to_string(),
        resource_dir: target.resource_dir.clone(),
        files,
    };
    write_manifest(&manifest)?;
    Ok(manifest)
}

/// Serialise a manifest as JSON.
#[must_use]
pub fn manifest_json(manifest: &Manifest) -> String {
    let files: serde_json::Map<String, serde_json::Value> = manifest
        .files
        .iter()
        .map(|(path, hash)| (path.clone(), serde_json::Value::String(hash.clone())))
        .collect();
    let document = serde_json::json!({
        "version": manifest.version,
        "resource_dir": manifest.resource_dir.to_string_lossy(),
        "files": files,
    });
    serde_json::to_string_pretty(&document).unwrap_or_else(|_| "{}".to_string())
}

/// Parse a manifest from JSON.
///
/// # Errors
///
/// Returns a message when the document is not an object with the expected fields.
pub fn parse_manifest(text: &str) -> Result<Manifest, String> {
    let value: serde_json::Value =
        serde_json::from_str(text).map_err(|e| format!("manifest is not JSON: {e}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| "manifest is not a JSON object".to_string())?;

    let version = object
        .get("version")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "manifest has no version".to_string())?
        .to_string();
    let resource_dir = object
        .get("resource_dir")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "manifest has no resource_dir".to_string())?
        .to_string();
    let files_object = object
        .get("files")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| "manifest has no files".to_string())?;

    let mut files = BTreeMap::new();
    for (path, hash) in files_object {
        let hash = hash
            .as_str()
            .ok_or_else(|| format!("manifest hash for {path} is not a string"))?;
        files.insert(path.clone(), hash.to_string());
    }

    Ok(Manifest {
        version,
        resource_dir: PathBuf::from(resource_dir),
        files,
    })
}

/// Write the manifest beside the installed scripts.
///
/// # Errors
///
/// Returns a message when the directory cannot be created or the file cannot be written.
pub fn write_manifest(manifest: &Manifest) -> Result<PathBuf, String> {
    let directory = manifest.resource_dir.join("Scripts").join(INSTALL_DIR);
    std::fs::create_dir_all(&directory)
        .map_err(|e| format!("cannot create {}: {e}", directory.display()))?;
    let path = directory.join(MANIFEST_NAME);
    std::fs::write(&path, manifest_json(manifest))
        .map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok(path)
}

/// Read the manifest from a resource directory.
///
/// # Errors
///
/// Returns a message when the manifest is missing or unreadable.
pub fn read_manifest(resource_dir: &Path) -> Result<Manifest, String> {
    let path = resource_dir
        .join("Scripts")
        .join(INSTALL_DIR)
        .join(MANIFEST_NAME);
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    parse_manifest(&text)
}

/// Remove every file the manifest lists, then the manifest itself.
///
/// A file that is already gone is not an error: a user may have deleted one by hand, and
/// removal must still finish. Returns the number of files removed.
///
/// # Errors
///
/// Returns a message when a file exists but cannot be removed.
pub fn uninstall(manifest: &Manifest) -> Result<usize, String> {
    let mut removed = 0;
    for relative in manifest.files.keys() {
        let path = manifest
            .resource_dir
            .join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
        match std::fs::remove_file(&path) {
            Ok(()) => removed += 1,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(format!("cannot remove {}: {e}", path.display())),
        }
    }

    let manifest_path = manifest
        .resource_dir
        .join("Scripts")
        .join(INSTALL_DIR)
        .join(MANIFEST_NAME);
    match std::fs::remove_file(&manifest_path) {
        Ok(()) => removed += 1,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("cannot remove {}: {e}", manifest_path.display())),
    }

    // The directory is removed only when empty, so a user's own file in it survives.
    let _ = std::fs::remove_dir(manifest.resource_dir.join("Scripts").join(INSTALL_DIR));
    Ok(removed)
}

/// Lowercase hex, which is how every other hash in this repository is written.
fn hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Run the task from the command line.
///
/// # Errors
///
/// Returns a message when detection, install or removal fails.
pub fn run(args: &[String]) -> Result<(), String> {
    let mut uninstall_requested = false;
    let mut resource_dir: Option<PathBuf> = None;
    let mut portable: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--uninstall" => uninstall_requested = true,
            "--resource-dir" => {
                index += 1;
                resource_dir = args.get(index).map(PathBuf::from);
            }
            "--portable" => {
                index += 1;
                portable = args.get(index).map(PathBuf::from);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
        index += 1;
    }

    let target = if let Some(dir) = resource_dir {
        Target {
            resource_dir: dir,
            portable: portable.is_some(),
        }
    } else {
        // A portable install keeps its settings beside the executable, so the directory
        // holding reaper.ini wins over the per-user resource path.
        let portable = portable.or_else(|| {
            let mut candidates = Vec::new();
            if let Some(exe) = std::env::var_os("REAPER_EXE") {
                if let Some(parent) = PathBuf::from(exe).parent() {
                    candidates.push(parent.to_path_buf());
                }
            }
            if let Some(dir) = std::env::var_os("REAPER_PORTABLE") {
                candidates.push(PathBuf::from(dir));
            }
            find_portable(&candidates)
        });
        let env = Env {
            appdata: std::env::var_os("APPDATA").map(PathBuf::from),
            home: std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .map(PathBuf::from),
            xdg_config_home: std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from),
            os: std::env::consts::OS.to_string(),
            portable,
        };
        detect_target(&env)?
    };

    if uninstall_requested {
        let manifest = read_manifest(&target.resource_dir)?;
        let removed = uninstall(&manifest)?;
        println!(
            "removed {removed} files from {}",
            target.resource_dir.display()
        );
        return Ok(());
    }

    let scripts_root = super::repo_root().join("apps/reaper/scripts");
    let ops = plan(&scripts_root, &target)?;
    let version = crate_version()?;
    let manifest = install(&ops, &target, &version)?;
    println!(
        "installed {} scripts into {} (REAPER {})",
        manifest.files.len(),
        target.resource_dir.display(),
        if target.portable {
            "portable"
        } else {
            "per-user"
        }
    );
    Ok(())
}

/// The product version, read from `version.toml`.
fn crate_version() -> Result<String, String> {
    let path = super::repo_root().join("version.toml");
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let value: toml::Value = text
        .parse()
        .map_err(|e| format!("{} is not TOML: {e}", path.display()))?;
    value
        .get("version")
        .and_then(toml::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| format!("{} has no version", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn windows_env() -> Env {
        Env {
            appdata: Some(PathBuf::from("C:/Users/recordist/AppData/Roaming")),
            os: "windows".to_string(),
            ..Env::default()
        }
    }

    #[test]
    fn detect_target_uses_appdata_on_windows() {
        let target = detect_target(&windows_env()).expect("detected");

        assert_eq!(
            target.resource_dir,
            PathBuf::from("C:/Users/recordist/AppData/Roaming/REAPER")
        );
        assert!(!target.portable);
    }

    #[test]
    fn detect_target_uses_application_support_on_macos() {
        let env = Env {
            home: Some(PathBuf::from("/Users/recordist")),
            os: "macos".to_string(),
            ..Env::default()
        };

        let target = detect_target(&env).expect("detected");

        assert_eq!(
            target.resource_dir,
            PathBuf::from("/Users/recordist/Library/Application Support/REAPER")
        );
    }

    #[test]
    fn detect_target_prefers_xdg_config_home_on_linux() {
        let env = Env {
            home: Some(PathBuf::from("/home/recordist")),
            xdg_config_home: Some(PathBuf::from("/home/recordist/.config-custom")),
            os: "linux".to_string(),
            ..Env::default()
        };

        let target = detect_target(&env).expect("detected");

        assert_eq!(
            target.resource_dir,
            PathBuf::from("/home/recordist/.config-custom/REAPER")
        );
    }

    #[test]
    fn detect_target_falls_back_to_dot_config_on_linux() {
        let env = Env {
            home: Some(PathBuf::from("/home/recordist")),
            os: "linux".to_string(),
            ..Env::default()
        };

        let target = detect_target(&env).expect("detected");

        assert_eq!(
            target.resource_dir,
            PathBuf::from("/home/recordist/.config/REAPER")
        );
    }

    #[test]
    fn detect_target_prefers_a_portable_install() {
        let env = Env {
            portable: Some(PathBuf::from("E:/REAPER")),
            ..windows_env()
        };

        let target = detect_target(&env).expect("detected");

        assert_eq!(target.resource_dir, PathBuf::from("E:/REAPER"));
        assert!(target.portable);
    }

    #[test]
    fn detect_target_names_the_missing_variable() {
        let env = Env {
            os: "windows".to_string(),
            ..Env::default()
        };

        let error = detect_target(&env).expect_err("no appdata");

        assert!(error.contains("APPDATA"), "got {error}");
    }

    #[test]
    fn detect_target_names_the_missing_variable_on_linux() {
        let env = Env {
            os: "linux".to_string(),
            ..Env::default()
        };

        let error = detect_target(&env).expect_err("no home");

        assert!(error.contains("XDG_CONFIG_HOME"), "got {error}");
    }

    #[test]
    fn is_portable_requires_reaper_ini() {
        let dir = tempfile::tempdir().expect("temp dir");
        assert!(!is_portable(dir.path()));

        std::fs::write(dir.path().join("reaper.ini"), "[REAPER]\n").expect("write");

        assert!(is_portable(dir.path()));
    }

    #[test]
    fn find_portable_returns_the_first_match() {
        let first = tempfile::tempdir().expect("temp dir");
        let second = tempfile::tempdir().expect("temp dir");
        std::fs::write(second.path().join("reaper.ini"), "[REAPER]\n").expect("write");

        let found = find_portable(&[first.path().to_path_buf(), second.path().to_path_buf()]);

        assert_eq!(found, Some(second.path().to_path_buf()));
    }

    #[test]
    fn find_portable_returns_none_when_nothing_matches() {
        let dir = tempfile::tempdir().expect("temp dir");

        assert_eq!(find_portable(&[dir.path().to_path_buf()]), None);
    }

    #[test]
    fn destination_for_keeps_the_lib_subdirectory() {
        let path = destination_for(Path::new("/r"), "lib/slate_api.lua");

        assert_eq!(path, Path::new("/r/Scripts/Slate-To-UCS/lib/slate_api.lua"));
    }

    #[test]
    fn plan_hashes_every_script() {
        let dir = tempfile::tempdir().expect("temp dir");
        for relative in SCRIPTS {
            let path = dir
                .path()
                .join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).expect("mkdir");
            }
            std::fs::write(&path, format!("-- {relative}\n")).expect("write");
        }
        let target = Target {
            resource_dir: PathBuf::from("/r"),
            portable: false,
        };

        let ops = plan(dir.path(), &target).expect("planned");

        assert_eq!(ops.len(), SCRIPTS.len());
        assert!(ops.iter().all(|op| op.sha256.len() == 64));
    }

    #[test]
    fn plan_reports_a_missing_script() {
        let dir = tempfile::tempdir().expect("temp dir");
        let target = Target {
            resource_dir: PathBuf::from("/r"),
            portable: false,
        };

        let error = plan(dir.path(), &target).expect_err("missing");

        assert!(error.contains("cannot read"), "got {error}");
    }

    #[test]
    fn install_writes_the_files_and_the_manifest() {
        let root = tempfile::tempdir().expect("temp dir");
        let scripts = root.path().join("scripts");
        std::fs::create_dir_all(scripts.join("lib")).expect("mkdir");
        std::fs::write(scripts.join("slate_tag_selected.lua"), "-- a\n").expect("write");
        std::fs::write(scripts.join("lib/slate_api.lua"), "-- b\n").expect("write");
        let target = Target {
            resource_dir: root.path().join("reaper"),
            portable: false,
        };
        let ops = vec![
            CopyOp {
                source: scripts.join("slate_tag_selected.lua"),
                destination: destination_for(&target.resource_dir, "slate_tag_selected.lua"),
                sha256: "a".repeat(64),
            },
            CopyOp {
                source: scripts.join("lib/slate_api.lua"),
                destination: destination_for(&target.resource_dir, "lib/slate_api.lua"),
                sha256: "b".repeat(64),
            },
        ];

        let manifest = install(&ops, &target, "0.1.0").expect("installed");

        assert_eq!(manifest.files.len(), 2);
        assert!(destination_for(&target.resource_dir, "slate_tag_selected.lua").is_file());
        assert!(destination_for(&target.resource_dir, "lib/slate_api.lua").is_file());
        assert!(target
            .resource_dir
            .join("Scripts/Slate-To-UCS")
            .join(MANIFEST_NAME)
            .is_file());
    }

    #[test]
    fn install_records_forward_slashes_in_the_manifest() {
        let root = tempfile::tempdir().expect("temp dir");
        let source = root.path().join("a.lua");
        std::fs::write(&source, "-- a\n").expect("write");
        let target = Target {
            resource_dir: root.path().join("reaper"),
            portable: false,
        };
        let ops = vec![CopyOp {
            source,
            destination: destination_for(&target.resource_dir, "lib/slate_api.lua"),
            sha256: "c".repeat(64),
        }];

        let manifest = install(&ops, &target, "0.1.0").expect("installed");

        assert!(manifest
            .files
            .contains_key("Scripts/Slate-To-UCS/lib/slate_api.lua"));
    }

    #[test]
    fn manifest_round_trips_through_json() {
        let mut files = BTreeMap::new();
        files.insert("Scripts/Slate-To-UCS/a.lua".to_string(), "d".repeat(64));
        let manifest = Manifest {
            version: "0.1.0".to_string(),
            resource_dir: PathBuf::from("/r"),
            files,
        };

        let parsed = parse_manifest(&manifest_json(&manifest)).expect("parsed");

        assert_eq!(parsed, manifest);
    }

    #[test]
    fn parse_manifest_rejects_a_non_object() {
        let error = parse_manifest("[]").expect_err("array");

        assert!(error.contains("not a JSON object"), "got {error}");
    }

    #[test]
    fn parse_manifest_rejects_a_missing_version() {
        let error = parse_manifest(r#"{"resource_dir":"/r","files":{}}"#).expect_err("no version");

        assert!(error.contains("version"), "got {error}");
    }

    #[test]
    fn parse_manifest_rejects_a_missing_files_map() {
        let error = parse_manifest(r#"{"version":"1","resource_dir":"/r"}"#).expect_err("no files");

        assert!(error.contains("files"), "got {error}");
    }

    #[test]
    fn parse_manifest_rejects_a_non_string_hash() {
        let error = parse_manifest(r#"{"version":"1","resource_dir":"/r","files":{"a":1}}"#)
            .expect_err("bad hash");

        assert!(error.contains("not a string"), "got {error}");
    }

    #[test]
    fn uninstall_removes_exactly_the_manifest_files() {
        let root = tempfile::tempdir().expect("temp dir");
        let target = Target {
            resource_dir: root.path().join("reaper"),
            portable: false,
        };
        let installed = destination_for(&target.resource_dir, "slate_tag_selected.lua");
        let mine = target.resource_dir.join("Scripts/Slate-To-UCS/mine.lua");
        std::fs::create_dir_all(installed.parent().expect("parent")).expect("mkdir");
        std::fs::write(&installed, "-- installed\n").expect("write");
        std::fs::write(&mine, "-- the user's own\n").expect("write");
        let mut files = BTreeMap::new();
        files.insert(
            "Scripts/Slate-To-UCS/slate_tag_selected.lua".to_string(),
            "e".repeat(64),
        );
        let manifest = Manifest {
            version: "0.1.0".to_string(),
            resource_dir: target.resource_dir.clone(),
            files,
        };
        write_manifest(&manifest).expect("manifest");

        let removed = uninstall(&manifest).expect("uninstalled");

        assert_eq!(removed, 2, "the script and the manifest");
        assert!(!installed.exists());
        assert!(mine.exists(), "a user's own file must survive");
    }

    #[test]
    fn uninstall_tolerates_a_file_that_is_already_gone() {
        let root = tempfile::tempdir().expect("temp dir");
        let mut files = BTreeMap::new();
        files.insert(
            "Scripts/Slate-To-UCS/missing.lua".to_string(),
            "f".repeat(64),
        );
        let manifest = Manifest {
            version: "0.1.0".to_string(),
            resource_dir: root.path().to_path_buf(),
            files,
        };

        let removed = uninstall(&manifest).expect("uninstalled");

        assert_eq!(removed, 0);
    }

    #[test]
    fn read_manifest_reports_a_missing_file() {
        let root = tempfile::tempdir().expect("temp dir");

        let error = read_manifest(root.path()).expect_err("missing");

        assert!(error.contains("cannot read"), "got {error}");
    }

    #[test]
    fn run_rejects_an_unknown_argument() {
        let error = run(&["--nope".to_string()]).expect_err("unknown");

        assert!(error.contains("unknown argument"), "got {error}");
    }

    #[test]
    fn run_installs_and_uninstalls_into_an_explicit_directory() {
        let root = tempfile::tempdir().expect("temp dir");
        let resource = root.path().join("reaper");

        run(&[
            "--resource-dir".to_string(),
            resource.to_string_lossy().to_string(),
        ])
        .expect("installed");
        assert!(destination_for(&resource, "slate_tag_selected.lua").is_file());

        run(&[
            "--resource-dir".to_string(),
            resource.to_string_lossy().to_string(),
            "--uninstall".to_string(),
        ])
        .expect("uninstalled");
        assert!(!destination_for(&resource, "slate_tag_selected.lua").exists());
    }

    #[test]
    fn crate_version_matches_the_version_file() {
        let version = crate_version().expect("version");

        assert!(!version.is_empty());
        assert!(version.split('.').count() >= 2, "got {version}");
    }
}
