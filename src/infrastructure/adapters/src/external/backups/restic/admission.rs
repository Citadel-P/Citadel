use super::*;
use citadel_contracts::citadel::images::v1::{
    BuildAdmissionResult, BuildCleanupState, DataOperationRequest,
};

#[derive(Clone)]
pub(super) struct DataScope {
    pub request: DataOperationRequest,
    operation: BackupDataOperation,
    host: BackupExecutionHost,
    agent: AgentExecutionClient,
}
impl ResticOperation<'_> {
    pub(super) async fn admit_data(
        &self,
        agent: &AgentExecutionClient,
        platform: Uuid,
        node: Option<&str>,
        cancellation: &CancellationToken,
    ) -> Result<Self, String> {
        let _wait = citadel_runtime::runtime_metrics::RuntimeWork::DataHostWait.start();
        let OperationOwner {
            repository,
            run: owner,
            attempt,
        } = self.owner;
        loop {
            if cancellation.is_cancelled() {
                return Err("Backup was cancelled while waiting for host capacity.".into());
            }
            let info = agent
                .check_build_host(cancellation)
                .await
                .map_err(|error| {
                    self.defer_if_unstarted();
                    error.to_string()
                })?;
            if !info.available {
                self.defer_if_unstarted();
                return Err("Backup execution host is unavailable.".into());
            }
            let host = BackupExecutionHost {
                daemon_id: info.docker_daemon_id,
                authority_id: Uuid::parse_str(&info.admission_authority_id)
                    .map_err(|_| "Invalid execution host identity")?,
                generation: info.admission_generation,
                data_slots: info.data_capacity,
                heavy_slots: info.heavy_capacity,
            };
            let operation = BackupDataOperation {
                platform_id: platform,
                node_id: node.map(str::to_owned),
                id: Uuid::now_v7(),
                repository_id: repository,
                owner_id: owner,
                attempt_id: attempt,
            };
            if self
                .executor
                .admission
                .reserve(&operation, &host)
                .await
                .map_err(|error| error.to_string())?
            {
                let request = DataOperationRequest {
                    run_id: owner.to_string(),
                    attempt_id: attempt.to_string(),
                    operation_id: operation.id.to_string(),
                    admission_authority_id: host.authority_id.to_string(),
                    docker_daemon_id: host.daemon_id.clone(),
                    admission_generation: host.generation,
                };
                let mut scoped = self.clone();
                scoped.data_scope = Some(DataScope {
                    request: request.clone(),
                    operation,
                    host,
                    agent: agent.clone(),
                });
                match agent.reserve_data(request, cancellation).await {
                    Ok(response)
                        if response.admission
                            == BuildAdmissionResult::BuildAdmissionAccepted as i32 =>
                    {
                        self.state
                            .execution_started
                            .store(true, std::sync::atomic::Ordering::Release);
                        return Ok(scoped);
                    }
                    Ok(response)
                        if matches!(
                            BuildAdmissionResult::try_from(response.admission),
                            Ok(BuildAdmissionResult::BuildAdmissionCapacityBusyNotStarted
                                | BuildAdmissionResult::BuildAdmissionFencedNotStarted)
                        ) =>
                    {
                        let scope = scoped.data_scope.as_ref().expect("assigned");
                        self.executor
                            .admission
                            .release(&scope.operation, &scope.host)
                            .await
                            .map_err(|error| error.to_string())?;
                    }
                    _ => {
                        // A lost admission response is not a refusal. Fence late
                        // requests and prove cleanup before releasing occupancy.
                        scoped.finish_data().await?;
                        return Err("Backup host admission could not be confirmed.".into());
                    }
                }
            }
            if self.defer_if_unstarted() {
                return Err("Waiting for execution host capacity.".into());
            }
            if let Some(progress) = &self.progress {
                progress(BackupLog {
                    stream: "system".into(),
                    message: "Waiting for execution host capacity.".into(),
                });
            }
            tokio::select! { () = cancellation.cancelled() => return Err("Backup was cancelled while waiting for host capacity.".into()), () = tokio::time::sleep(Duration::from_secs(5)) => {} }
        }
    }
    pub(super) async fn finish_data(&self) -> Result<(), String> {
        let _cleanup = citadel_runtime::runtime_metrics::RuntimeWork::DataHostCleanup.start();
        let scope = self
            .data_scope
            .as_ref()
            .ok_or("Backup host reservation is missing")?;
        let cleanup = async {
            loop {
                let response = scope
                    .agent
                    .reconcile_data(scope.request.clone(), &CancellationToken::new())
                    .await
                    .ok()?;
                match BuildCleanupState::try_from(response.state).ok()? {
                    BuildCleanupState::BuildCleanupConfirmed => {
                        return self
                            .executor
                            .admission
                            .release(&scope.operation, &scope.host)
                            .await
                            .ok();
                    }
                    BuildCleanupState::BuildCleanupActive => {
                        tokio::time::sleep(Duration::from_millis(250)).await
                    }
                    _ => return None,
                }
            }
        };
        if matches!(
            tokio::time::timeout(Duration::from_secs(30), cleanup).await,
            Ok(Some(()))
        ) {
            self.state
                .cleanup_uncertain
                .store(false, std::sync::atomic::Ordering::Release);
            Ok(())
        } else {
            self.mark_cleanup_uncertain();
            Err("Backup host cleanup could not be verified. Host and storage capacity remain reserved.".into())
        }
    }
}

