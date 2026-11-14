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

impl EngineError {
    /// A stable, machine-readable name for this failure.
    ///
    /// The desktop front end branches on this string, so it is part of the IPC
    /// contract: renaming a variant must not silently rename a kind. The values match
    /// `apps/desktop/src/lib/engine.ts`.
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::NotInstalled(_) => "not_installed",
            Self::StartupTimeout(_) => "startup_timeout",
            Self::WarmTimeout(_) => "warm_timeout",
            Self::Unhealthy(_) => "unhealthy",
            Self::Unauthorised => "unauthorised",
            Self::SchemaMismatch { .. } => "schema_mismatch",
            Self::RequestTooLarge(_) => "request_too_large",
            Self::EndpointFile { .. } => "endpoint_file",
            Self::Status { .. } => "status",
            Self::Malformed(_) => "malformed",
            Self::Transport(_) => "transport",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_variant_has_a_distinct_kind() {
        let variants = [
            EngineError::NotInstalled("x".to_string()),
            EngineError::StartupTimeout(1),
            EngineError::WarmTimeout(1),
            EngineError::Unhealthy("x".to_string()),
            EngineError::Unauthorised,
            EngineError::SchemaMismatch {
                found: 2,
                expected: 1,
            },
            EngineError::RequestTooLarge("x".to_string()),
            EngineError::EndpointFile {
                path: "p".to_string(),
                reason: "r".to_string(),
            },
            EngineError::Status {
                status: 500,
                path: "/predict".to_string(),
                body: "x".to_string(),
            },
            EngineError::Malformed("x".to_string()),
            EngineError::Transport("x".to_string()),
        ];

        let mut kinds: Vec<&str> = variants.iter().map(EngineError::kind).collect();
        let total = kinds.len();
        kinds.sort_unstable();
        kinds.dedup();

        assert_eq!(kinds.len(), total, "two variants share a kind: {kinds:?}");
    }

    #[test]
    fn kinds_are_lower_snake_case() {
        let kinds = [
            EngineError::NotInstalled(String::new()).kind(),
            EngineError::Unauthorised.kind(),
            EngineError::SchemaMismatch {
                found: 1,
                expected: 1,
            }
            .kind(),
        ];

        for kind in kinds {
            assert!(
                kind.chars().all(|c| c.is_ascii_lowercase() || c == '_'),
                "{kind} is not lower snake case"
            );
        }
    }

    #[test]
    fn the_kind_is_stable_for_the_front_end() {
        assert_eq!(EngineError::Unauthorised.kind(), "unauthorised");
        assert_eq!(
            EngineError::SchemaMismatch {
                found: 2,
                expected: 1
            }
            .kind(),
            "schema_mismatch"
        );
    }
}
