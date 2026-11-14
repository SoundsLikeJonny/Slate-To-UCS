//! A child process that can be cancelled without leaving an orphan.

use std::ffi::OsString;
use std::path::Path;

/// A speech-to-text process that can be cancelled.
///
/// The CLI runs Whisper synchronously, but the desktop shell must be able to stop a
/// transcription when the owner cancels. Dropping this value kills and reaps the child,
/// so a cancelled or panicking caller cannot leave an orphan holding a temp file.
#[derive(Debug)]
pub struct Cancellable {
    child: std::process::Child,
}

impl Cancellable {
    /// Start a process with no inherited standard streams.
    ///
    /// # Errors
    ///
    /// As [`std::process::Command::spawn`].
    pub fn spawn(binary: &Path, arguments: &[OsString]) -> std::io::Result<Self> {
        let child = std::process::Command::new(binary)
            .args(arguments)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        Ok(Self { child })
    }

    /// The operating-system process identifier.
    #[must_use]
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    /// Kill the process and reap it.
    ///
    /// Killing an already-exited process is not an error, so a cancel that races with a
    /// natural exit still returns the real status.
    ///
    /// # Errors
    ///
    /// As [`std::process::Child::kill`] and [`std::process::Child::wait`].
    pub fn cancel(&mut self) -> std::io::Result<std::process::ExitStatus> {
        let _ = self.child.kill();
        self.child.wait()
    }

    /// Wait for the process to exit on its own.
    ///
    /// # Errors
    ///
    /// As [`std::process::Child::wait`].
    pub fn wait(&mut self) -> std::io::Result<std::process::ExitStatus> {
        self.child.wait()
    }

    /// Whether the process has already exited, without blocking.
    ///
    /// # Errors
    ///
    /// As [`std::process::Child::try_wait`].
    pub fn try_wait(&mut self) -> std::io::Result<Option<std::process::ExitStatus>> {
        self.child.try_wait()
    }
}

impl Drop for Cancellable {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A command that runs far longer than any test, so a cancel is observable.
    fn long_running() -> (std::path::PathBuf, Vec<OsString>) {
        #[cfg(windows)]
        {
            (
                std::path::PathBuf::from("cmd"),
                vec![
                    OsString::from("/C"),
                    OsString::from("ping -n 60 127.0.0.1 > NUL"),
                ],
            )
        }
        #[cfg(not(windows))]
        {
            (
                std::path::PathBuf::from("sleep"),
                vec![OsString::from("60")],
            )
        }
    }

    #[test]
    fn cancel_stops_a_running_process() {
        let (binary, arguments) = long_running();
        let mut child = Cancellable::spawn(&binary, &arguments).expect("spawned");

        let status = child.cancel().expect("cancelled");

        assert!(
            !status.success(),
            "a cancelled process must not report success"
        );
        assert_eq!(
            child.try_wait().expect("try_wait"),
            Some(status),
            "the process must be reaped, not merely signalled"
        );
    }

    #[test]
    fn cancel_is_idempotent_after_the_process_is_gone() {
        let (binary, arguments) = long_running();
        let mut child = Cancellable::spawn(&binary, &arguments).expect("spawned");
        let first = child.cancel().expect("cancelled");

        let second = child.cancel().expect("cancelled twice");

        assert_eq!(first, second);
    }

    #[test]
    fn dropping_the_handle_kills_the_process() {
        let dir = tempfile::tempdir().expect("temp dir");
        let marker = dir.path().join("finished.txt");
        let script = dir
            .path()
            .join(if cfg!(windows) { "slow.cmd" } else { "slow.sh" });
        #[cfg(windows)]
        std::fs::write(
            &script,
            format!(
                "@echo off\r\nping -n 3 127.0.0.1 > NUL\r\necho done > \"{}\"\r\n",
                marker.display()
            ),
        )
        .expect("write script");
        #[cfg(not(windows))]
        std::fs::write(
            &script,
            format!("#!/bin/sh\nsleep 2\nprintf done > '{}'\n", marker.display()),
        )
        .expect("write script");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&script).expect("stat").permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script, perms).expect("chmod");
        }

        {
            let child = Cancellable::spawn(&script, &[]).expect("spawned");
            assert!(child.id() > 0);
        }

        std::thread::sleep(std::time::Duration::from_millis(3000));

        assert!(
            !marker.exists(),
            "a dropped handle must kill the child before it can finish"
        );
    }

    #[test]
    fn a_missing_binary_is_an_io_error() {
        let error = Cancellable::spawn(Path::new("definitely-not-a-real-binary-xyz"), &[])
            .expect_err("missing binary");

        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn wait_returns_the_natural_exit_status() {
        #[cfg(windows)]
        let (binary, arguments) = (
            std::path::PathBuf::from("cmd"),
            vec![OsString::from("/C"), OsString::from("exit 0")],
        );
        #[cfg(not(windows))]
        let (binary, arguments) = (
            std::path::PathBuf::from("sh"),
            vec![OsString::from("-c"), OsString::from("exit 0")],
        );
        let mut child = Cancellable::spawn(&binary, &arguments).expect("spawned");

        let status = child.wait().expect("waited");

        assert!(status.success());
    }
}
