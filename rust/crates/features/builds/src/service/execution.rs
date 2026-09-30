use super::*;
use crate::runs::logs;

impl BuildService {
    pub async fn process_one(&self, shutdown: &CancellationToken) -> Result<bool, BuildError> {
        let Some(claim) = self.store.claim_next(Utc::now() - self.stale_after).await? else {
            return Ok(false);
        };
        self.changed();
        let cancellation = shutdown.child_token();
        self.active
            .lock()
            .map_err(|_| BuildError::Storage("Build cancellation state is poisoned.".to_owned()))?
            .insert(claim.run.id, cancellation.clone());
        let logs =
            logs::PersistedBuildLogs::new(self.store.clone(), claim.run.id, self.on_log.clone());
        let execution = async {
            if let Err(error) = self
                .ensure_execution_entitlements(&claim.project, &claim.run.trigger)
                .await
            {
                return BuildExecutionResult {
                    status: crate::BuildRunStatus::Failed,
                    exit_code: None,
                    image_digest: None,
                    resolved_commit_sha: None,
                    image_references: vec![],
                    error_code: Some("build.entitlement".into()),
                    error_message: Some(error.to_string()),
                    logs: vec![],
                };
            }
            self.executor.execute(&claim, &logs, &cancellation).await
        };
        tokio::pin!(execution);
        let result = tokio::select! {
            result = &mut execution => result,
            () = tokio::time::sleep(std::time::Duration::from_secs(claim.run.timeout_seconds as u64)) => {
                cancellation.cancel();
                // Executors must observe cancellation and finish child cleanup before the
                // claim is persisted as terminal. This prevents detached Docker/Git work.
                let _ = execution.await;
                BuildExecutionResult {
                    status: crate::BuildRunStatus::TimedOut,
                    exit_code: None,
                    image_digest: None,
                    resolved_commit_sha: None,
                    image_references: vec![],
                    error_code: Some("build.timeout".to_owned()),
                    error_message: Some("Build Run exceeded its configured timeout.".to_owned()),
                    logs: vec![BuildLog {
                        stream: "stderr".to_owned(),
                        message: "Build Run exceeded its configured timeout.".to_owned(),
                    }],
                }
            }
        };
        if let Ok(mut active) = self.active.lock() {
            active.remove(&claim.run.id);
        }
        if !self.store.finish(&claim, &result).await? {
            return Ok(true);
        }
        self.changed();
        if matches!(
            result.status,
            crate::BuildRunStatus::Failed | crate::BuildRunStatus::TimedOut
        ) && let Some(alerts) = &self.alerts
        {
            let message = result
                .error_message
                .as_deref()
                .unwrap_or("Build Run failed.");
            let observation = AlertObservation {
                alert_type: "BuildRunFailed".to_owned(),
                info: serde_json::json!({
                    "RunId": claim.run.id,
                    "Trigger": claim.run.trigger,
                    "Status": result.status,
                    "ExitCode": result.exit_code,
                    "ErrorMessage": message,
                    "HumanMessage": message,
                }),
                resource_id: claim.project.id,
                resource_name: claim.project.name.clone(),
                resource_type: "Build".to_owned(),
                deduplication_component: claim.run.id.to_string(),
                observed_at: Utc::now(),
                value: None,
                matched: true,
            };
            if let Err(error) = alerts.observe(&observation).await {
                tracing::error!(%error, run_id=%claim.run.id, "Build failure alert evaluation failed");
            }
        }
        Ok(true)
    }

    pub async fn cancel(&self, id: Uuid) -> Result<(), BuildError> {
        if let Some(token) = self
            .active
            .lock()
            .map_err(|_| BuildError::Storage("Build cancellation state is poisoned.".to_owned()))?
            .get(&id)
            .cloned()
        {
            token.cancel();
            return Ok(());
        }
        if self.store.cancel_queued(id).await? {
            Ok(())
        } else {
            Err(BuildError::Conflict("Build Run is not active.".to_owned()))
        }
    }
}
