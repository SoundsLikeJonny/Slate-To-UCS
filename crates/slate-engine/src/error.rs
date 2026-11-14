//! Errors raised by the engine client and launcher.

use thiserror::Error;

/// Everything that can go wrong talking to, or starting, `ttucs serve`.
///
/// Each variant maps to one message the UI can display verbatim, so a user is never
/// shown a raw transport error without an action to take.
#[derive(Debug, Error)]
pub enum EngineError {
    /// The `ttucs` executable could not be found or started.
    #[error("ttucs is not installed or could not be started: {0}")]
    NotInstalled(String),

    /// The server did not write its endpoint file within the startup budget.
    #[error("ttucs serve did not start within {0} seconds")]
    StartupTimeout(u64),

    /// The server started but never reported itself warm.
    #[error("ttucs serve did not become warm within {0} seconds")]
    WarmTimeout(u64),

    /// The health probe failed, so the server is not usable.
    #[error("ttucs serve is not healthy: {0}")]
    Unhealthy(String),

    /// The token was missing or rejected.
    #[error("ttucs serve rejected the token")]
    Unauthorised,

    /// The engine speaks a contract version this client does not understand.
    #[error("ttucs serve speaks schema_version {found}, this client supports {expected}")]
    SchemaMismatch {
        /// The version the server reported.
        found: u32,
        /// The version this client implements.
        expected: u32,
    },

    /// A request exceeded a client-side cap before it was sent.
    #[error("{0}")]
    RequestTooLarge(String),

    /// The endpoint file could not be read, written or parsed.
    #[error("endpoint file {path}: {reason}")]
    EndpointFile {
        /// The path involved.
        path: String,
        /// What went wrong.
        reason: String,
    },

    /// The server answered with an unexpected status.
    #[error("ttucs serve returned {status} for {path}: {body}")]
    Status {
        /// HTTP status code.
        status: u16,
        /// Request path.
        path: String,
        /// Response body, truncated for display.
        body: String,
    },

    /// The response body was not the JSON the contract promises.
    #[error("malformed response from ttucs serve: {0}")]
    Malformed(String),

    /// A socket-level failure.
    #[error("transport error: {0}")]
    Transport(String),
}
