//! Idempotent removal authorized by Core's persisted successful update outcome.
use citadel_docker::client::{DockerClient, DockerError};
use serde_json::Value;
use tonic::Status;

fn conflict() -> Status {
    Status::failed_precondition(
        "Agent update cleanup identity changed; retained containers require inspection",
    )
}

async fn inspect(docker: &DockerClient, id: &str) -> Result<Option<Value>, Status> {
    match docker.inspect_container_document(id).await {
        Ok(document) => Ok(Some(document)),
        Err(DockerError::Api { status, .. }) if status.as_u16() == 404 => Ok(None),
        Err(error) => Err(crate::direct::docker_error(error)),
    }
}

async fn remove(docker: &DockerClient, id: &str) -> Result<(), Status> {
    // Never force removal or remove identity/data volumes shared with the replacement.
    match docker.delete_container(id, false, false).await {
        Ok(()) => Ok(()),
        Err(DockerError::Api { status, .. }) if status.as_u16() == 404 => Ok(()),
        Err(error) => Err(crate::direct::docker_error(error)),
    }
}

pub(super) async fn cleanup(
    docker: &DockerClient,
    current: &Value,
    source: &str,
    source_image: &str,
    operation: &str,
) -> Result<(), Status> {
    let current_id = current["Id"]
        .as_str()
        .filter(|id| id.len() == 64 && id.bytes().all(|b| b.is_ascii_hexdigit()))
        .ok_or_else(conflict)?;
    let image = current["Config"]["Image"].as_str().ok_or_else(conflict)?;
    crate::updates::validate_image(image).map_err(Status::failed_precondition)?;
    if current_id == source
        || current["State"]["Running"] != true
        || current["Config"]["Labels"]["com.citadel.agent-update"] != operation
        || current["Config"]["Labels"]["com.citadel.system"] != "true"
        || !matches!(
            current["Config"]["Labels"]["com.citadel.system-role"].as_str(),
            Some("agent" | "edge-agent")
        )
        || current["Config"]["Labels"]
            .get("com.docker.swarm.service.id")
            .is_some()
    {
        return Err(conflict());
    }
    let helper = inspect(docker, &crate::updates::helper_name(operation, source)).await?;
    let previous = inspect(docker, source).await?;
    let helper_id = if let Some(helper) = &helper {
        crate::updates::settled(current, helper, source, source_image, operation)
            .map_err(Status::failed_precondition)?;
        Some(
            helper["Id"]
                .as_str()
                .filter(|id| {
                    id.len() == 64
                        && id.bytes().all(|b| b.is_ascii_hexdigit())
                        && *id != source
                        && current_id != *id
                })
                .ok_or_else(conflict)?,
        )
    } else {
        // The helper is removed last. Absence is accepted only on this post-commit
        // cleanup path, never as replacement evidence during ordinary settlement.
        if previous.is_some() {
            return Err(conflict());
        }
        None
    };
    if let Some(previous) = &previous {
        let expected_name = format!("/citadel-agent-previous-{operation}-{}", &source[..12]);
        if previous["Id"] != source
            || previous["Image"] != source_image
            || previous["Name"] != expected_name
            || previous["State"]["Running"] != false
            || previous["State"]["Status"] != "exited"
            || previous["HostConfig"]["RestartPolicy"]["Name"] != "no"
        {
            return Err(conflict());
        }
        remove(docker, source).await?;
    }
    if let Some(id) = helper_id {
        remove(docker, id).await?;
    }
    Ok(())
}
