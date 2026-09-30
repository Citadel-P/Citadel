use super::*;
impl AutomationService {
    pub(super) async fn execute_claim(
        &self,
        claim: &AutomationRunClaim,
        cancellation: &CancellationToken,
        progress: Option<&mpsc::Sender<AutomationProgress>>,
    ) -> Result<(), AutomationError> {
        self.changed();
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
        let result = self.execute(claim, &run_cancellation, progress).await;
        if let Ok(mut active) = self.active_runs.lock() {
            active.remove(&claim.run.id);
        }
        if self.store.finish(claim, &result).await? {
            self.changed();
            self.raise_failure_alert(claim, &result).await;
            if let Some(sender) = progress {
                // Preserve the terminal item even when the bounded queue is full.
                // An abandoned viewer must not hold this task or its slot forever.
                let _ = tokio::time::timeout(
                    Duration::from_secs(5),
                    sender.send(AutomationProgress::completed(&claim.run, &result)),
                )
                .await;
            }
        }
        Ok(())
    }
    pub async fn cancel(&self, action_id: Uuid, run_id: Uuid) -> Result<(), AutomationError> {
        let token = self
            .active_runs
            .lock()
            .map_err(|_| {
                AutomationError::Storage("Automation cancellation state is poisoned.".to_owned())
            })?
            .get(&run_id)
            .cloned();
        if let Some(token) = token {
            let run = self.store.get_run(action_id, run_id).await?;
            if run.status != crate::AutomationRunStatus::Running {
                return Err(AutomationError::Conflict(
                    "Automation run is not active.".to_owned(),
                ));
            }
            token.cancel();
            return Ok(());
        }
        self.store.cancel(action_id, run_id).await
    }
    pub(super) async fn execute(
        &self,
        claim: &AutomationRunClaim,
        cancellation: &CancellationToken,
        progress: Option<&mpsc::Sender<AutomationProgress>>,
    ) -> AutomationRunResult {
        if let Err(error) = self.options.validate_execution(claim.run.timeout_seconds) {
            return AutomationRunResult::failed(None, error.to_string());
        }
        if matches!(claim.run.trigger.as_str(), "Schedule" | "Webhook")
            && let Err(error) = self.ensure_paid_trigger().await
        {
            return AutomationRunResult::failed(None, error.to_string());
        }
        if claim.run.code_snapshot.is_none() {
            return AutomationRunResult::failed(None, "Execution claim has no source code.".into());
        }
        let token = match self
            .token_issuer
            .issue(
                ActorId::new(claim.run.run_as_actor_id),
                claim.run.id,
                Duration::from_secs(claim.run.timeout_seconds as u64 + 60),
            )
            .await
        {
            Ok(token) => token,
            Err(error) => {
                return AutomationRunResult::failed(
                    None,
                    format!("Run-as authorization failed: {error}"),
                );
            }
        };
        let directory = self.work_root.join(claim.run.id.to_string());
        if let Some(cache) = &self.cache_directory
            && let Err(error) = tokio::fs::create_dir_all(cache).await
        {
            return AutomationRunResult::failed(
                None,
                format!("Could not prepare Deno cache: {error}"),
            );
        }
        if let Err(error) = tokio::fs::create_dir_all(&directory).await {
            return AutomationRunResult::failed(None, format!("Could not prepare run: {error}"));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // The script contains a short-lived credential. Restrict the run
            // directory before writing it, regardless of the process umask.
            if let Err(error) =
                tokio::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).await
            {
                let _ = tokio::fs::remove_dir(&directory).await;
                return AutomationRunResult::failed(
                    None,
                    format!("Could not secure run directory: {error}"),
                );
            }
        }
        let script = directory.join("action.ts");
        let source = automation_source(
            &self.internal_base_url,
            &token,
            &claim.run,
            &self.endpoint_catalog_json,
        );
        if let Err(error) = tokio::fs::write(&script, source.as_bytes()).await {
            let _ = tokio::fs::remove_dir_all(&directory).await;
            return AutomationRunResult::failed(None, format!("Could not write action: {error}"));
        }
        let mut args = vec![
            OsString::from("run"),
            OsString::from("--no-prompt"),
            OsString::from(format!("--allow-read={}", directory.display())),
            OsString::from(format!("--allow-write={}", directory.display())),
            OsString::from("--allow-env=NO_COLOR,DENO_DIR"),
        ];
        let allow_net = self
            .allow_net
            .as_deref()
            .unwrap_or_else(|| allow_net_authority(&self.internal_base_url))
            .trim();
        if !allow_net.is_empty() {
            args.push(format!("--allow-net={allow_net}").into());
        }
        args.push(script.as_os_str().to_owned());
        let mut request = ProcessRequest::new(self.deno_path.clone())
            .args(args)
            .current_dir(&directory)
            .env("NO_COLOR", "1")
            .env("CITADEL_ACTION_ARGS", &claim.run.args_json)
            .limits(ProcessLimits {
                timeout: Duration::from_secs(claim.run.timeout_seconds as u64),
                maximum_stdout_bytes: self.maximum_log_bytes,
                maximum_stderr_bytes: self.maximum_log_bytes,
                output_limit_policy: OutputLimitPolicy::Truncate,
            });
        if let Some(cache) = &self.cache_directory {
            request = request.env("DENO_DIR", cache.as_os_str());
        }
        let output = if let Some(sender) = progress {
            let (chunks, mut receiver) = mpsc::channel(8);
            let consume = async {
                let mut stdout = Vec::new();
                let mut stderr = Vec::new();
                while let Some(chunk) = receiver.recv().await {
                    let chunk: citadel_execution::ProcessChunk = chunk;
                    let pending = if chunk.stream == "stderr" {
                        &mut stderr
                    } else {
                        &mut stdout
                    };
                    pending.extend_from_slice(&chunk.bytes);
                    if let Some(end) = pending.iter().rposition(|byte| *byte == b'\n') {
                        let text =
                            redact_run_logs(&String::from_utf8_lossy(&pending[..=end]), &token);
                        pending.drain(..=end);
                        progress::send(
                            sender,
                            AutomationProgress {
                                run_id: Some(claim.run.id),
                                stream: Some(text),
                                ..Default::default()
                            },
                            cancellation,
                        )
                        .await;
                    }
                }
                for pending in [stdout, stderr] {
                    if !pending.is_empty() {
                        let text = redact_run_logs(&String::from_utf8_lossy(&pending), &token);
                        progress::send(
                            sender,
                            AutomationProgress {
                                run_id: Some(claim.run.id),
                                stream: Some(text),
                                ..Default::default()
                            },
                            cancellation,
                        )
                        .await;
                    }
                }
            };
            let (output, ()) = tokio::join!(
                self.process.run(request.output(chunks), cancellation),
                consume
            );
            output
        } else {
            self.process.run(request, cancellation).await
        };
        let _ = tokio::fs::remove_dir_all(&directory).await;
        match output {
            Ok(output) => {
                let logs = redact_run_logs(
                    &joined_logs(
                        &output.stdout,
                        &output.stderr,
                        output.stdout_truncated || output.stderr_truncated,
                    ),
                    &token,
                );
                if output.succeeded() {
                    AutomationRunResult::success(output.exit_code, logs)
                } else {
                    let mut result = AutomationRunResult::failed(output.exit_code, logs);
                    result.error = Some(format!(
                        "Deno exited with code {}.",
                        output
                            .exit_code
                            .map_or_else(|| "unknown".into(), |code| code.to_string())
                    ));
                    result
                }
            }
            Err(ProcessError::Cancelled) => AutomationRunResult::cancelled(),
            Err(ProcessError::Timeout(_)) => AutomationRunResult::timed_out(),
            Err(error) => AutomationRunResult::failed(None, error.to_string()),
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
