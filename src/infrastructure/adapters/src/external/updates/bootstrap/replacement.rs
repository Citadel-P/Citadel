//! Observe the approved replacement without requiring normal application readiness.
use super::super::{
    CheckedCoreReplacement, InstallerComposeInputs, RegisteredComposeInstallation,
    docker::{LocalDocker, failed},
};
use super::observation::{CoreState, observe_with_helper};
use citadel_updates::UpdateError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::external::updates) struct ObservedReplacement {
    pub container_id: String,
    pub image_id: String,
    pub started_at: String,
    pub runtime_sha256: String,
    pub retained_runtime_sha256: String,
}

pub(in crate::external::updates) async fn observe_replacement(
    docker: &LocalDocker<'_>,
    registration: &RegisteredComposeInstallation,
    configuration: &CheckedCoreReplacement,
    helper: Option<&str>,
) -> Result<ObservedReplacement, UpdateError> {
    let inputs = InstallerComposeInputs {
        directory: registration.directory.clone(),
        project: registration.project.clone(),
        compose_files: registration.compose_files.clone(),
    };
    let (checked, config) = inputs.resolve(docker.process, docker.cancellation).await?;
    if configuration.source_configuration_sha256 != registration.configuration_sha256
        || checked.configuration_sha256 != configuration.target_configuration_sha256
        || checked.core_image != configuration.target_image
        || checked.core_volume != registration.core_volume
        || checked.database_volume != registration.database_volume
        || checked.network != registration.network
    {
        return Err(failed());
    }
    let observed = observe_with_helper(
        docker,
        &inputs,
        &config,
        &checked,
        &registration.state_directory,
        helper,
        CoreState::TargetRunning,
    )
    .await?;
    if observed.daemon != registration.daemon_id
        || observed.core == registration.core_container_id
        || helper.is_some_and(|id| observed.core == id)
        || observed.database != registration.database_container_id
        || !citadel_updates::is_sha256(&observed.core)
        || !observed
            .core_image
            .strip_prefix("sha256:")
            .is_some_and(citadel_updates::is_sha256)
        || registration.fingerprint(std::path::Path::new("/"))?
            != configuration.target_configuration_sha256
        || !docker
            .containers(&format!("id={}", registration.core_container_id))
            .await?
            .is_empty()
    {
        return Err(failed());
    }
    Ok(ObservedReplacement {
        container_id: observed.core,
        image_id: observed.core_image,
        started_at: observed.core_started_at,
        runtime_sha256: observed.fingerprint,
        retained_runtime_sha256: observed.retained_fingerprint,
    })
}
