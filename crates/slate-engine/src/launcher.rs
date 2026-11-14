//! The three launcher steps: attach, spawn, shut down only if owned.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::client::Client;
use crate::endpoint::Endpoint;
use crate::error::EngineError;
use crate::schema::check_schema_version;
use crate::types::{Health, Prediction};

/// How long to wait for the endpoint file to appear after spawning.
pub const STARTUP_TIMEOUT: Duration = Duration::from_secs(30);

/// How long to wait for the model to report itself warm on a first run.
pub const WARM_TIMEOUT: Duration = Duration::from_secs(60);

/// Poll interval while waiting for the endpoint file or a warm model.
pub const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Environment variable that overrides the `ttucs` executable.
pub const TTUCS_BIN_ENV: &str = "SLATE_TTUCS_BIN";

/// A connected engine, and whether this process started it.
#[derive(Debug)]
pub struct Engine {
    client: Client,
    owned: bool,
}

impl Engine {
    /// Attach to a running server, if there is one.
    ///
    /// Reads the endpoint file, checks the contract version, and probes `/health`.
    /// Returns `Ok(None)` when no server is running, which is not an error.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::EndpointFile`] for an unreadable endpoint file and
    /// [`EngineError::SchemaMismatch`] for an unsupported contract version.
    pub fn connect() -> Result<Option<Self>, EngineError> {
        let Some(endpoint) = Endpoint::read()? else {
            return Ok(None);
        };
        check_schema_version(endpoint.schema_version)?;
        let client = Client::new(endpoint);
        match client.health() {
            Ok(_) => Ok(Some(Self {
                client,
                owned: false,
            })),
            Err(EngineError::Transport(_)) | Err(EngineError::Status { .. }) => Ok(None),
            Err(other) => Err(other),
        }
    }

    /// Attach to a running server or start one.
    ///
    /// # Errors
    ///
    /// Propagates [`Engine::connect`] errors and the spawn errors from
    /// [`Engine::spawn`].
    pub fn connect_or_spawn() -> Result<Self, EngineError> {
        match Self::connect()? {
            Some(engine) => Ok(engine),
            None => Self::spawn(),
        }
    }

    /// Start `ttucs serve`, wait for its endpoint file, then wait for it to warm.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::NotInstalled`] when the executable cannot be started,
    /// [`EngineError::StartupTimeout`] when no endpoint file appears, and
    /// [`EngineError::WarmTimeout`] when the model never warms.
    pub fn spawn() -> Result<Self, EngineError> {
        Self::spawn_with(
            &ttucs_binary(),
            &Endpoint::default_path(),
            STARTUP_TIMEOUT,
            WARM_TIMEOUT,
        )
    }

    /// Start a specific executable against a specific endpoint file.
    ///
    /// The timeouts are parameters so tests do not wait a minute to prove a timeout.
    ///
    /// # Errors
    ///
    /// As [`Engine::spawn`].
    pub fn spawn_with(
        binary: &str,
        endpoint_path: &Path,
        startup_timeout: Duration,
        warm_timeout: Duration,
    ) -> Result<Self, EngineError> {
        Endpoint::clear_at(endpoint_path)?;

        let mut child = Command::new(binary)
            .arg("serve")
            .arg("--port")
            .arg("0")
            .arg("--token-file")
            .arg(endpoint_path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| EngineError::NotInstalled(format!("{binary}: {e}")))?;

        let endpoint = match wait_for_endpoint(endpoint_path, startup_timeout) {
            Ok(endpoint) => endpoint,
            Err(e) => {
                let _ = child.kill();
                return Err(e);
            }
        };
        check_schema_version(endpoint.schema_version)?;

        let client = Client::new(endpoint);
        match wait_for_warm(&client, warm_timeout) {
            Ok(()) => Ok(Self {
                client,
                owned: true,
            }),
            Err(e) => {
                let _ = child.kill();
                Err(e)
            }
        }
    }

    /// Whether this process started the server.
    #[must_use]
    pub fn is_owned(&self) -> bool {
        self.owned
    }

    /// The underlying client.
    #[must_use]
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// `GET /health`.
    ///
    /// # Errors
    ///
    /// As [`Client::health`].
    pub fn health(&self) -> Result<Health, EngineError> {
        self.client.health()
    }

    /// `POST /predict`.
    ///
    /// # Errors
    ///
    /// As [`Client::predict`].
    pub fn predict(
        &self,
        texts: &[String],
        top_k: Option<u32>,
    ) -> Result<Vec<Prediction>, EngineError> {
        self.client.predict(texts, top_k)
    }

