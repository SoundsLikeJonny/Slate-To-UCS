//! One instance per user, enforced with an OS file lock.

use std::fs::{File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};

/// An exclusive claim on the single-instance lock file.
///
/// The lock is an advisory OS lock on an open file handle, not a pid file. The
/// kernel drops it when the process dies, so a crash or a `kill -9` cannot leave a
/// stale lock that blocks the next launch. The file itself is left in place: deleting
/// it on drop would race with a second process that has already opened it.
#[derive(Debug)]
pub struct InstanceGuard {
    path: PathBuf,
    file: File,
    held: bool,
}

impl InstanceGuard {
    /// Try to become the only instance.
    ///
    /// Returns `Ok(Some(guard))` when this process holds the lock, `Ok(None)` when
    /// another live process holds it, and `Err` only for a real I/O failure such as a
    /// missing parent directory or a permission denial.
    ///
    /// # Errors
    ///
    /// Returns the underlying [`io::Error`] when the lock file cannot be created or
    /// opened.
    pub fn acquire(path: impl AsRef<Path>) -> Result<Option<Self>, io::Error> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .open(&path)?;
        match file.try_lock() {
            Ok(()) => Ok(Some(Self {
                path,
                file,
                held: true,
            })),
            Err(std::fs::TryLockError::WouldBlock) => Ok(None),
            Err(std::fs::TryLockError::Error(e)) => Err(e),
        }
    }

    /// The lock file this guard holds.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Whether this guard still holds the lock.
    ///
    /// This reads a flag rather than re-locking the handle. Probing with `try_lock` is
    /// not portable: `flock` on a descriptor that already holds the lock succeeds, so on
    /// Unix a live guard would report itself as unheld. The flag is set on acquire and
    /// cleared by [`InstanceGuard::release`], which is the only way the lock is dropped.
    #[must_use]
    pub fn is_held(&self) -> bool {
        self.held
    }

    /// Drop the lock and mark the guard released.
    fn release(&mut self) {
        self.held = false;
        let _ = self.file.unlock();
    }
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_first_acquire_wins() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("instance.lock");

        let guard = InstanceGuard::acquire(&path)
            .expect("acquire")
            .expect("held");

        assert_eq!(guard.path(), path);
        assert!(guard.is_held());
        assert!(path.is_file());
    }

    #[test]
    fn releasing_clears_the_held_flag() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("instance.lock");
        let mut guard = InstanceGuard::acquire(&path)
            .expect("acquire")
            .expect("held");

        guard.release();

        assert!(!guard.is_held(), "a released guard must not claim the lock");
    }

    #[test]
    fn releasing_twice_is_harmless() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("instance.lock");
        let mut guard = InstanceGuard::acquire(&path)
            .expect("acquire")
            .expect("held");

        guard.release();
        guard.release();

        assert!(!guard.is_held());
    }

    #[test]
    fn a_second_acquire_is_refused_while_the_first_is_alive() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("instance.lock");
        let _first = InstanceGuard::acquire(&path)
            .expect("acquire")
            .expect("held");

        let second = InstanceGuard::acquire(&path).expect("acquire");

        assert!(second.is_none(), "a second instance must not start");
    }

    #[test]
    fn dropping_the_guard_releases_the_lock() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("instance.lock");
        let first = InstanceGuard::acquire(&path)
            .expect("acquire")
            .expect("held");
        drop(first);

        let second = InstanceGuard::acquire(&path).expect("acquire");

        assert!(second.is_some(), "the lock must be reusable after drop");
    }

    #[test]
    fn a_stale_file_does_not_block_a_new_instance() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("instance.lock");
        std::fs::write(&path, b"12345").expect("stale file");

        let guard = InstanceGuard::acquire(&path).expect("acquire");

        assert!(guard.is_some(), "a leftover file is not a live lock");
    }

    #[test]
    fn the_parent_directory_is_created() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir
            .path()
            .join("nested")
            .join("deeper")
            .join("instance.lock");

        let guard = InstanceGuard::acquire(&path).expect("acquire");

        assert!(guard.is_some());
        assert!(path.is_file());
    }

    #[test]
    fn an_unwritable_location_is_an_error_not_a_refusal() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("instance.lock");
        std::fs::create_dir(&path).expect("directory in the way");

        let err = InstanceGuard::acquire(&path).expect_err("cannot open a directory");

        // Windows reports a denied open, Unix reports EISDIR; both are real errors and
        // neither may be mistaken for "another instance is running".
        assert!(
            matches!(
                err.kind(),
                io::ErrorKind::IsADirectory | io::ErrorKind::PermissionDenied
            ),
            "unexpected kind: {:?}",
            err.kind()
        );
    }
}
