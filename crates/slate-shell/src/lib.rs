//! Desktop shell logic that does not need a window.
//!
//! The Tauri binary in `apps/desktop/src-tauri` is deliberately thin: it owns the
//! window, the menu and the IPC surface, and calls into this crate for everything
//! that can be tested without a display. That split is why this crate is a workspace
//! member and the Tauri binary is not — `cargo test --workspace` stays fast and needs
//! no WebView.
//!
//! Two pieces live here:
//!
//! - [`InstanceGuard`]: one window per user, enforced with an OS file lock rather than
//!   a pid file, so a crashed process cannot leave a stale lock behind.
//! - [`Sidecar`]: where the bundled `ttucs` tree lives and how to start it offline.
//!
//! ```
//! use slate_shell::Sidecar;
//!
//! let sidecar = Sidecar::new("C:/Program Files/Slate/resources/ttucs");
//! assert!(sidecar.binary().ends_with("ttucs.exe") || sidecar.binary().ends_with("ttucs"));
//! ```

#![deny(missing_docs)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

mod instance;
mod sidecar;

pub use instance::InstanceGuard;
pub use sidecar::{default_root, default_timeouts, Sidecar, OFFLINE_ENV};