    /// Shut the server down, but only if this process started it.
    ///
    /// # Errors
    ///
    /// As [`Client::shutdown`]. A shutdown failure is reported rather than ignored,
    /// because a server left running holds a port and a few hundred megabytes.
    pub fn shutdown_if_owned(self) -> Result<(), EngineError> {
        if self.owned {
            self.client.shutdown()?;
            Endpoint::clear_at(&Endpoint::default_path())?;
        }
        Ok(())
    }
}

/// The `ttucs` executable to run, honouring the override variable.
#[must_use]
pub fn ttucs_binary() -> String {
    std::env::var(TTUCS_BIN_ENV).unwrap_or_else(|_| "ttucs".to_string())
}

/// Poll for the endpoint file until it appears or the budget runs out.
fn wait_for_endpoint(path: &Path, timeout: Duration) -> Result<Endpoint, EngineError> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(endpoint) = Endpoint::read_from(path)? {
            return Ok(endpoint);
        }
        if Instant::now() >= deadline {
            return Err(EngineError::StartupTimeout(timeout.as_secs()));
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Poll `/health` until the model reports itself warm.
fn wait_for_warm(client: &Client, timeout: Duration) -> Result<(), EngineError> {
    let deadline = Instant::now() + timeout;
    loop {
        match client.health() {
            Ok(health) if health.warm => return Ok(()),
            Ok(_) => {}
            Err(EngineError::Transport(_)) => {}
            Err(other) => return Err(other),
        }
        if Instant::now() >= deadline {
            return Err(EngineError::WarmTimeout(timeout.as_secs()));
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// The endpoint path a spawned server would use, for callers that need to clean up.
#[must_use]
pub fn default_endpoint_path() -> PathBuf {
    Endpoint::default_path()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binary_defaults_to_ttucs() {
        let _guard = crate::test_support::env_lock();
        let previous = std::env::var_os(TTUCS_BIN_ENV);
        std::env::remove_var(TTUCS_BIN_ENV);

        let binary = ttucs_binary();

        if let Some(value) = previous {
            std::env::set_var(TTUCS_BIN_ENV, value);
        }
        assert_eq!(binary, "ttucs");
    }

    #[test]
    fn binary_honours_the_override() {
        let _guard = crate::test_support::env_lock();
        let previous = std::env::var_os(TTUCS_BIN_ENV);
        std::env::set_var(TTUCS_BIN_ENV, "/opt/ttucs");

        let binary = ttucs_binary();

        match previous {
            Some(value) => std::env::set_var(TTUCS_BIN_ENV, value),
            None => std::env::remove_var(TTUCS_BIN_ENV),
        }
        assert_eq!(binary, "/opt/ttucs");
    }

    #[test]
    fn a_missing_binary_reports_not_installed() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("serve.json");

        let err = Engine::spawn_with(
            "definitely-not-a-real-binary-xyz",
            &path,
            Duration::from_millis(200),
            Duration::from_millis(200),
        )
        .expect_err("missing binary");

        assert!(matches!(err, EngineError::NotInstalled(_)), "got {err:?}");
    }

    #[test]
    fn a_binary_that_never_writes_the_file_times_out() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("serve.json");
        let sleeper = if cfg!(windows) { "cmd" } else { "sleep" };
        let args: &[&str] = if cfg!(windows) {
            &["/c", "exit 0"]
        } else {
            &["0"]
        };

        let mut command = Command::new(sleeper);
        command.args(args);
        let _ = command;

        let err = Engine::spawn_with(
            sleeper,
            &path,
            Duration::from_millis(300),
            Duration::from_millis(300),
        )
        .expect_err("no endpoint file");

        assert!(matches!(err, EngineError::StartupTimeout(_)), "got {err:?}");
    }

    #[test]
    fn connect_returns_none_when_no_endpoint_file_exists() {
        let _guard = crate::test_support::env_lock();
        let dir = tempfile::tempdir().expect("temp dir");
        let previous = std::env::var_os("TTUCS_HOME");
        std::env::set_var("TTUCS_HOME", dir.path());

        let engine = Engine::connect();

        match previous {
            Some(value) => std::env::set_var("TTUCS_HOME", value),
            None => std::env::remove_var("TTUCS_HOME"),
        }
        assert!(engine.expect("no error").is_none());
    }
}
