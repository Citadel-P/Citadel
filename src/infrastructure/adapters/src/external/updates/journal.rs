use super::files::{open_secure, read_bounded, unavailable_input};
use citadel_updates::{UpdateError, UpdateJournal};
use rustix::fs::{AtFlags, Mode, OFlags, RenameFlags, openat, renameat, renameat_with, unlinkat};
use serde::{Serialize, de::DeserializeOwned};
use std::os::unix::fs::MetadataExt;
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};
use uuid::Uuid;

const MAX_JOURNAL: u64 = 256 * 1024;

/// The caller retains this value throughout helper execution. Process exit drops
/// the OS lock, but the durable marker still requires identity reconciliation.
pub struct InstallationLock {
    _file: File,
}

impl Drop for InstallationLock {
    fn drop(&mut self) {
        // A concurrent fork can temporarily inherit this open description until
        // exec closes CLOEXEC descriptors. Release ownership explicitly instead
        // of waiting for every inherited descriptor to close.
        let _ = self._file.unlock();
    }
}

pub struct JournalDirectory {
    path: PathBuf,
    directory: File,
}

impl JournalDirectory {
    pub(super) fn sync(&self) -> Result<(), UpdateError> {
        self.directory.sync_all().map_err(|_| {
            UpdateError::Unavailable("Cannot durably synchronize update state.".into())
        })
    }
    pub(super) fn verify_location(&self) -> Result<(), UpdateError> {
        let actual = open_secure(&self.path, true)?
            .metadata()
            .map_err(|_| unavailable_input())?;
        let held = self.directory.metadata().map_err(|_| unavailable_input())?;
        if (actual.dev(), actual.ino()) != (held.dev(), held.ino())
            || actual.mode() & 0o077 != 0
            || actual.uid() != rustix::process::geteuid().as_raw()
        {
            return Err(UpdateError::Conflict(
                "Update state directory moved during registration.".into(),
            ));
        }
        Ok(())
    }

    pub(super) fn require_unregistered(&self) -> Result<(), UpdateError> {
        let entries =
            rustix::fs::Dir::read_from(&self.directory).map_err(|_| unavailable_input())?;
        for entry in entries {
            let entry = entry.map_err(|_| unavailable_input())?;
            if ![b".".as_slice(), b"..", b"installation.lock"]
                .contains(&entry.file_name().to_bytes())
            {
                return Err(UpdateError::Conflict(
                    "Update state already contains evidence; refusing a new registration.".into(),
                ));
            }
        }
        Ok(())
    }

    pub fn open(path: PathBuf) -> Result<Self, UpdateError> {
        let directory = open_secure(&path, true)?;
        let metadata = directory.metadata().map_err(|_| unavailable_input())?;
        if metadata.mode() & 0o077 != 0 || metadata.uid() != rustix::process::geteuid().as_raw() {
            return Err(UpdateError::Unavailable(
                "Update state must be private and owned by the executor (0700).".into(),
            ));
        }
        Ok(Self { path, directory })
    }

    pub fn lock(&self) -> Result<InstallationLock, UpdateError> {
        self.open_lock(true)
    }

    /// Startup observes an existing registration without creating or repairing
    /// evidence. The returned lock must not be held while awaiting a helper.
    pub(super) fn existing_lock(&self) -> Result<InstallationLock, UpdateError> {
        self.open_lock(false)
    }

    fn open_lock(&self, create: bool) -> Result<InstallationLock, UpdateError> {
        let access = if create {
            OFlags::RDWR | OFlags::CREATE
        } else {
            OFlags::RDONLY
        };
        let file = File::from(
            openat(
                &self.directory,
                "installation.lock",
                access | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
                Mode::from_raw_mode(0o600),
            )
            .map_err(|_| unavailable_input())?,
        );
        let metadata = file.metadata().map_err(|_| unavailable_input())?;
        if !metadata.is_file()
            || metadata.nlink() != 1
            || metadata.mode() & 0o077 != 0
            || metadata.uid() != rustix::process::geteuid().as_raw()
        {
            return Err(unavailable_input());
        }
        file.try_lock()
            .map_err(|_| UpdateError::Conflict("Another updater owns this installation.".into()))?;
        Ok(InstallationLock { _file: file })
    }

