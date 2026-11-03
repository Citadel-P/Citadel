//! Reuse the registered profile checks without reading the database or repairing identity.
use super::super::{
    InstallerComposeInputs, RegisteredComposeInstallation,
    docker::{LocalDocker, failed},
};
use super::observation::observe;
use citadel_updates::UpdateError;

pub(in crate::external::updates) async fn verify_handoff_source(
    docker: &LocalDocker<'_>,
    registration: &RegisteredComposeInstallation,
) -> Result<(), UpdateError> {
    let inputs = InstallerComposeInputs {
        directory: registration.directory.clone(),
        project: registration.project.clone(),
        compose_files: registration.compose_files.clone(),
    };
    let (checked, config) = inputs.resolve(docker.process, docker.cancellation).await?;
    let observed = observe(
        docker,
        &inputs,
        &config,
        &checked,
        &registration.state_directory,
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
    Ok(())
}
