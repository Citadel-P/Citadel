use super::*;
impl AutomationService {
    pub(super) async fn execute_claim(
        &self,
        claim: &AutomationRunClaim,
        cancellation: &CancellationToken,
        progress: Option<&tokio::sync::broadcast::Sender<AutomationProgress>>,
    ) -> Result<(), AutomationError> {
        // Verify before registering local state or starting external work.
        let cancelled = self.store.cancellation_requested(claim.run.id).await?;
        self.changed(Some(claim.run.action_id));
        let run_cancellation = cancellation.child_token();
        self.active_runs
            .lock()
            .map_err(|_| {
                AutomationError::Storage("Automation cancellation state is poisoned.".to_owned())
            })?
            .insert(claim.run.id, run_cancellation.clone());
        if let Some(sender) = progress {
            progress::send(
                sender,
                AutomationProgress::state(&claim.run, crate::AutomationRunStatus::Running),
                &run_cancellation,
            )
            .await;
        }
        // Do not start a process when cancellation won the claim race.
        if cancelled {
            run_cancellation.cancel();
        }
        let execution = self.execute(claim, &run_cancellation, progress);
        tokio::pin!(execution);
        let mut check = tokio::time::interval(Duration::from_secs(1));
        check.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let result = loop {
            tokio::select! {
                result = &mut execution => break result,
                _ = check.tick(), if !run_cancellation.is_cancelled() => {
                    // Keep process output/deadline handling polled while the DB
                    // is slow. A failed check does not replay or drop execution.
                    let requested = self.store.cancellation_requested(claim.run.id);
                    tokio::pin!(requested);
                    tokio::select! {
                        result = &mut execution => break result,
                        value = tokio::time::timeout(Duration::from_secs(5), &mut requested) => {
                            if matches!(value, Ok(Ok(true))) { run_cancellation.cancel(); }
                        }
                    }
                }
            }
        };
        if let Ok(mut active) = self.active_runs.lock() {
            active.remove(&claim.run.id);
        }
        let result = match result {
            RemoteAutomationOutcome::Completed(result) => result,
            RemoteAutomationOutcome::Deferred if run_cancellation.is_cancelled() => {
                AutomationRunResult::cancelled()
            }
            RemoteAutomationOutcome::Deferred => {
                let status = self.store.defer(claim).await?;
                self.changed(Some(claim.run.action_id));
                let _ = self
                    .progress
                    .send(AutomationProgress::state(&claim.run, status));
                return Ok(());
            }
        };
        if self.store.finish(claim, &result).await? {
            self.changed(Some(claim.run.action_id));
            self.raise_failure_alert(claim, &result).await;
            if let Some(sender) = progress {
                let _ = sender.send(AutomationProgress::completed(&claim.run, &result));
            }
        }
        Ok(())
    }
    pub async fn cancel(&self, action_id: Uuid, run_id: Uuid) -> Result<(), AutomationError> {
        // Persist first; an in-memory miss can be the instant between claiming
        // the queue row and registering its local cancellation token.
        self.store.cancel(action_id, run_id).await?;
        if let Some(token) = self
            .active_runs
            .lock()
            .map_err(|_| {
                AutomationError::Storage("Automation cancellation state is poisoned.".to_owned())
            })?
            .get(&run_id)
        {
            token.cancel();
        }
        self.changed(Some(action_id));
        Ok(())
    }
    pub(super) async fn execute(
        &self,
        claim: &AutomationRunClaim,
        cancellation: &CancellationToken,
        progress: Option<&tokio::sync::broadcast::Sender<AutomationProgress>>,
    ) -> RemoteAutomationOutcome {
        if cancellation.is_cancelled() {
            return AutomationRunResult::cancelled().into();
        }
        if let Err(error) = self.options.validate_execution(claim.run.timeout_seconds) {
            return AutomationRunResult::failed(None, error.to_string()).into();
        }
        if (matches!(claim.run.trigger.as_str(), "Schedule" | "Webhook")
            || matches!(claim.run.executor, AutomationExecutor::Automatic))
            && let Err(error) = self.ensure_automated_operations().await
        {
            return AutomationRunResult::failed(None, error.to_string()).into();
        }
        if claim.run.code_snapshot.is_none() {
            return AutomationRunResult::failed(None, "Execution claim has no source code.".into())
                .into();
        }
        let Some(attempt_id) = claim.run.attempt_id else {
            return AutomationRunResult::failed(
                None,
                "Automation claim has no attempt identity.".into(),
            )
            .into();
        };
        let token = match self
            .token_issuer
            .issue(
                ActorId::new(claim.run.run_as_actor_id),
                claim.run.id,
                attempt_id,
                Duration::from_secs(claim.run.timeout_seconds as u64 + 60),
            )
            .await
        {
            Ok(token) => token,
            Err(error) => {
                return AutomationRunResult::failed(
                    None,
                    format!("Run-as authorization failed: {error}"),
                )
                .into();
            }
        };
        let local = matches!(claim.run.executor, AutomationExecutor::Core);
        let base_url = if local {
            &self.internal_base_url
        } else {
            &self.remote_base_url
        };
        let script = crate::sandbox::AutomationScript {
            run_id: claim.run.id,
            source: automation_source(base_url, &token, &claim.run, &self.endpoint_catalog_json),
            token,
            args_json: claim.run.args_json.clone(),
            base_url: base_url.clone(),
            timeout_seconds: claim.run.timeout_seconds,
            maximum_log_bytes: self.maximum_log_bytes,
        };
        if local {
            return self
                .sandbox
                .execute(script, cancellation, progress)
                .await
                .into();
        }
        let Some(remote) = &self.remote else {
            return AutomationRunResult::failed(
                None,
                "Agent automation execution is unavailable.".into(),
            )
            .into();
        };
        match remote
            .execute(claim, script, &self.progress, cancellation)
            .await
        {
            Ok(result) => result,
            Err(error) => AutomationRunResult::failed(None, error.to_string()).into(),
        }
    }
}

impl AutomationService {
    pub(super) async fn raise_failure_alert(
        &self,
        claim: &AutomationRunClaim,
        result: &AutomationRunResult,
    ) {
        if claim.run.trigger == "Test"
            || !matches!(
                result.status,
                crate::AutomationRunStatus::Failed | crate::AutomationRunStatus::TimedOut
            )
        {
            return;
        }
        let Some(alerts) = &self.alerts else {
            return;
        };
        let action = match self.store.get(claim.run.action_id).await {
            Ok(action) if action.alert_on_failure => action,
            Ok(_) => return,
            Err(error) => {
                tracing::error!(%error, run_id=%claim.run.id, "Automation alert policy lookup failed");
                return;
            }
        };
        let message = result.error.as_deref().unwrap_or("Automation run failed.");
        let observation = AlertObservation {
            alert_type: "AutomationActionRunFailed".to_owned(),
            info: serde_json::json!({
                "RunId": claim.run.id,
                "Trigger": claim.run.trigger,
                "Status": result.status,
                "ExitCode": result.exit_code,
                "ErrorMessage": message,
                "HumanMessage": message,
            }),
            resource_id: action.id,
            resource_name: action.name,
            resource_type: "AutomationAction".to_owned(),
            deduplication_component: claim.run.id.to_string(),
            observed_at: Utc::now(),
            value: None,
            matched: true,
        };
        if let Err(error) = alerts.observe(&observation).await {
            tracing::error!(%error, run_id=%claim.run.id, "Automation failure alert evaluation failed");
        }
    }
}
