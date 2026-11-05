//! Publish an already reviewed image selector without rewriting other installer inputs.
use super::super::{
    files::{open_secure, unavailable_input},
    registration::fingerprint_inputs,
};
use super::{CheckedCoreReplacement, InstallerComposeInputs};
use citadel_updates::UpdateError;
use rustix::fs::{AtFlags, Mode, OFlags, openat, renameat, unlinkat};
use std::{fs::File, io::Write, os::unix::fs::MetadataExt};
use tokio_util::sync::CancellationToken;
use zeroize::{Zeroize, Zeroizing};

pub(in crate::external::updates) struct PreparedCoreReplacement {
    pub checked: CheckedCoreReplacement,
    pub(super) environment: Zeroizing<Vec<u8>>,
}

impl PreparedCoreReplacement {
    /// Caller must own the installation lock and persist replacement intent first.
    /// A failure after rename is ambiguous; never retry or restore the old selector.
    pub(in crate::external::updates) fn publish(
        self,
        inputs: &InstallerComposeInputs,
        cancellation: &CancellationToken,
    ) -> Result<(), UpdateError> {
        let directory = open_secure(&inputs.directory, true)?;
        let identity = directory.metadata().map_err(|_| unavailable_input())?;
        let check = || -> Result<(), UpdateError> {
            let current = open_secure(&inputs.directory, true)?
                .metadata()
                .map_err(|_| unavailable_input())?;
            if (identity.dev(), identity.ino()) != (current.dev(), current.ino()) {
                return Err(unavailable_input());
            }
            let mut captured = inputs.capture()?;
            let source = fingerprint_inputs(&inputs.project, &captured);
            let original = &mut captured.last_mut().ok_or_else(unavailable_input)?.1;
            original.zeroize();
            *original = self.environment.to_vec();
            let target = fingerprint_inputs(&inputs.project, &captured);
            for (_, bytes) in &mut captured {
                bytes.zeroize();
            }
            if source != self.checked.source_configuration_sha256
                || target != self.checked.target_configuration_sha256
            {
                return Err(UpdateError::Conflict(
                    "Installer inputs changed before image publication; preserve update evidence."
                        .into(),
                ));
            }
            check_environment(inputs)?;
            if cancellation.is_cancelled() {
                return Err(UpdateError::Cancelled);
            }
            Ok(())
        };
        check()?;
        let temporary = format!(".citadel-image-{}.tmp", uuid::Uuid::now_v7());
        let result = (|| -> Result<(), UpdateError> {
            let mut file = File::from(
                openat(
                    &directory,
                    temporary.as_str(),
                    OFlags::WRONLY
                        | OFlags::CREATE
                        | OFlags::EXCL
                        | OFlags::NOFOLLOW
                        | OFlags::CLOEXEC,
                    Mode::from_raw_mode(0o600),
                )
                .map_err(|_| unavailable_input())?,
            );
            file.write_all(&self.environment)
                .and_then(|()| file.sync_all())
                .map_err(|_| unavailable_input())?;
            check()?;
            renameat(&directory, temporary.as_str(), &directory, ".env")
                .map_err(|_| unavailable_input())?;
            directory.sync_all().map_err(|_| unavailable_input())?;
            let mut actual = inputs.capture()?;
            let fingerprint = fingerprint_inputs(&inputs.project, &actual);
            for (_, bytes) in &mut actual {
                bytes.zeroize();
            }
            if fingerprint != self.checked.target_configuration_sha256 {
                return Err(unavailable_input());
            }
            Ok(())
        })();
        if result.is_err() {
            let _ = unlinkat(&directory, temporary.as_str(), AtFlags::empty());
        }
        result
    }
}

/// Check before shutdown as well as immediately before publication.
pub(super) fn check_environment(inputs: &InstallerComposeInputs) -> Result<(), UpdateError> {
    let env = open_secure(&inputs.directory.join(".env"), false)?
        .metadata()
        .map_err(|_| unavailable_input())?;
    // Do not silently change ownership or replace one alias of a hard-linked secret.
    if env.uid() != rustix::process::geteuid().as_raw()
        || env.nlink() != 1
        || env.mode() & 0o077 != 0
    {
        return Err(unavailable_input());
    }
    Ok(())
}
