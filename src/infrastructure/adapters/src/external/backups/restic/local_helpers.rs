use super::*;

impl ResticOperation<'_> {
    pub(super) fn local_scope(
        &self,
    ) -> Result<Option<(BackupDataOperation, BackupExecutionHost)>, String> {
        self.state
            .local_scope
            .lock()
            .map(|v| v.clone())
            .map_err(|_| "Local backup ownership is unavailable.".into())
    }
    // Split create/start. Recovery can delete an immutable ID after registration;
    // a delayed start then fails, and a fenced attempt cannot create another helper.
    pub(super) async fn execute(
        &self,
        mut args: Vec<OsString>,
        env: Vec<(OsString, OsString)>,
        timeout: Duration,
        cancellation: &CancellationToken,
    ) -> Result<citadel_execution::ProcessOutput, String> {
        let Some((operation, _)) = self.local_scope()? else {
            return self.execute_raw(args, env, timeout, cancellation).await;
        };
        let is_run = args.first().is_some_and(|v| v == "run");
        if is_run || args.first().is_some_and(|v| v == "create") {
            let name = args
                .windows(2)
                .find(|v| v[0] == "--name")
                .and_then(|v| v[1].to_str())
                .ok_or("Local helper name is missing.")?
                .to_owned();
            self.executor
                .admission
                .begin_helper(&operation, &name)
                .await
                .map_err(|e| e.to_string())?;
            args[0] = "create".into();
            args.splice(
                1..1,
                [
                    OsString::from("--label"),
                    format!("com.citadel.data-operation={}", operation.id).into(),
                ],
            );
            let created = self
                .execute_raw(
                    args,
                    env,
                    timeout.min(Duration::from_secs(60)),
                    cancellation,
                )
                .await;
            let output = match created {
                Ok(output) => output,
                Err(error) => {
                    self.mark_cleanup_uncertain();
                    return Err(error);
                }
            };
            let id =
                restore_helper_identity(&output).inspect_err(|_| self.mark_cleanup_uncertain())?;
            self.executor
                .admission
                .register_helper(&operation, &name, &id)
                .await
                .map_err(|e| {
                    self.mark_cleanup_uncertain();
                    e.to_string()
                })?;
            self.executor
                .admission
                .authorize_helper_start(&operation, &id)
                .await
                .map_err(|e| e.to_string())?;
            if !is_run {
                return Ok(output);
            }
            return self
                .execute_raw(
                    vec!["start".into(), "--attach".into(), id.into()],
                    vec![],
                    timeout,
                    cancellation,
                )
                .await;
        }
        if args.first().is_some_and(|v| v == "start") {
            let id = args
                .last()
                .and_then(|v| v.to_str())
                .ok_or("Local helper ID is missing.")?;
            self.executor
                .admission
                .authorize_helper_start(&operation, id)
                .await
                .map_err(|e| e.to_string())?;
        }
        self.execute_raw(args, env, timeout, cancellation).await
    }
    pub(super) async fn reconcile_local_helpers(
        &self,
        operation: &BackupDataOperation,
        host: &BackupExecutionHost,
        name: Option<&str>,
    ) -> Result<(), String> {
        let cancel = CancellationToken::new();
        let info = self
            .execute_raw(
                vec!["info".into(), "--format".into(), "{{.ID}}".into()],
                vec![],
                Duration::from_secs(5),
                &cancel,
            )
            .await?;
        if info.stdout_truncated || String::from_utf8_lossy(&info.stdout).trim() != host.daemon_id {
            return Err("Local Docker daemon identity changed. Original helper cleanup must be verified before storage can be released.".into());
        }
        let helpers = self
            .executor
            .admission
            .local_helpers(operation)
            .await
            .map_err(|e| e.to_string())?;
        for helper in helpers {
            if helper.cleaned
                || name.is_some_and(|name| {
                    name != helper.name && Some(name) != helper.container_id.as_deref()
                })
            {
                continue;
            }
            let filter = helper.container_id.as_ref().map_or_else(
                || format!("name=^/{}$", helper.name),
                |id| format!("id={id}"),
            );
            let output = self
                .execute_raw(
                    vec![
                        "ps".into(),
                        "--all".into(),
                        "--quiet".into(),
                        "--no-trunc".into(),
                        "--filter".into(),
                        filter.into(),
                    ],
                    vec![],
                    Duration::from_secs(5),
                    &cancel,
                )
                .await?;
            if output.stdout_truncated {
                return Err("Local helper inspection was incomplete.".into());
            }
            let ids = String::from_utf8_lossy(&output.stdout);
            let ids = ids.lines().filter(|v| !v.is_empty()).collect::<Vec<_>>();
            if ids.is_empty() {
                if helper.container_id.is_none() {
                    return Err("Local helper creation was interrupted before its ID was recorded. Storage remains reserved until creation can be reconciled.".into());
                }
            } else {
                if ids.len() != 1
                    || (helper
                        .container_id
                        .as_deref()
                        .is_some_and(|id| id != ids[0]))
                {
                    return Err("Local helper identity did not match its reservation.".into());
                }
                let id = ids[0];
                let inspect = self
                    .execute_raw(
                        vec![
                            "inspect".into(),
                            "--format".into(),
                            "{{json .Config.Labels}}".into(),
                            id.into(),
                        ],
                        vec![],
                        Duration::from_secs(5),
                        &cancel,
                    )
                    .await?;
                let labels: Value = serde_json::from_slice(&inspect.stdout)
                    .map_err(|_| "Local helper labels could not be verified.")?;
                if inspect.stdout_truncated
                    || labels["com.citadel.data-operation"].as_str()
                        != Some(operation.id.to_string().as_str())
                {
                    return Err(
                        "Local helper belongs to another operation; it was not removed.".into(),
                    );
                }
                // Remember recovered create responses before removing anything.
                if helper.container_id.is_none() {
                    self.executor
                        .admission
                        .register_helper(operation, &helper.name, id)
                        .await
                        .map_err(|e| e.to_string())?;
                }
                self.execute_raw(
                    vec!["rm".into(), "--force".into(), id.into()],
                    vec![],
                    Duration::from_secs(10),
                    &cancel,
                )
                .await?;
            }
            self.executor
                .admission
                .helper_cleaned(operation, &helper.name)
                .await
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
