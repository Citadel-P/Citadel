//! Reuse the registered profile checks without reading the database or repairing identity.
use super::super::{
    InstallerComposeInputs, RegisteredComposeInstallation,
    docker::{LocalDocker, failed},
};
use super::observation::{CoreState, Observation, observe_with_helper};
use citadel_updates::UpdateError;

pub(in crate::external::updates) async fn verify_handoff_source(
    docker: &LocalDocker<'_>,
    registration: &RegisteredComposeInstallation,
    helper: Option<&str>,
) -> Result<(), UpdateError> {
    verify_source(docker, registration, helper, CoreState::Healthy)
        .await
        .map(|_| ())
}

pub(in crate::external::updates) async fn verify_stopped_source(
    docker: &LocalDocker<'_>,
    registration: &RegisteredComposeInstallation,
    helper: &str,
) -> Result<String, UpdateError> {
    verify_source(docker, registration, Some(helper), CoreState::Stopped)
        .await
        .map(|observed| observed.retained_fingerprint)
}

async fn verify_source(
    docker: &LocalDocker<'_>,
    registration: &RegisteredComposeInstallation,
    helper: Option<&str>,
    state: CoreState,
) -> Result<Observation, UpdateError> {
    let inputs = InstallerComposeInputs {
        directory: registration.directory.clone(),
        project: registration.project.clone(),
        compose_files: registration.compose_files.clone(),
    };
    let (checked, config) = inputs.resolve(docker.process, docker.cancellation).await?;
    let observed = observe_with_helper(
        docker,
        &inputs,
        &config,
        &checked,
        &registration.state_directory,
        helper,
        state,
    )
    .await?;
    if checked.configuration_sha256 != registration.configuration_sha256
        || checked.core_volume != registration.core_volume
        || checked.database_volume != registration.database_volume
        || checked.network != registration.network
        || observed.daemon != registration.daemon_id
        || observed.core != registration.core_container_id
        || observed.database != registration.database_container_id
        || observed.fingerprint != registration.runtime_sha256
    {
        return Err(failed());
    }
    Ok(observed)
}

pub(in crate::external::updates) async fn verify_recoverable_source(
    docker: &LocalDocker<'_>,
    registration: &RegisteredComposeInstallation,
    helper: Option<&str>,
) -> Result<(), UpdateError> {
    verify_source(
        docker,
        registration,
        helper,
        CoreState::AcceptedTargetRunning,
    )
    .await
    .map(|_| ())
}
