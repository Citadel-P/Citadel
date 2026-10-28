use super::*;
impl ScriptService {
    pub(super) async fn execute_claim(
        &self,
        claim: &ScriptRunClaim,
        cancellation: &CancellationToken,
        progress: Option<&tokio::sync::broadcast::Sender<ScriptProgress>>,
    ) -> Result<(), ScriptError> {
        // Verify before registering local state or starting external work.
        let cancelled = self.store.cancellation_requested(claim.run.id).await?;
        self.changed(Some(claim.run.script_id));
        let run_cancellation = cancellation.child_token();
        self.active_runs
            .lock()
            .map_err(|_| ScriptError::Storage("Script cancellation state is poisoned.".to_owned()))?
            .insert(claim.run.id, run_cancellation.clone());
        if let Some(sender) = progress {
            progress::send(
                sender,
                ScriptProgress::state(&claim.run, crate::ScriptRunStatus::Running),
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
            RemoteScriptOutcome::Completed(result) => result,
            RemoteScriptOutcome::Deferred if run_cancellation.is_cancelled() => {
                ScriptRunResult::cancelled()
            }
            RemoteScriptOutcome::Deferred => {
                let status = self.store.defer(claim).await?;
                self.changed(Some(claim.run.script_id));
                let _ = self
                    .progress
                    .send(ScriptProgress::state(&claim.run, status));
                return Ok(());
            }
        };
        if self.store.finish(claim, &result).await? {
            self.changed(Some(claim.run.script_id));
            self.raise_failure_alert(claim, &result).await;
            if let Some(sender) = progress {
                let _ = sender.send(ScriptProgress::completed(&claim.run, &result));
            }
        }
        Ok(())
    }
    pub async fn cancel(&self, script_id: Uuid, run_id: Uuid) -> Result<(), ScriptError> {
        // Persist first; an in-memory miss can be the instant between claiming
        // the queue row and registering its local cancellation token.
        self.store.cancel(script_id, run_id).await?;
        if let Some(token) = self
            .active_runs
            .lock()
            .map_err(|_| ScriptError::Storage("Script cancellation state is poisoned.".to_owned()))?
            .get(&run_id)
        {
            token.cancel();
        }
        self.changed(Some(script_id));
        Ok(())
    }
    pub(super) async fn execute(
        &self,
        claim: &ScriptRunClaim,
        cancellation: &CancellationToken,
        progress: Option<&tokio::sync::broadcast::Sender<ScriptProgress>>,
    ) -> RemoteScriptOutcome {
        if cancellation.is_cancelled() {
            return ScriptRunResult::cancelled().into();
        }
        if let Err(error) = self.options.validate_execution(claim.run.timeout_seconds) {
            return ScriptRunResult::failed(None, error.to_string()).into();
        }
        if (matches!(claim.run.trigger.as_str(), "Schedule" | "Webhook")
            || matches!(claim.run.executor, ScriptExecutor::Automatic))
            && let Err(error) = self.ensure_automated_operations().await
        {
            return ScriptRunResult::failed(None, error.to_string()).into();
        }
        if claim.run.code_snapshot.is_none() {
            return ScriptRunResult::failed(None, "Execution claim has no source code.".into())
                .into();
        }
        let Some(attempt_id) = claim.run.attempt_id else {
            return ScriptRunResult::failed(None, "Script claim has no attempt identity.".into())
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
                return ScriptRunResult::failed(
                    None,
                    format!("Run-as authorization failed: {error}"),
                )
                .into();
            }
        };
        let local = matches!(claim.run.executor, ScriptExecutor::Core);
        let base_url = if local {
            &self.internal_base_url
        } else {
            &self.remote_base_url
        };
        let script = crate::sandbox::ScriptExecution {
            run_id: claim.run.id,
            source: script_source(base_url, &token, &claim.run, &self.endpoint_catalog_json),
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
            return ScriptRunResult::failed(None, "Agent script execution is unavailable.".into())
                .into();
        };
        match remote
            .execute(claim, script, &self.progress, cancellation)
            .await
        {
            Ok(result) => result,
            Err(error) => ScriptRunResult::failed(None, error.to_string()).into(),
        }
    }
}

impl ScriptService {
    pub(super) async fn raise_failure_alert(
        &self,
        claim: &ScriptRunClaim,
        result: &ScriptRunResult,
    ) {
        if claim.run.trigger == "Test"
            || !matches!(
                result.status,
                crate::ScriptRunStatus::Failed | crate::ScriptRunStatus::TimedOut
            )
        {
            return;
        }
        let Some(alerts) = &self.alerts else {
            return;
        };
        let script = match self.store.get(claim.run.script_id).await {
            Ok(script) if script.alert_on_failure => script,
            Ok(_) => return,
            Err(error) => {
                tracing::error!(%error, run_id=%claim.run.id, "Script alert policy lookup failed");
                return;
            }
        };
        let message = result.error.as_deref().unwrap_or("Script run failed.");
        let observation = AlertObservation {
            alert_type: "ScriptRunFailed".to_owned(),
            info: serde_json::json!({
                "RunId": claim.run.id,
                "Trigger": claim.run.trigger,
                "Status": result.status,
                "ExitCode": result.exit_code,
                "ErrorMessage": message,
                "HumanMessage": message,
            }),
            resource_id: script.id,
            resource_name: script.name,
            resource_type: "Script".to_owned(),
            deduplication_component: claim.run.id.to_string(),
            observed_at: Utc::now(),
            value: None,
            matched: true,
        };
        if let Err(error) = alerts.observe(&observation).await {
            tracing::error!(%error, run_id=%claim.run.id, "Script failure alert evaluation failed");
        }
    }
}
