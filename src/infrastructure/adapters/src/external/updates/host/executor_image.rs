use super::HostUpdateRequest;
use crate::external::updates::{
    docker::{LocalDocker, failed, single},
    handoff::release_image,
};
use citadel_updates::{UpdateError, official_image_digest};

pub(super) struct HostImage {
    pub executor_id: String,
    pub core_digest: String,
    pub architecture: String,
}

pub(super) async fn resolve(
    docker: &LocalDocker<'_>,
    request: &HostUpdateRequest,
    core_id: &str,
) -> Result<HostImage, UpdateError> {
    let raw = docker.json(&["image", "inspect", core_id]).await?;
    let image = single(&raw)?;
    let architecture = image["Architecture"]
        .as_str()
        .filter(|s| matches!(*s, "amd64" | "arm64"))
        .ok_or_else(failed)?;
    if image["Os"] != "linux" {
        return Err(failed());
    }
    let digests = image["RepoDigests"].as_array().ok_or_else(failed)?;
    let mut executor_id = core_id.to_owned();
    let core_digest = if let Some(operation) = request.cleanup_operation() {
        let plan = &operation.plan;
        plan.validate(operation.created_at)?;
        if architecture != plan.target.architecture
            || !digests.iter().any(|r| r == &plan.target.image)
        {
            return Err(UpdateError::Conflict("The running Core image does not match the approved update target. Preserve the checkpoint and update state.".into()));
        }
        // Journal validation uses the source binary's embedded migration catalog.
        // The running target still verifies its own database and completion receipt.
        if plan.source.image != plan.target.image {
            executor_id = release_image(docker, &plan.source).await.map_err(|error| match error {
                UpdateError::Cancelled => error,
                _ => UpdateError::Conflict("Update cleanup could not verify the approved source Core image on this Docker host. Keep both release images, the checkpoint and update state.".into()),
            })?;
        }
        &plan.target.image
    } else {
        digests
            .iter()
            .filter_map(|d| d.as_str())
            .find(|d| official_image_digest(d))
            .ok_or_else(failed)?
    };
    Ok(HostImage {
        executor_id,
        core_digest: core_digest.into(),
        architecture: architecture.into(),
    })
}
