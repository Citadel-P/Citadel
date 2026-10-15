//! Policy-independent, temporary Restic snapshot and restore on one daemon.
use super::*;
use crate::connectors::{
    agent::{client::AgentContainerAction, execution::AgentExecutionClient},
    docker::{DockerClient, DockerExecEvent},
    routing::platforms::runtime::Runtime,
};
use citadel_contracts::citadel::{
    containers::v1::{CreateContainerRequest, ExecBinaryRequest},
    shared_models::v1::Mount,
};
use citadel_platforms::{ContainerInventoryPort, containers::ContainerInspectionPort};
use futures_util::StreamExt;
use serde_json::{Value, json};
use std::{collections::HashMap, time::Duration};
use uuid::Uuid;

use crate::external::backups::current_copy::{COPY_JOB_LABEL, CURRENT_COPY_SCRIPT};
const TIMEOUT: Duration = Duration::from_secs(14_400);
fn helper_name(id: Uuid) -> String {
    format!("citadel-backup-helper-copy-{}", id.simple())
}
fn agent(runtime: &Runtime<'_>) -> Option<AgentExecutionClient> {
    match runtime {
        Runtime::Local(_) => None,
        Runtime::Agent(a) => Some(AgentExecutionClient::Direct(Arc::new(a.clone()))),
        Runtime::Edge(e) => Some(AgentExecutionClient::Edge(e.session.clone())),
    }
}
fn helper(platform: Uuid, job: &CopyRestore, image: String) -> CreateContainerRequest {
    CreateContainerRequest {
        name: helper_name(job.id),
        image_id: image,
        working_dir: Some("/tmp".into()),
        user: Some("0".into()),
        memory_limit: Some(512 * 1024 * 1024),
        memory_swap: Some(512 * 1024 * 1024),
        pids_limit: Some(128),
        auto_remove: Some(true),
        privileged: Some(false),
        readonly_rootfs: Some(false),
        network_mode: Some("none".into()),
        labels: HashMap::from([
            ("citadel.backup-helper".into(), "true".into()),
            ("citadel.platform-id".into(), platform.to_string()),
            ("com.citadel.platform-id".into(), platform.to_string()),
            ("com.citadel.system".into(), "true".into()),
            ("com.citadel.system-role".into(), "backup-helper".into()),
            (COPY_JOB_LABEL.into(), job.id.to_string()),
        ]),
        entry_point: vec!["/bin/sh".into()],
        command: vec!["-c".into(), "trap 'exit 0' TERM INT; sleep 14700".into()],
        cap_add: vec!["DAC_READ_SEARCH".into(), "FOWNER".into(), "CHOWN".into()],
        cap_drop: vec!["ALL".into()],
        security_opt: vec!["no-new-privileges".into()],
        mounts: vec![
            (&job.source, "/source", true),
            (&job.target, "/target/source", false),
        ]
        .into_iter()
        .map(|(source, target, read_only)| Mount {
            source: Some(source.clone()),
            target: Some(target.into()),
            r#type: Some("volume".into()),
            read_only: Some(read_only),
            ..Default::default()
        })
        .collect(),
        ..Default::default()
    }
}
fn local_config(r: &CreateContainerRequest) -> Value {
    json!({"Image":r.image_id,"User":r.user,"WorkingDir":r.working_dir,"Entrypoint":r.entry_point,"Cmd":r.command,"Labels":r.labels,
        "HostConfig":{"AutoRemove":r.auto_remove,"Memory":r.memory_limit,"MemorySwap":r.memory_swap,"PidsLimit":r.pids_limit,"Privileged":false,"ReadonlyRootfs":false,"NetworkMode":r.network_mode,"CapAdd":r.cap_add,"CapDrop":r.cap_drop,"SecurityOpt":r.security_opt,
        "Mounts":r.mounts.iter().map(|m|json!({"Type":m.r#type,"Source":m.source,"Target":m.target,"ReadOnly":m.read_only})).collect::<Vec<_>>()}})
}
async fn remove_helper(
    runtime: &Runtime<'_>,
    platform: Uuid,
    job: &CopyRestore,
    cancel: &CancellationToken,
) -> Result<(), CopyError> {
    let containers = match runtime {
        Runtime::Local(docker) => ContainerInventoryPort::list_containers(*docker, cancel).await,
        _ => {
            agent(runtime)
                .expect("remote runtime")
                .list_containers(cancel)
                .await
        }
    }
    .map_err(storage_error)?;
    let name = helper_name(job.id);
    let Some(container) = containers
        .iter()
        .find(|container| container.name.trim_start_matches('/') == name)
    else {
        return Ok(());
    };
    // Inspection requires an immutable ID. Name lookup alone is not ownership.
    let doc = match runtime.inspection(&container.id, cancel).await {
        Ok(doc) => doc,
        Err(e) if e.kind == RuntimeErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(storage_error(e)),
    };
    let labels = &doc["config"]["labels"];
    if labels[COPY_JOB_LABEL].as_str() != Some(job.id.to_string().as_str())
        || labels["citadel.platform-id"].as_str() != Some(platform.to_string().as_str())
        || labels["com.citadel.system-role"] != "backup-helper"
    {
        return Err(conflict(
            "Copy helper ownership could not be verified; no container was removed.",
        ));
    }
    let id = doc["id"]
        .as_str()
        .filter(|id| !id.is_empty())
        .ok_or_else(|| conflict("Copy helper identity is missing."))?;
    match runtime {
        Runtime::Local(docker) => match docker.delete_container(id, false, true).await {
            Ok(()) => Ok(()),
            Err(crate::connectors::docker::DockerError::Api { status, .. })
                if status == reqwest::StatusCode::NOT_FOUND =>
            {
                Ok(())
            }
            Err(error) => Err(storage_error(error)),
        },
        _ => agent(runtime)
            .expect("remote runtime")
            .delete_container(id, cancel)
            .await
            .map_err(storage_error),
    }
}
impl CopyStorageRuntime {
    pub(super) async fn cleanup_current_helper(
        &self,
        plan: &CopyPlan,
        job: &CopyRestore,
        cancel: &CancellationToken,
    ) -> Result<(), CopyError> {
        let volume = plan
            .preview
            .volumes
            .iter()
            .find(|v| v.target == job.target)
            .ok_or_else(|| conflict("Copy receipt is missing."))?;
        let runtime = self
            .runtime
            .resolve(
                plan.source.platform_id,
                volume.docker_node_id.as_deref(),
                false,
                cancel,
            )
            .await
            .map_err(storage_error)?;
        tokio::time::timeout(
            Duration::from_secs(30),
            remove_helper(&runtime, plan.source.platform_id, job, cancel),
        )
        .await
        .map_err(|_| {
            conflict("Copy helper cleanup timed out. Retry cleanup when the platform is available.")
        })?
    }
    pub(super) async fn current(
        &self,
        plan: &CopyPlan,
        job: &CopyRestore,
        cancel: &CancellationToken,
    ) -> Result<(), CopyError> {
        let volume = plan
            .preview
            .volumes
            .iter()
            .find(|v| v.target == job.target && v.source == job.source)
            .ok_or_else(|| conflict("Copy receipt does not match its plan."))?;
        let platform = plan.source.platform_id;
        let runtime = self
            .runtime
            .resolve(platform, volume.docker_node_id.as_deref(), true, cancel)
            .await
            .map_err(storage_error)?;
        let volumes = self
            .runtime
            .volumes(platform, volume.docker_node_id.as_deref(), cancel)
            .await
            .map_err(storage_error)?;
        let owner =
            crate::external::backups::restore_storage::RestoreVolumeOwner::new(platform, job.id);
        let execution = async {
            let source = volumes
                .inspect_volume(&job.source, cancel)
                .await
                .map_err(storage_error)?;
            if source.content_access_allowed != Some(true)
                || source.driver != "local"
                || source.scope != "local"
                || !source.options.is_empty()
            {
                return Err(conflict("Only ordinary local volumes can be copied."));
            }
            match volumes.inspect_volume(&job.target, cancel).await {
                Err(e) if e.kind == RuntimeErrorKind::NotFound => {}
                Err(e) => return Err(storage_error(e)),
                Ok(_) => {
                    return Err(conflict(
                        "Destination already exists. Current data copies are never replayed over existing storage.",
                    ));
                }
            }
            let created = volumes
                .create_volume(&owner.request(&job.target), cancel)
                .await
                .map_err(storage_error)?;
            owner
                .verify(&job.target, &created)
                .map_err(CopyError::Conflict)?;
            let remote = agent(&runtime);
            let image = if let Some(a) = &remote {
                let image = a.runtime_image(cancel).await.map_err(storage_error)?;
                if image.is_empty() {
                    self.image.clone()
                } else {
                    image
                }
            } else {
                self.image.clone()
            };
            if let Runtime::Local(docker) = &runtime {
                match docker.inspect_image(&image).await {
                    Ok(_) => {}
                    Err(crate::connectors::docker::DockerError::Api { status, .. })
                        if status == reqwest::StatusCode::NOT_FOUND =>
                    {
                        use citadel_platforms::image_pull::ImagePullPort;
                        let mut stream = docker
                            .pull_image_stream(&image, None, cancel)
                            .await
                            .map_err(storage_error)?;
                        while let Some(item) = stream.next().await {
                            let item = item.map_err(storage_error)?;
                            if item.error_message.is_some() || item.error.is_some() {
                                return Err(conflict(
                                    "Could not pull the configured Restic helper image.",
                                ));
                            }
                        }
                    }
                    Err(_) => {
                        return Err(conflict(
                            "Could not inspect the configured Restic helper image.",
                        ));
                    }
                }
            }
            let request = helper(platform, job, image);
            let id = match &runtime {
                Runtime::Local(docker) => docker
                    .create_container(&request.name, &local_config(&request))
                    .await
                    .map_err(storage_error)?,
                _ => remote
                    .as_ref()
                    .expect("remote runtime")
                    .create_container(request, cancel)
                    .await
                    .map_err(storage_error)?,
            };
            // The created helper pins both mounts against Docker removal/recreation.
            let source = volumes
                .inspect_volume(&job.source, cancel)
                .await
                .map_err(storage_error)?;
            if source.content_access_allowed != Some(true)
                || source.driver != "local"
                || source.scope != "local"
                || !source.options.is_empty()
            {
                return Err(conflict(
                    "Source storage changed before the copy helper was created.",
                ));
            }
            owner
                .verify(
                    &job.target,
                    &volumes
                        .inspect_volume(&job.target, cancel)
                        .await
                        .map_err(storage_error)?,
                )
                .map_err(CopyError::Conflict)?;
            let containers = match &runtime {
                Runtime::Local(docker) => ContainerInventoryPort::list_containers(*docker, cancel)
                    .await
                    .map_err(storage_error)?,
                _ => remote
                    .as_ref()
                    .expect("remote runtime")
                    .list_containers(cancel)
                    .await
                    .map_err(storage_error)?,
            };
            crate::external::backups::restore_storage::ensure_unused(&job.target, &id, &containers)
                .map_err(CopyError::Conflict)?;
            match &runtime {
                Runtime::Local(docker) => {
                    docker.start_container(&id).await.map_err(storage_error)?
                }
                _ => remote
                    .as_ref()
                    .expect("remote runtime")
                    .change_containers_state(
                        std::slice::from_ref(&id),
                        AgentContainerAction::Start,
                        cancel,
                    )
                    .await
                    .map_err(storage_error)?,
            }
            let command = vec!["/bin/sh".into(), "-ec".into(), CURRENT_COPY_SCRIPT.into()];
            let mut password = [0u8; 32];
            getrandom::fill(&mut password).map_err(storage_error)?;
            let password = password
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            let env = HashMap::from([
                ("RESTIC_REPOSITORY".into(), "/tmp/citadel-copy".into()),
                ("RESTIC_PASSWORD".into(), password),
            ]);
            let exit = match &runtime {
                Runtime::Local(docker) => local_exec(docker, &id, command, env, cancel).await?,
                _ => {
                    remote
                        .as_ref()
                        .expect("remote runtime")
                        .exec_binary(
                            ExecBinaryRequest {
                                container_id: id,
                                cmd: command,
                                env,
                                attach_stdout: Some(true),
                                attach_stderr: Some(true),
                                tty: false,
                            },
                            TIMEOUT,
                            1024 * 1024,
                            cancel,
                        )
                        .await
                        .map_err(storage_error)?
                        .exit_code
                }
            };
            if exit != 0 {
                return Err(conflict(
                    "The temporary snapshot or restore failed. Check source consistency and free disk space; the partial destination was retained for cleanup.",
                ));
            }
            Ok(())
        };
        let result = tokio::select! {
            ()=cancel.cancelled()=>Err(conflict("Current data copy cancelled; partial storage may remain.")),
            result=tokio::time::timeout(TIMEOUT,execution)=>result.unwrap_or_else(|_| Err(conflict("Current data copy timed out; partial storage may remain."))),
        };
        // Always remove the helper by inspected identity, including ambiguous
        // create/start responses. Its temporary encrypted repository dies with it.
        let cleanup_cancel = CancellationToken::new();
        let cleanup = tokio::time::timeout(
            Duration::from_secs(30),
            remove_helper(&runtime, platform, job, &cleanup_cancel),
        )
        .await;
        match cleanup {
            Ok(Ok(())) => result,
            _ => Err(conflict(
                "Copy helper cleanup could not be verified. No workload was created. Retry Clean up when the platform is available.",
            )),
        }
    }
}
async fn local_exec(
    docker: &DockerClient,
    id: &str,
    command: Vec<String>,
    env: HashMap<String, String>,
    cancel: &CancellationToken,
) -> Result<i32, CopyError> {
    let mut stream = docker
        .execute_binary(
            id,
            citadel_docker_api::models::ExecConfig {
                cmd: Some(command),
                env: Some(env.into_iter().map(|(k, v)| format!("{k}={v}")).collect()),
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                tty: Some(false),
                ..Default::default()
            },
            cancel,
        )
        .await
        .map_err(storage_error)?;
    let mut bytes = 0usize;
    while let Some(event) = stream.next().await {
        match event.map_err(storage_error)? {
            DockerExecEvent::Exit(code) => return Ok(code),
            DockerExecEvent::Output { data, .. } => {
                bytes += data.len();
                if bytes > 1024 * 1024 {
                    return Err(conflict("Copy helper exceeded its output limit."));
                }
            }
        }
    }
    Err(conflict(
        "Copy helper disconnected before reporting completion.",
    ))
}
