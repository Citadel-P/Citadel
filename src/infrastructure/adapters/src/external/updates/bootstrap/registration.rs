//! Persist local operator registration after stable runtime observation.
use super::super::docker::refuse;
use super::super::files::validate_absolute;
use super::super::{InstallerComposeInputs, JournalDirectory, RegisteredComposeInstallation};
use super::inspection::inspect_installation;
use citadel_execution::ProcessRunner;
use citadel_updates::UpdateError;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
pub(super) const REGISTRATION: &str = "installation.json";

pub async fn register_compose_installation(
    inputs: &InstallerComposeInputs,
    state_directory: PathBuf,
    process: &dyn ProcessRunner,
    cancellation: &CancellationToken,
) -> Result<RegisteredComposeInstallation, UpdateError> {
    register(inputs, state_directory, process, cancellation, false).await
}

/// Explicit operator refresh after a planned Core/configuration change. Never
/// replaces installation/storage identity or unresolved execution evidence.
pub async fn refresh_compose_installation(
    inputs: &InstallerComposeInputs,
    state_directory: PathBuf,
    process: &dyn ProcessRunner,
    cancellation: &CancellationToken,
) -> Result<RegisteredComposeInstallation, UpdateError> {
    register(inputs, state_directory, process, cancellation, true).await
}

async fn register(
    inputs: &InstallerComposeInputs,
    state_directory: PathBuf,
    process: &dyn ProcessRunner,
    cancellation: &CancellationToken,
    refresh: bool,
) -> Result<RegisteredComposeInstallation, UpdateError> {
    validate_absolute(&state_directory)?;
    if state_directory == inputs.directory || state_directory == Path::new("/") {
        return Err(refuse("Use a dedicated private update state directory."));
    }
    let state = JournalDirectory::open(state_directory.clone())?;
    let _lock = state.lock()?;
    // Invalid or unreadable existing evidence is never treated as missing.
    let existing: Option<RegisteredComposeInstallation> = state.read_optional(REGISTRATION)?;
    if let Some(existing) = &existing {
        existing.validate()?;
    }
    if refresh {
        if existing.is_none() {
            return Err(UpdateError::NotFound);
        }
        state.require_registration_only()?;
    }
    if existing.is_none() {
        state.require_unregistered()?;
    }
    let registration = inspect_installation(
        inputs,
        state_directory,
        existing.as_ref().map(|r| r.id).unwrap_or_else(Uuid::now_v7),
        process,
        cancellation,
        refresh,
    )
    .await?
    .0;
    state.verify_location()?;
    if let Some(existing) = existing {
        if refresh && existing != registration {
            let mut expected = existing.clone();
            expected.core_container_id = registration.core_container_id.clone();
            expected.configuration_sha256 = registration.configuration_sha256.clone();
            expected.runtime_sha256 = registration.runtime_sha256.clone();
            if expected != registration
                || state.read::<RegisteredComposeInstallation>(REGISTRATION)? != existing
                || cancellation.is_cancelled()
            {
                return Err(UpdateError::Conflict(
                    "Registration refresh changed installation identity or was interrupted.".into(),
                ));
            }
            state.verify_lock(&_lock)?;
            state.require_registration_only()?;
            state.write(REGISTRATION, &registration)?;
            return Ok(registration);
        }
        if existing != registration {
            return Err(UpdateError::Conflict("A different installation is already registered. Existing registration and recovery state were preserved.".into()));
        }
        // A previous create may have published the entry before directory fsync
        // failed. An idempotent acknowledgement must make that entry durable.
        state.sync()?;
        return Ok(existing);
    }
    state.create(REGISTRATION, &registration)?;
    Ok(registration)
}
