//! Where the bundled `ttucs` tree lives and how to start it offline.

use std::path::{Path, PathBuf};

use slate_engine::{Engine, SpawnOptions, STARTUP_TIMEOUT, WARM_TIMEOUT};

/// The variable that keeps the sidecar from ever reaching the network.
pub const OFFLINE_ENV: &str = "TTUCS_OFFLINE";

/// The bundled `ttucs` onedir tree.
///
/// The tree is a PyInstaller `onedir` layout, not `onefile`: torch unpacks slowly from
/// a single file, and the app has to show a window while the model loads. The
/// executable sits at the root of the tree on every platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sidecar {
    root: PathBuf,
}

impl Sidecar {
    /// A sidecar rooted at a directory.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The directory holding the sidecar tree.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// The `ttucs` executable inside the tree.
    ///
    /// Windows ships `ttucs.exe`; the other platforms ship `ttucs`. The name is chosen
    /// by the target, not by probing the filesystem, so a missing tree fails at spawn
    /// with a clear message instead of silently picking the wrong file.
    #[must_use]
    pub fn binary(&self) -> PathBuf {
        self.root
            .join(if cfg!(windows) { "ttucs.exe" } else { "ttucs" })
    }

    /// Whether the executable exists on disk.
    #[must_use]
    pub fn is_installed(&self) -> bool {
        self.binary().is_file()
    }

    /// Spawn options that start this sidecar offline.
    ///
    /// `TTUCS_OFFLINE=1` is set on the child only. Setting it on the shell's own
    /// environment would leak into every other thread and into any child the shell
    /// starts later.
    #[must_use]
    pub fn spawn_options(&self, endpoint_path: impl Into<PathBuf>) -> SpawnOptions {
        SpawnOptions::new(self.binary().to_string_lossy().to_string(), endpoint_path)
            .with_env(OFFLINE_ENV, "1")
    }

    /// Start the sidecar and wait for it to warm.
    ///
    /// # Errors
    ///
    /// As [`Engine::spawn_with_options`], plus [`slate_engine::EngineError::NotInstalled`]
    /// when the tree is missing.
    pub fn spawn(
        &self,
        endpoint_path: impl Into<PathBuf>,
    ) -> Result<Engine, slate_engine::EngineError> {
        Engine::spawn_with_options(&self.spawn_options(endpoint_path))
    }

    /// Start the sidecar with explicit timeouts, for tests and for a slow first run.
    ///
    /// # Errors
    ///
    /// As [`Sidecar::spawn`].
    pub fn spawn_with_timeouts(
        &self,
        endpoint_path: impl Into<PathBuf>,
        startup_timeout: std::time::Duration,
        warm_timeout: std::time::Duration,
    ) -> Result<Engine, slate_engine::EngineError> {
        let mut options = self.spawn_options(endpoint_path);
        options.startup_timeout = startup_timeout;
        options.warm_timeout = warm_timeout;
        Engine::spawn_with_options(&options)
    }
}

impl Default for Sidecar {
    fn default() -> Self {
        Self::new(default_root())
    }
}

/// The sidecar directory a bundled app uses, relative to the executable.
///
/// Tauri copies `resources/` next to the executable on Windows and Linux and into
/// `Contents/Resources` on macOS; the caller passes the resolved resource directory,
/// so this only names the subdirectory.
#[must_use]
pub fn default_root() -> PathBuf {
    PathBuf::from("resources").join("ttucs")
}

/// The default timeouts the shell uses for a cold start.
#[must_use]
pub fn default_timeouts() -> (std::time::Duration, std::time::Duration) {
    (STARTUP_TIMEOUT, WARM_TIMEOUT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_binary_name_matches_the_platform() {
        let sidecar = Sidecar::new("/opt/slate/resources/ttucs");

        let name = sidecar
            .binary()
            .file_name()
            .expect("file name")
            .to_string_lossy()
            .to_string();

        if cfg!(windows) {
            assert_eq!(name, "ttucs.exe");
        } else {
            assert_eq!(name, "ttucs");
        }
    }

    #[test]
    fn the_root_is_preserved() {
        let sidecar = Sidecar::new("/opt/slate/resources/ttucs");

        assert_eq!(sidecar.root(), Path::new("/opt/slate/resources/ttucs"));
        assert_eq!(sidecar.binary().parent(), Some(sidecar.root()));
    }

    #[test]
    fn a_missing_tree_is_not_installed() {
        let dir = tempfile::tempdir().expect("temp dir");
        let sidecar = Sidecar::new(dir.path());

        assert!(!sidecar.is_installed());
    }

    #[test]
    fn a_present_executable_is_installed() {
        let dir = tempfile::tempdir().expect("temp dir");
        let sidecar = Sidecar::new(dir.path());
        std::fs::write(sidecar.binary(), b"stub").expect("write stub");

        assert!(sidecar.is_installed());
    }

    #[test]
    fn spawn_options_are_offline_and_point_at_the_tree() {
        let sidecar = Sidecar::new("/opt/slate/resources/ttucs");

        let options = sidecar.spawn_options("/tmp/serve.json");

        assert_eq!(options.binary, sidecar.binary().to_string_lossy());
        assert_eq!(options.endpoint_path, Path::new("/tmp/serve.json"));
        assert_eq!(
            options.env,
            vec![(OFFLINE_ENV.to_string(), "1".to_string())]
        );
        assert_eq!(options.startup_timeout, STARTUP_TIMEOUT);
        assert_eq!(options.warm_timeout, WARM_TIMEOUT);
    }

    #[test]
    fn spawning_a_missing_tree_reports_not_installed() {
        let dir = tempfile::tempdir().expect("temp dir");
        let sidecar = Sidecar::new(dir.path().join("absent"));

        let err = sidecar
            .spawn_with_timeouts(
                dir.path().join("serve.json"),
                std::time::Duration::from_millis(100),
                std::time::Duration::from_millis(100),
            )
            .expect_err("missing tree");

        assert!(
            matches!(err, slate_engine::EngineError::NotInstalled(_)),
            "got {err:?}"
        );
    }

    #[test]
    fn the_default_root_is_under_resources() {
        let root = default_root();

        assert!(root.ends_with(Path::new("resources").join("ttucs")));
    }

    #[test]
    fn the_default_timeouts_are_the_engine_ones() {
        assert_eq!(default_timeouts(), (STARTUP_TIMEOUT, WARM_TIMEOUT));
    }

    #[test]
    fn the_default_sidecar_uses_the_default_root() {
        assert_eq!(Sidecar::default().root(), default_root());
    }
}
