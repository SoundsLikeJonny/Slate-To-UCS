//! Endpoint discovery through `$TTUCS_HOME/serve.json`.

use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::EngineError;

/// The endpoint file the engine writes on startup.
///
/// The file is mode `0600` and carries the port and token. It is deleted on a clean
/// shutdown, so a file that is present but unreachable means the server died.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct Endpoint {
    /// Contract version the server speaks.
    pub schema_version: u32,
    /// Bind host, always loopback.
    pub host: String,
    /// Bound port.
    pub port: u16,
    /// Bearer token for every route except `/health`.
    pub token: String,
    /// Server process id.
    pub pid: u32,
    /// Start time, ISO 8601.
    pub started: String,
    /// Version of the `ttucs` package serving.
    pub ttucs_version: String,
}

impl Endpoint {
    /// `http://host:port`, for display and for error messages.
    #[must_use]
    pub fn base_url(&self) -> String {
        format!("http://{}:{}", self.host, self.port)
    }

    /// The token, redacted for display.
    #[must_use]
    pub fn redacted_token(&self) -> &'static str {
        "<redacted>"
    }

    /// The default endpoint file path: `$TTUCS_HOME/serve.json`, falling back to the
    /// platform cache directory the same way `ttucs.hub` does.
    #[must_use]
    pub fn default_path() -> PathBuf {
        if let Some(home) = std::env::var_os("TTUCS_HOME") {
            return PathBuf::from(home).join("serve.json");
        }
        cache_root().join("serve.json")
    }

    /// Read the endpoint file at the default path.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::EndpointFile`] when the file exists but cannot be read
    /// or parsed. A missing file is `Ok(None)`, not an error.
    pub fn read() -> Result<Option<Self>, EngineError> {
        Self::read_from(&Self::default_path())
    }

    /// Read an endpoint file at an explicit path.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::EndpointFile`] when the file exists but cannot be read
    /// or parsed.
    pub fn read_from(path: &Path) -> Result<Option<Self>, EngineError> {
        match std::fs::read_to_string(path) {
            Ok(text) => {
                serde_json::from_str(&text)
                    .map(Some)
                    .map_err(|e| EngineError::EndpointFile {
                        path: path.display().to_string(),
                        reason: e.to_string(),
                    })
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(EngineError::EndpointFile {
                path: path.display().to_string(),
                reason: e.to_string(),
            }),
        }
    }

    /// Write the endpoint file, creating the parent directory.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::EndpointFile`] when the directory or file cannot be
    /// written.
    pub fn write_to(&self, path: &Path) -> Result<(), EngineError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| EngineError::EndpointFile {
                path: parent.display().to_string(),
                reason: e.to_string(),
            })?;
        }
        let text = serde_json::to_string_pretty(self).map_err(|e| EngineError::EndpointFile {
            path: path.display().to_string(),
            reason: e.to_string(),
        })?;
        std::fs::write(path, text).map_err(|e| EngineError::EndpointFile {
            path: path.display().to_string(),
            reason: e.to_string(),
        })
    }

    /// Remove the endpoint file, ignoring a missing one.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::EndpointFile`] when the file exists but cannot be
    /// removed.
    pub fn clear_at(path: &Path) -> Result<(), EngineError> {
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(EngineError::EndpointFile {
                path: path.display().to_string(),
                reason: e.to_string(),
            }),
        }
    }

    /// Whether a TCP connection to the endpoint succeeds.
    ///
    /// Process liveness by pid is deliberately not checked: it needs platform APIs,
    /// and a dead server fails this probe anyway, which is the decision that matters.
    #[must_use]
    pub fn is_reachable(&self, timeout: Duration) -> bool {
        let Ok(mut addrs) = (self.host.as_str(), self.port).to_socket_addrs() else {
            return false;
        };
        let Some(address) = addrs.next() else {
            return false;
        };
        TcpStream::connect_timeout(&address, timeout).is_ok()
    }
}

/// The platform cache root, matching `ttucs.hub.cache_root()`.
fn cache_root() -> PathBuf {
    if let Some(xdg) = std::env::var_os("XDG_CACHE_HOME") {
        return PathBuf::from(xdg).join("ttucs");
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        return PathBuf::from(local).join("ttucs").join("cache");
    }
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join(".cache").join("ttucs");
    }
    std::env::temp_dir().join("ttucs")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Endpoint {
        Endpoint {
            schema_version: 1,
            host: "127.0.0.1".to_string(),
            port: 8731,
            token: "secret-token".to_string(),
            pid: 4242,
            started: "2026-11-14T00:00:00Z".to_string(),
            ttucs_version: "0.3.0".to_string(),
        }
    }

    #[test]
    fn base_url_joins_host_and_port() {
        assert_eq!(sample().base_url(), "http://127.0.0.1:8731");
    }

    #[test]
    fn token_is_redacted_for_display() {
        assert_eq!(sample().redacted_token(), "<redacted>");
    }

    #[test]
    fn round_trips_through_a_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("serve.json");

        sample().write_to(&path).expect("written");
        let read = Endpoint::read_from(&path).expect("read").expect("present");

        assert_eq!(read, sample());
    }

    #[test]
    fn creates_the_parent_directory() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("nested").join("deeper").join("serve.json");

        sample().write_to(&path).expect("written");

        assert!(path.is_file());
    }

    #[test]
    fn a_missing_file_is_not_an_error() {
        let dir = tempfile::tempdir().expect("temp dir");

        let read = Endpoint::read_from(&dir.path().join("absent.json")).expect("ok");

        assert!(read.is_none());
    }

    #[test]
    fn a_malformed_file_is_an_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("serve.json");
        std::fs::write(&path, "{not json").expect("written");

        let err = Endpoint::read_from(&path).expect_err("malformed");

        assert!(
            matches!(err, EngineError::EndpointFile { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn clear_removes_the_file_and_tolerates_absence() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("serve.json");
        sample().write_to(&path).expect("written");

        Endpoint::clear_at(&path).expect("cleared");
        Endpoint::clear_at(&path).expect("clearing twice is fine");

        assert!(!path.exists());
    }

    #[test]
    fn an_unreachable_endpoint_is_not_reachable() {
        let endpoint = Endpoint {
            port: 1,
            ..sample()
        };

        assert!(!endpoint.is_reachable(Duration::from_millis(200)));
    }

    #[test]
    fn default_path_honours_ttucs_home() {
        let _guard = crate::test_support::env_lock();
        let dir = tempfile::tempdir().expect("temp dir");
        let previous = std::env::var_os("TTUCS_HOME");
        std::env::set_var("TTUCS_HOME", dir.path());

        let path = Endpoint::default_path();

        match previous {
            Some(value) => std::env::set_var("TTUCS_HOME", value),
            None => std::env::remove_var("TTUCS_HOME"),
        }
        assert_eq!(path, dir.path().join("serve.json"));
    }
}
