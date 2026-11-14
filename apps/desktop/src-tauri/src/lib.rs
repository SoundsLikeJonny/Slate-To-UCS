//! The Slate window: one instance, one sidecar, one IPC surface.
//!
//! Everything that can be tested without a display lives in `slate-shell` and
//! `slate-engine`. This crate wires those to a window and exposes four commands to the
//! front end. The engine token never crosses the IPC boundary: the front end asks for
//! health and predictions, and Rust holds the client.

#![deny(missing_docs)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

use std::sync::Mutex;

use slate_engine::{Engine, EngineError, Health, Prediction};
use slate_shell::{InstanceGuard, Sidecar};
use tauri::{Manager, State};

/// The engine handle the window shares between commands.
///
/// `None` until the sidecar is up, so a command that arrives before startup reports a
/// clear "not ready" instead of blocking the UI thread.
#[derive(Default)]
struct Shell {
    engine: Mutex<Option<Engine>>,
}

/// The error shape the front end receives.
#[derive(Debug, serde::Serialize)]
struct CommandError {
    kind: &'static str,
    message: String,
}

impl From<EngineError> for CommandError {
    fn from(error: EngineError) -> Self {
        Self {
            kind: error.kind(),
            message: error.to_string(),
        }
    }
}

/// `GET /health` through the shell's client.
#[tauri::command]
fn health(shell: State<'_, Shell>) -> Result<Health, CommandError> {
    let guard = shell
        .engine
        .lock()
        .map_err(|_| CommandError {
            kind: "internal",
            message: "the engine lock is poisoned".to_string(),
        })?;
    let engine = guard.as_ref().ok_or(CommandError {
        kind: "not_ready",
        message: "the engine is still starting".to_string(),
    })?;
    engine.health().map_err(Into::into)
}

/// `POST /predict` through the shell's client.
#[tauri::command]
fn predict(
    shell: State<'_, Shell>,
    texts: Vec<String>,
    top_k: Option<u32>,
) -> Result<Vec<Prediction>, CommandError> {
    let guard = shell
        .engine
        .lock()
        .map_err(|_| CommandError {
            kind: "internal",
            message: "the engine lock is poisoned".to_string(),
        })?;
    let engine = guard.as_ref().ok_or(CommandError {
        kind: "not_ready",
        message: "the engine is still starting".to_string(),
    })?;
    engine.predict(&texts, top_k).map_err(Into::into)
}

/// Start the sidecar and store the handle.
///
/// Called from `setup` rather than from the front end so a slow first run cannot be
/// interrupted by a page reload.
fn start_engine(shell: &Shell, sidecar: &Sidecar, endpoint: std::path::PathBuf) {
    match sidecar.spawn(endpoint) {
        Ok(engine) => {
            if let Ok(mut guard) = shell.engine.lock() {
                *guard = Some(engine);
            }
        }
        Err(error) => {
            eprintln!("slate: the engine did not start: {error}");
        }
    }
}

/// Build and run the window.
///
/// # Panics
///
/// Panics when Tauri cannot create the window, which is unrecoverable for a GUI app.
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let lock = app
                .path()
                .app_config_dir()
                .map(|dir| dir.join("instance.lock"))
                .unwrap_or_else(|_| std::path::PathBuf::from("instance.lock"));
            match InstanceGuard::acquire(&lock) {
                Ok(Some(guard)) => {
                    app.manage(guard);
                }
                Ok(None) => {
                    eprintln!("slate: another window is already open");
                    std::process::exit(0);
                }
                Err(error) => eprintln!("slate: the instance lock failed: {error}"),
            }

            let shell = Shell::default();
            let resource = app
                .path()
                .resource_dir()
                .map(|dir| dir.join("resources").join("ttucs"))
                .unwrap_or_else(|_| slate_shell::default_root());
            let endpoint = slate_engine::default_endpoint_path();
            start_engine(&shell, &Sidecar::new(resource), endpoint);
            app.manage(shell);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![health, predict])
        .run(tauri::generate_context!())
        .expect("the Slate window failed to start");
}