/// Core-local Docker work participates in the same durable daemon capacity.
/// A process restart cannot silently clear an unconfirmed local helper.
pub(super) struct LocalDataScope {
    operation: BackupDataOperation,
    host: BackupExecutionHost,
    _permit: citadel_runtime::IoPermit,
}
impl ResticOperation<'_> {
    pub(super) async fn admit_local(
        &self,
        platform: Uuid,
        node: Option<&str>,
        cancellation: &CancellationToken,
    ) -> Result<LocalDataScope, String> {
        let permit = self.local_admission(cancellation).await?;
        let OperationOwner {
            repository,
            run: owner,
            attempt,
        } = self.owner;
        let output = self
            .execute(
                vec!["info".into(), "--format".into(), "{{.ID}}".into()],
                vec![],
                Duration::from_secs(10),
                cancellation,
            )
            .await?;
        let host = BackupExecutionHost {
            daemon_id: String::from_utf8_lossy(&output.stdout).trim().into(),
            authority_id: self.executor.local_authority,
            generation: 1,
            data_slots: 2,
            heavy_slots: 2,
        };
        let operation = BackupDataOperation {
            id: Uuid::now_v7(),
            repository_id: repository,
            owner_id: owner,
            attempt_id: attempt,
            platform_id: platform,
            node_id: node.map(str::to_owned),
        };
        loop {
            if cancellation.is_cancelled() {
                return Err("Backup was cancelled.".into());
            }
            if self
                .executor
                .admission
                .reserve(&operation, &host)
                .await
                .map_err(|e| e.to_string())?
            {
                break;
            }
            if self.defer_if_unstarted() {
                return Err("Waiting for local Docker execution capacity.".into());
            }
            tokio::select! { () = cancellation.cancelled() => return Err("Backup was cancelled.".into()), () = tokio::time::sleep(Duration::from_secs(5)) => {} }
        }
        self.state
            .execution_started
            .store(true, std::sync::atomic::Ordering::Release);
        Ok(LocalDataScope {
            operation,
            host,
            _permit: permit,
        })
    }
    pub(super) async fn finish_local(&self, scope: LocalDataScope) -> Result<(), String> {
        if self.cleanup_is_uncertain() {
            return Err("Local helper cleanup could not be verified. Host and storage capacity remain reserved.".into());
        }
        self.executor
            .admission
            .release(&scope.operation, &scope.host)
            .await
            .map_err(|error| {
                self.mark_cleanup_uncertain();
                error.to_string()
            })
    }
}
