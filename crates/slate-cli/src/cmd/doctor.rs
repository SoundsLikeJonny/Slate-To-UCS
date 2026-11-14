//! `slate doctor`: report what is installed and what is reachable.
//!
//! This is what the install smoke test runs on a clean machine, so every line is
//! either a fact or a clear "not found" — never a silent omission.

use slate_engine::{Endpoint, Engine};

use crate::EXIT_ENGINE;

/// Run `slate doctor` and return the process exit code.
pub fn run() -> u8 {
    let mut healthy = true;

    println!("slate {}", env!("CARGO_PKG_VERSION"));

    match Endpoint::read() {
        Ok(Some(endpoint)) => {
            println!(
                "endpoint      {} (pid {})",
                endpoint.base_url(),
                endpoint.pid
            );
            match Engine::connect() {
                Ok(Some(engine)) => match engine.health() {
                    Ok(health) => {
                        println!("engine        ok, warm={}", health.warm);
                        println!("ttucs         {}", health.ttucs_version);
                        println!("ucs           {}", health.ucs_version);
                        println!(
                            "model         {} {} (source {}, manifest_verified {})",
                            health.model.name,
                            health.model.version,
                            health.model.source,
                            health.model.manifest_verified
                        );
                        println!("offline       {}", health.offline);
                        println!("device        {}", health.device);
                    }
                    Err(error) => {
                        println!("engine        unhealthy: {error}");
                        healthy = false;
                    }
                },
                Ok(None) => {
                    println!("engine        endpoint file present but unreachable");
                    healthy = false;
                }
                Err(error) => {
                    println!("engine        {error}");
                    healthy = false;
                }
            }
        }
        Ok(None) => {
            println!("endpoint      not running");
            println!("engine        start it with `ttucs serve`");
            healthy = false;
        }
        Err(error) => {
            println!("endpoint      {error}");
            healthy = false;
        }
    }

    println!("reaper        {}", reaper_status());

    if healthy {
        0
    } else {
        EXIT_ENGINE
    }
}

/// Describe the REAPER integration state without touching the filesystem beyond a
/// directory check.
fn reaper_status() -> String {
    let Some(resources) = reaper_resource_dir() else {
        return "resource directory not found".to_string();
    };
    let plugins = resources.join("UserPlugins");
    if plugins.is_dir() {
        format!("resources at {}", resources.display())
    } else {
        format!(
            "resources at {}, no UserPlugins directory",
            resources.display()
        )
    }
}

/// The REAPER resource directory for the current user, if it exists.
fn reaper_resource_dir() -> Option<std::path::PathBuf> {
    let candidate = if cfg!(windows) {
        std::env::var_os("APPDATA").map(|base| std::path::PathBuf::from(base).join("REAPER"))
    } else if cfg!(target_os = "macos") {
        std::env::var_os("HOME")
            .map(|home| std::path::PathBuf::from(home).join("Library/Application Support/REAPER"))
    } else {
        std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".config/REAPER"))
    };
    candidate.filter(|path| path.is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EXIT_USER;

    #[test]
    fn reaper_status_is_never_empty() {
        assert!(!reaper_status().is_empty());
    }

    #[test]
    fn exit_codes_are_the_documented_values() {
        assert_eq!(EXIT_ENGINE, 2);
        assert_eq!(EXIT_USER, 1);
    }
}
