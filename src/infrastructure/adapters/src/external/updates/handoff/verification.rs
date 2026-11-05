use super::super::{
    InstallerComposeInputs,
    bootstrap::validate_executor_mounts,
    docker::{LocalDocker, single},
};
use super::evidence::{HelperAcknowledgement, HelperBinding, HelperLaunchIntent, conflict};
use citadel_updates::{UpdateError, is_sha256};
use serde_json::{Value, json};

pub(super) const NONCE: &str = "com.citadel.update-nonce";
pub(super) const OPERATION: &str = "com.citadel.update-operation";

pub(super) struct RunningHelper {
    pub image: String,
    pub started_at: String,
}

/// Network sharing copies PostgreSQL's hostname, so it cannot identify this
/// process. Docker exec enters the exact helper's private mount namespace;
/// compare that namespace with ours before acquiring execution ownership.
pub(super) async fn helper_process(
    docker: &LocalDocker<'_>,
    intent: &HelperLaunchIntent,
    binding: &HelperBinding,
) -> Result<(), UpdateError> {
    helper(docker, intent, binding).await?;
    let own = std::fs::read_link("/proc/self/ns/mnt").map_err(|_| conflict())?;
    let observed = docker
        .run(&[
            "exec",
            &binding.helper_container_id,
            "readlink",
            "/proc/self/ns/mnt",
        ])
        .await?;
    if std::str::from_utf8(&observed).map(str::trim).ok() != own.to_str() {
        return Err(conflict());
    }
    Ok(())
}

pub(super) async fn helper(
    docker: &LocalDocker<'_>,
    intent: &HelperLaunchIntent,
    binding: &HelperBinding,
) -> Result<RunningHelper, UpdateError> {
    let actual = helper_container(docker, intent, binding).await?;
    if actual["State"]["Running"] != true || actual["State"]["Status"] != "running" {
        return Err(conflict());
    }
    let started_at = actual["State"]["StartedAt"].as_str().ok_or_else(conflict)?;
    chrono::DateTime::parse_from_rfc3339(started_at).map_err(|_| conflict())?;
    Ok(RunningHelper {
        image: actual["Image"].as_str().ok_or_else(conflict)?.into(),
        started_at: started_at.into(),
    })
}

/// Later phases must retain the process incarnation accepted at handoff.
pub(super) async fn acknowledged_helper(
    docker: &LocalDocker<'_>,
    intent: &HelperLaunchIntent,
    acknowledgement: &HelperAcknowledgement,
) -> Result<RunningHelper, UpdateError> {
    let running = helper(docker, intent, &acknowledgement.binding).await?;
    if running.started_at != acknowledgement.started_at {
        return Err(conflict());
    }
    Ok(running)
}

/// A created container must pass the same profile checks before it can execute.
pub(super) async fn created_helper(
    docker: &LocalDocker<'_>,
    intent: &HelperLaunchIntent,
    binding: &HelperBinding,
) -> Result<(), UpdateError> {
    let actual = helper_container(docker, intent, binding).await?;
    if actual["State"]["Running"] != false || actual["State"]["Status"] != "created" {
        return Err(conflict());
    }
    Ok(())
}

/// Observe the acknowledged process without granting permission to resume Core.
/// Creation, execution and exit all retain the same exact container profile.
pub(super) async fn helper_exited(
    docker: &LocalDocker<'_>,
    intent: &HelperLaunchIntent,
    acknowledgement: &HelperAcknowledgement,
) -> Result<bool, UpdateError> {
    let actual = helper_container(docker, intent, &acknowledgement.binding).await?;
    let state = &actual["State"];
    if state["StartedAt"] != acknowledgement.started_at
        || state["OOMKilled"] != false
        || state["Error"] != ""
    {
        return Err(conflict());
    }
    match (state["Running"].as_bool(), state["Status"].as_str()) {
        (Some(true), Some("running")) => Ok(false),
        (Some(false), Some("exited")) if state["ExitCode"] == 0 => Ok(true),
        _ => Err(conflict()),
    }
}

pub(super) async fn helper_container(
    docker: &LocalDocker<'_>,
    intent: &HelperLaunchIntent,
    binding: &HelperBinding,
) -> Result<Value, UpdateError> {
    if !is_sha256(&binding.helper_container_id) || binding.nonce != intent.nonce {
        return Err(conflict());
    }
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
        &inputs,
        &intent.registration.state_directory,
        true,
    )?;
    // Docker copies the network donor's hostname on start. A created helper
    // still has its own default hostname; neither hostname is process identity.
    let hostname = if actual["State"]["Status"] == "created"
        || intent.operation.plan.database.is_some()
    {
        id[..12].to_owned()
    } else {
        container(docker, &intent.registration.database_container_id).await?["Config"]["Hostname"]
            .as_str()
            .filter(|v| !v.is_empty())
            .ok_or_else(conflict)?
            .to_owned()
    };
    if intent.operation.plan.database.is_some() {
        let networks = actual["NetworkSettings"]["Networks"]
            .as_object()
            .ok_or_else(conflict)?;
        if networks.len() != 1 || !networks.contains_key(&intent.registration.network) {
            return Err(conflict());
        }
        let endpoint = &networks[&intent.registration.network];
        if ["Aliases", "DNSNames"].iter().any(|field| {
            endpoint[*field]
                .as_array()
                .is_some_and(|names| names.iter().any(|name| name == "pg_db"))
        }) {
            return Err(conflict());
        }
    }
    if actual["Config"]["Hostname"] != hostname {
        return Err(conflict());
    }
    // A killed helper must not leave disposable Docker configuration in the
    // durable journal directory, where unknown entries prevent safe admission.
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
        || actual["State"]["Paused"] != false
        || actual["State"]["Restarting"] != false
        || actual["RestartCount"] != 0
        || actual["HostConfig"]["RestartPolicy"]["Name"] != "no"
        || actual["HostConfig"]["NetworkMode"]
            != if intent.operation.plan.database.is_some() {
                intent.registration.network.clone()
            } else {
                format!("container:{}", intent.registration.database_container_id)
            }
        || actual["HostConfig"]["Privileged"] != false
        || actual["HostConfig"]["ReadonlyRootfs"] != true
        || actual["HostConfig"]["Tmpfs"] != json!({"/tmp":"rw,noexec,nosuid,size=64m"})
        || !actual["Config"]["Env"].as_array().is_some_and(|env| {
            env.iter()
                .filter_map(Value::as_str)
                .filter(|value| value.starts_with("TMPDIR="))
                .eq(["TMPDIR=/tmp"])
        })
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
