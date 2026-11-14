use crate::direct::{Runtime, docker_error};
use citadel_contracts::citadel::platforms::v1::{
    AgentUpdateInfo, UpdateAgentRequest, UpdateAgentResponse,
};
use futures_util::StreamExt;
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
        if request.cleanup && (!request.resume || request.source_container_id.is_empty()) {
            return Err(Status::invalid_argument(
                "Cleanup requires a settled standalone update",
            ));
        }
        if request.resume {
            if !request.source_container_id.is_empty() {
                if request.source_container_id.len() != 64
                    || !request
                        .source_container_id
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit())
                {
                    return Err(Status::invalid_argument(
                        "Invalid source container identity",
                    ));
                }
                let id = self.runtime_container.as_ref().ok_or_else(|| {
                    Status::failed_precondition("Agent container identity is unavailable")
                })?;
                let current = self
                    .docker
                    .inspect_container_document(id)
                    .await
                    .map_err(docker_error)?;
                if request.cleanup {
                    return super::cleanup::cleanup(
                        &self.docker,
                        &current,
                        &request.source_container_id,
                        &request.source_image,
                        &request.operation_id,
                    )
                    .await;
                }
                if current["Id"] == request.source_container_id {
                    if current["Image"] != request.source_image
                        || current["State"]["Running"] != true
                    {
                        return Err(Status::failed_precondition(
                            "Agent source changed during recovery",
                        ));
                    }
                    // Only the live admission gate can prove preparation ended before
                    // helper creation. Helper absence alone cannot fence delayed Docker calls.
                    return self.update_gate.settle_preparation(operation);
                }
                let name = crate::updates::helper_name(
                    &request.operation_id,
                    &request.source_container_id,
                );
                let helper = self
                    .docker
                    .inspect_container_document(&name)
                    .await
                    .map_err(docker_error)?;
                crate::updates::settled(
                    &current,
                    &helper,
                    &request.source_container_id,
                    &request.source_image,
                    &request.operation_id,
                )
                .map_err(Status::failed_precondition)?;
            }
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
        let info = self.agent_update_info().await;
        let mut drain = self
            .update_gate
            .drain(
                uuid::Uuid::parse_str(&request.operation_id)
                    .map_err(|_| Status::invalid_argument("Invalid operation"))?,
            )
            .await?;
        let preparation = async {
            let info = info?;
            if !info.supported
                || info.installation_id != request.installation_id
                || info.source_image != request.source_image
            {
                return Err(Status::failed_precondition(
                    "Agent installation changed or requires manual update",
                ));
            }

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
            // Reinspect after the pull: helper and replacement must use the same live source.
            let source = self
                .docker
                .inspect_container_document(&info.container_id)
                .await
                .map_err(docker_error)?;
            if source["Id"] != info.container_id || source["Image"] != info.source_image {
                return Err(Status::failed_precondition("Agent source identity changed"));
            }
            let configuration = crate::updates::helper_config(
                &source,
                &request.target_image,
                &request.operation_id,
            )
            .map_err(Status::failed_precondition)?;
            // Creation can also complete after a lost response. Retain admission before dispatch.
            drain.retain()?;
            let helper = self
                .docker
                .create_container(
                    &crate::updates::helper_name(&request.operation_id, &info.container_id),
                    &configuration,
                )
                .await
                .map_err(docker_error)?;
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
