//! Launcher contract, endpoint discovery and typed client for `ttucs serve`.
//!
//! This crate is the reference implementation of the three launcher steps in
//! `docs/PLAN.md` §5.4. The C++ REAPER shim implements the same steps; a change here
//! is a change there.
//!
//! 1. Read `$TTUCS_HOME/serve.json`. If it is present and `/health` answers with the
//!    token, use that server.
//! 2. Otherwise spawn `ttucs serve --port 0 --token-file <path>` detached, poll the
//!    file for up to 30 s, then poll `/health` until `warm` within a 60 s budget.
//! 3. On exit, call `/shutdown` only if this process started the server.
//!
//! ```
//! use slate_engine::{Endpoint, SCHEMA_VERSION};
//!
//! let endpoint = Endpoint {
//!     schema_version: SCHEMA_VERSION,
//!     host: "127.0.0.1".to_string(),
//!     port: 8731,
//!     token: "secret".to_string(),
//!     pid: 42,
//!     started: "2026-11-14T00:00:00Z".to_string(),
//!     ttucs_version: "0.3.0".to_string(),
//! };
//!
//! assert_eq!(endpoint.base_url(), "http://127.0.0.1:8731");
//! assert_eq!(endpoint.redacted_token(), "<redacted>");
//! ```

#![deny(missing_docs)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod client;
mod endpoint;
mod error;
mod http;
mod launcher;
mod schema;
mod token;
mod types;

pub use client::{Client, DEFAULT_TIMEOUT};
pub use endpoint::Endpoint;
pub use error::EngineError;
pub use launcher::{
    default_endpoint_path, ttucs_binary, Engine, POLL_INTERVAL, STARTUP_TIMEOUT, TTUCS_BIN_ENV,
    WARM_TIMEOUT,
};
pub use schema::{check_schema_version, SCHEMA_VERSION};
pub use token::Token;
pub use types::{
    Alternative, CategoryScore, DecisionKind, Health, Limits, ModelInfo, Prediction, SlateFields,
    TaxonomyEntry,
};

/// Maximum number of texts accepted in one request, matching the engine's cap.
pub const MAX_TEXTS: usize = 512;

/// Maximum request body in bytes, matching the engine's cap.
pub const MAX_BODY_BYTES: usize = 1024 * 1024;

#[cfg(test)]
pub(crate) mod test_support {
    //! Helpers shared by the unit tests in this crate.

    use std::sync::{Mutex, MutexGuard, OnceLock};

    /// Serialise tests that read or write process environment variables.
    ///
    /// The test harness runs tests in parallel threads, so two tests that both set
    /// `TTUCS_HOME` would otherwise observe each other's value.
    pub(crate) fn env_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}
