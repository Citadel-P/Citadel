use crate::direct::{Runtime, docker_error};
use citadel_contracts::citadel::platforms::v1::{
    AgentUpdateInfo, UpdateAgentRequest, UpdateAgentResponse,
};
use futures_util::StreamExt;
use serde_json::json;
use tonic::Status;

impl Runtime {
    pub(crate) fn update_admission(
        &self,
        kind: citadel_contracts::citadel::edge::v1::EdgeCommandKind,
    ) -> Result<Option<tokio::sync::OwnedRwLockReadGuard<()>>, Status> {
        use citadel_contracts::citadel::edge::v1::EdgeCommandKind::*;
        if matches!(
            kind,
            PlatformAgentUpdateDrain
                | PlatformAgentUpdateInfo
                | PlatformUpdateAgent
                | PlatformGetInfo
                | PlatformCheckHealth
                | PlatformDaemonEventsStream
                | PlatformStatsStream
                | ContainerStatsStream
                | ContainersStatsStream
                | ContainerLogsStream
                | SwarmServiceLogs
                | SwarmTaskLogs
                | ImageReconcileData
                | ImageReconcileBuild
                | ScriptReconcile
        ) {
            return Ok(None);
        }
        self.update_gate.enter().map(Some)
    }
    pub(crate) async fn drain_agent_update(
        &self,
        request: citadel_contracts::citadel::platforms::v1::AgentUpdateDrainRequest,
    ) -> Result<(), Status> {
        let operation = uuid::Uuid::parse_str(&request.operation_id)
            .map_err(|_| Status::invalid_argument("Invalid Agent update operation"))?;
        if request.resume {
            return self.update_gate.resume(operation);
        }
        let mut drain = self.update_gate.drain(operation).await?;
        let _heavy = tokio::time::timeout(
            std::time::Duration::from_secs(120),
            self.heavy
                .clone()
                .acquire_many_owned(self.heavy_capacity as u32),
        )
        .await
        .map_err(|_| Status::deadline_exceeded("Agent has unresolved work"))?
        .map_err(|_| Status::unavailable("Agent is stopping"))?;
        drain.retain()?;
        Ok(())
    }
    pub(crate) async fn agent_update_info(&self) -> Result<AgentUpdateInfo, Status> {
        let Some(id) = &self.runtime_container else {
            return Ok(AgentUpdateInfo {
                version: crate::VERSION.into(),
                reason: "Manual update required: Agent is not running in a recognized container."
                    .into(),
                ..Default::default()
            });
        };
        let source = self
            .docker
            .inspect_container_document(id)
            .await
            .map_err(docker_error)?;
        let sample = format!("ghcr.io/citadel-p/citadel.agent@sha256:{}", "0".repeat(64));
        let error = if self.update_gate.is_closed() {
            Some("An Agent update is active or requires recovery.")
        } else {
            crate::updates::replacement_config(&source, &sample, "preflight").err()
        };
        let daemon = self
            .docker
            .get_info_with_details(&self.shutdown)
            .await
            .map_err(crate::direct::runtime_error)?
            .0
            .daemon_id;
        if daemon.is_empty() {
            return Err(Status::failed_precondition(
                "Docker daemon identity is unavailable",
            ));
        }
        Ok(AgentUpdateInfo {
            runtime_image: source["Config"]["Image"]
                .as_str()
                .unwrap_or_default()
                .into(),
            installation_id: format!(
                "{daemon}/{}",
                source["Name"]
                    .as_str()
                    .unwrap_or_default()
                    .trim_start_matches('/')
            ),
            container_id: source["Id"].as_str().unwrap_or_default().into(),
            version: crate::VERSION.into(),
            source_image: source["Image"].as_str().unwrap_or_default().into(),
            supported: error.is_none(),
            reason: error.unwrap_or_default().into(),
        })
    }
    pub(crate) async fn start_agent_update(
        &self,
        request: UpdateAgentRequest,
    ) -> Result<UpdateAgentResponse, Status> {
        uuid::Uuid::parse_str(&request.operation_id)
            .map_err(|_| Status::invalid_argument("Operation ID is invalid"))?;
        crate::updates::validate_image(&request.target_image).map_err(Status::invalid_argument)?;
        let info = self.agent_update_info().await?;
        if !info.supported
            || info.installation_id != request.installation_id
            || info.source_image != request.source_image
        {
            return Err(Status::failed_precondition(
                "Agent installation changed or requires manual update",
            ));
        }
        let mut drain = self
            .update_gate
            .drain(
                uuid::Uuid::parse_str(&request.operation_id)
                    .map_err(|_| Status::invalid_argument("Invalid operation"))?,
            )
            .await?;
        let preparation = async {
            // Reservation permits include unresolved builds/data operations; never erase ownership to update.
            let _heavy = tokio::time::timeout(
                std::time::Duration::from_secs(120),
                self.heavy
                    .clone()
                    .acquire_many_owned(self.heavy_capacity as u32),
            )
            .await
            .map_err(|_| Status::deadline_exceeded("Agent has unresolved work"))?
            .map_err(|_| Status::unavailable("Agent is stopping"))?;
            let mut pull = self
                .docker
                .pull_image(&request.target_image, None)
                .await
                .map_err(docker_error)?;
            while let Some(message) = pull.next().await {
                let message = message.map_err(docker_error)?;
                if message.error.is_some() || message.error_detail.is_some() {
                    return Err(Status::unavailable("Agent image pull failed"));
                }
            }
            let helper=self.docker.create_container(&format!("citadel-agent-updater-{}-{}",request.operation_id, &info.container_id[..12.min(info.container_id.len())]),&json!({
                "Image":request.target_image,"Entrypoint":["/app/citadel-agent"],
                "Cmd":["replace-agent",info.container_id,info.source_image,request.target_image,request.operation_id],
                "Labels":{"com.citadel.system":"true","com.citadel.system-role":"agent-updater","com.citadel.agent-update":request.operation_id},
                "HostConfig":{"Binds":["/var/run/docker.sock:/var/run/docker.sock"],"NetworkMode":"none","ReadonlyRootfs":true,"CapDrop":["ALL"],"SecurityOpt":["no-new-privileges"],"RestartPolicy":{"Name":"no"}}
            })).await.map_err(docker_error)?;
            drain.retain()?;
            // After creation, a lost start reply is ambiguous: keep admission closed for recovery.
            self.docker
                .start_container(&helper)
                .await
                .map_err(docker_error)?;
            Ok(UpdateAgentResponse { helper_id: helper })
        };
        // Keep the gate closed after a successful handoff; only the replacement reopens admission.
        tokio::time::timeout(std::time::Duration::from_secs(300), preparation)
            .await
            .map_err(|_| Status::deadline_exceeded("Agent update preparation timed out"))?
    }
}