    pub(super) fn require_registration_only(&self) -> Result<(), UpdateError> {
        let entries =
            rustix::fs::Dir::read_from(&self.directory).map_err(|_| unavailable_input())?;
        for entry in entries {
            let entry = entry.map_err(|_| unavailable_input())?;
            if ![
                b".".as_slice(),
                b"..",
                b"installation.lock",
                b"installation.json",
            ]
            .contains(&entry.file_name().to_bytes())
            {
                return Err(UpdateError::Conflict(
                    "Update execution evidence requires recovery verification before Core can start.".into(),
                ));
            }
        }
        Ok(())
    }

    pub fn read<T: DeserializeOwned>(&self, name: &str) -> Result<T, UpdateError> {
        self.read_optional(name)?.ok_or_else(unavailable_input)
    }

    pub fn read_optional<T: DeserializeOwned>(&self, name: &str) -> Result<Option<T>, UpdateError> {
        self.entry(name)?;
        let file = match openat(
            &self.directory,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
        ) {
            Ok(file) => File::from(file),
            Err(rustix::io::Errno::NOENT) => return Ok(None),
            Err(_) => return Err(unavailable_input()),
        };
        let metadata = file.metadata().map_err(|_| unavailable_input())?;
        if metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.mode() & 0o077 != 0
            || metadata.nlink() != 1
        {
            return Err(unavailable_input());
        }
        let bytes = read_bounded(file, MAX_JOURNAL)?;
        serde_json::from_slice(&bytes).map(Some).map_err(|_| {
            UpdateError::Conflict("Update state is corrupt or uses an unsupported format.".into())
        })
    }

    pub fn write<T: Serialize>(&self, name: &str, value: &T) -> Result<(), UpdateError> {
        self.write_entry(name, value, false)
    }

    /// Atomically publish once. Existing state, including an unsafe entry, is
    /// never overwritten; a caller can reconcile a durable success after retry.
    pub fn create<T: Serialize>(&self, name: &str, value: &T) -> Result<(), UpdateError> {
        self.write_entry(name, value, true)
    }

    fn write_entry<T: Serialize>(
        &self,
        name: &str,
        value: &T,
        exclusive: bool,
    ) -> Result<(), UpdateError> {
        self.entry(name)?;
        let bytes = serde_json::to_vec(value)
            .map_err(|_| UpdateError::Validation("Cannot encode update state.".into()))?;
        if bytes.len() as u64 > MAX_JOURNAL {
            return Err(UpdateError::Validation(
                "Update state exceeds its size limit.".into(),
            ));
        }
        let temporary = format!(".{}.tmp", Uuid::now_v7());
        let result = (|| {
            let mut file = File::from(openat(
                &self.directory,
                temporary.as_str(),
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::from_raw_mode(0o600),
            )?);
            file.write_all(&bytes)?;
            file.sync_all()?;
            if exclusive {
                renameat_with(
                    &self.directory,
                    temporary.as_str(),
                    &self.directory,
                    name,
                    RenameFlags::NOREPLACE,
                )?;
            } else {
                renameat(&self.directory, temporary.as_str(), &self.directory, name)?;
            }
            self.directory.sync_all()
        })();
        if result.is_err() {
            let _ = unlinkat(&self.directory, temporary.as_str(), AtFlags::empty());
        }
        result.map_err(|_: std::io::Error| {
            UpdateError::Unavailable("Cannot durably write update state.".into())
        })
    }

    pub fn journal(&self, id: Uuid) -> Result<UpdateJournal, UpdateError> {
        self.read(&format!("{id}.json"))
    }

    pub fn save(&self, journal: &UpdateJournal) -> Result<(), UpdateError> {
        self.write(&format!("{}.json", journal.operation.id), journal)
    }

    fn entry(&self, name: &str) -> Result<PathBuf, UpdateError> {
        if name.is_empty()
            || name.len() > 100
            || !name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".-".contains(&b))
            || name.starts_with('.')
        {
            return Err(UpdateError::Validation("Invalid update state name.".into()));
        }
        Ok(self.path.join(name))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn dropping_the_owner_releases_the_lock_even_with_an_inherited_descriptor() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::set_permissions(directory.path(), std::fs::Permissions::from_mode(0o700)).unwrap();
        let state = JournalDirectory::open(directory.path().into()).unwrap();
        let owner = state.lock().unwrap();
        let inherited = owner._file.try_clone().unwrap();
        assert!(state.lock().is_err());
        drop(owner);
        let next = state.lock().unwrap();
        drop(inherited);
        assert!(state.lock().is_err());
        drop(next);
        state.lock().unwrap();
    }
}
