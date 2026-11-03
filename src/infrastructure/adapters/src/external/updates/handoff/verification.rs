use super::super::{
    InstallerComposeInputs,
    bootstrap::validate_executor_mounts,
    docker::{LocalDocker, single},
};
use super::evidence::{HelperBinding, HelperLaunchIntent, conflict};
use citadel_updates::{UpdateError, is_sha256};
use serde_json::{Value, json};

pub(super) const NONCE: &str = "com.citadel.update-nonce";
pub(super) const OPERATION: &str = "com.citadel.update-operation";

pub(super) struct RunningHelper {
    pub image: String,
    pub started_at: String,
}

pub(super) async fn helper(
    docker: &LocalDocker<'_>,
    intent: &HelperLaunchIntent,
    binding: &HelperBinding,
) -> Result<RunningHelper, UpdateError> {
    let actual = checked_container(docker, intent, binding, true).await?;
    let started_at = actual["State"]["StartedAt"].as_str().ok_or_else(conflict)?;
    chrono::DateTime::parse_from_rfc3339(started_at).map_err(|_| conflict())?;
    Ok(RunningHelper {
        image: actual["Image"].as_str().ok_or_else(conflict)?.into(),
        started_at: started_at.into(),
    })
}

/// A created container must pass the same profile checks before it can execute.
pub(super) async fn created_helper(
    docker: &LocalDocker<'_>,
    intent: &HelperLaunchIntent,
    binding: &HelperBinding,
) -> Result<(), UpdateError> {
    checked_container(docker, intent, binding, false).await?;
    Ok(())
}

async fn checked_container(
    docker: &LocalDocker<'_>,
    intent: &HelperLaunchIntent,
    binding: &HelperBinding,
    running: bool,
) -> Result<Value, UpdateError> {
    let raw = docker
        .json(&["container", "inspect", &binding.helper_container_id])
        .await?;
    let actual = single(&raw)?;
    let id = &binding.helper_container_id;
    let inputs = InstallerComposeInputs {
        directory: intent.registration.directory.clone(),
        project: intent.registration.project.clone(),
        compose_files: intent.registration.compose_files.clone(),
    };
    let mounts = actual["Mounts"].as_array().ok_or_else(conflict)?;
    if mounts.len() != 4
        || mounts
            .iter()
            .filter(|m| {
                m["Type"] == "volume"
                    && m["Name"] == intent.registration.core_volume
                    && m["Destination"] == "/app/data"
                    && m["RW"] == false
            })
            .count()
            != 1
    {
        return Err(conflict());
    }
    let mut executor = actual.clone();
    executor["Mounts"] = Value::Array(
        mounts
            .iter()
            .filter(|m| m["Type"] != "volume")
            .cloned()
            .collect(),
    );
    validate_executor_mounts(
        &executor,
        &id[..12],
        &inputs,
        &intent.registration.state_directory,
        true,
    )?;
    if actual["Id"] != *id
        || actual["Name"] != format!("/{}", intent.helper_name())
        || actual["Config"]["Image"] != intent.operation.plan.helper_image
        || actual["Config"]["Labels"][NONCE] != intent.nonce.to_string()
        || actual["Config"]["Labels"][OPERATION] != intent.operation.id.to_string()
        || actual["Config"]["User"]
            != format!(
                "{}:{}",
                intent.registration.executor_uid, intent.registration.executor_gid
            )
        || actual["State"]["Running"] != running
        || actual["State"]["Paused"] != false
        || actual["State"]["Restarting"] != false
        || actual["State"]["Status"] != if running { "running" } else { "created" }
        || actual["RestartCount"] != 0
        || actual["HostConfig"]["RestartPolicy"]["Name"] != "no"
        || actual["HostConfig"]["NetworkMode"]
            != format!("container:{}", intent.registration.database_container_id)
        || actual["HostConfig"]["Privileged"] != false
        || actual["HostConfig"]["ReadonlyRootfs"] != true
        || actual["HostConfig"]["AutoRemove"] != false
        || actual["Config"]["Healthcheck"]["Test"] != json!(["NONE"])
        || actual["Path"] != "/app/citadel-server"
        || actual["Args"]
            != json!([
                "run-update-helper",
                "--state-directory",
                intent.registration.state_directory,
                "--operation-id",
                intent.operation.id.to_string(),
                "--fence",
                intent.operation.fence.to_string()
            ])
    {
        return Err(conflict());
    }
    if actual["Image"] != source_image(docker, intent).await? {
        return Err(conflict());
    }
    Ok(actual.clone())
}

pub(super) async fn source_image(
    docker: &LocalDocker<'_>,
    intent: &HelperLaunchIntent,
) -> Result<String, UpdateError> {
    release_image(docker, &intent.operation.plan.source).await
}

pub(super) async fn release_image(
    docker: &LocalDocker<'_>,
    identity: &citadel_updates::CoreIdentity,
) -> Result<String, UpdateError> {
    // Match release inspection: a containerd image store may report a platform
    // manifest ID separately from the locally addressable container image ID.
    let platform = format!("linux/{}", identity.architecture);
    let raw = docker
        .json(&["image", "inspect", "--platform", &platform, &identity.image])
        .await?;
    let image = single(&raw)?;
    if image["Os"] != "linux"
        || image["Architecture"] != identity.architecture
        || !(image["Config"]["Volumes"].is_null()
            || image["Config"]["Volumes"]
                .as_object()
                .is_some_and(|v| v.is_empty()))
        || !image["RepoDigests"]
            .as_array()
            .is_some_and(|v| v.iter().any(|s| s == &identity.image))
    {
        return Err(conflict());
    }
    let local = docker.json(&["image", "inspect", &identity.image]).await?;
    let id = single(&local)?["Id"]
        .as_str()
        .filter(|s| s.strip_prefix("sha256:").is_some_and(is_sha256))
        .ok_or_else(conflict)?;
    Ok(id.into())
}

pub(super) async fn container(docker: &LocalDocker<'_>, id: &str) -> Result<Value, UpdateError> {
    let raw = docker.json(&["container", "inspect", id]).await?;
    let actual = single(&raw)?;
    if actual["Id"] != id {
        return Err(conflict());
    }
    Ok(actual.clone())
}

pub(super) fn database_running(database: &Value) -> Result<(), UpdateError> {
    if database["State"]["Running"] != true
        || database["State"]["Paused"] != false
        || database["State"]["Restarting"] != false
        || database["State"]["Health"]["Status"] != "healthy"
    {
        return Err(conflict());
    }
    Ok(())
}
