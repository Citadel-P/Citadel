use super::*;

impl AutomationService {
    pub async fn run(
        self: &Arc<Self>,
        actor: ActorId,
        id: Uuid,
        trigger: &str,
        args: &Value,
        timeout_seconds: Option<i32>,
        draft: Option<&AutomationDraft>,
    ) -> Result<mpsc::Receiver<AutomationProgress>, AutomationError> {
        if let Some(draft) = draft
            && (trigger != "Test" || draft.code.trim().is_empty() || draft.code.len() > 256 * 1024)
        {
            return Err(AutomationError::Validation(
                "Draft code is only accepted for tests and must contain between 1 byte and 256 KiB.".into(),
            ));
        }
        let configured = match timeout_seconds {
            Some(seconds) => seconds,
            None => self.store.get(id).await?.timeout_seconds,
        };
        self.options.validate_execution(configured)?;
        let service = self.clone();
        let trigger = trigger.to_owned();
        let args = args.clone();
        let draft = draft.cloned();
        let (started, ready) = tokio::sync::oneshot::channel();
        // The process owns admission even if HTTP disappears during the commit.
        // Execution belongs to the queue workers, not to this progress viewer.
        if !self.tasks.spawn(
            "automation.progress",
            Box::pin(async move {
                let mut updates = service.progress.subscribe();
                let run = match service
                    .store
                    .enqueue_with_draft(actor, id, &trigger, &args, timeout_seconds, draft.as_ref())
                    .await
                {
                    Ok(run) => run,
                    Err(error) => {
                        let _ = started.send(Err(error));
                        return Ok(());
                    }
                };
                service.changed(Some(id));
                let (sender, receiver) = mpsc::channel(16);
                let _ = sender.try_send(AutomationProgress::state(&run, AutomationRunStatus::Queued));
                let _ = started.send(Ok(receiver));
                // A terminal database read also covers queued cancellation and a
                // completion that raced admission or overflowed the progress ring.
                let mut refresh = tokio::time::interval(Duration::from_secs(2));
                loop {
                    tokio::select! {
                        () = service.shutdown.cancelled() => break,
                        () = sender.closed() => break,
                        update = updates.recv() => match update {
                            Ok(item) if item.run_id == Some(run.id) => {
                                let terminal = item.status.is_some_and(is_terminal);
                                if terminal {
                                    let _ = tokio::time::timeout(Duration::from_secs(5), sender.send(item)).await;
                                    break;
                                }
                                // Logs remain available on the persisted run. A
                                // slow viewer must not backpressure the executor.
                                let _ = sender.try_send(item);
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                            _ => {}
                        },
                        _ = refresh.tick() => {
                            let current = service.store.get_run(id, run.id).await?;
                            if is_terminal(current.status) {
                                let result = AutomationRunResult {
                                    status: current.status,
                                    exit_code: current.exit_code,
                                    logs: current.logs.unwrap_or_default(),
                                    error: current.error_message,
                                };
                                let _ = tokio::time::timeout(Duration::from_secs(5), sender.send(AutomationProgress::completed(&run, &result))).await;
                                break;
                            }
                        }
                    }
                }
                Ok(())
            }),
        ) {
            return Err(AutomationError::Conflict("Server is shutting down.".into()));
        }
        ready
            .await
            .map_err(|_| AutomationError::Storage("Automation admission was interrupted.".into()))?
    }
}

fn is_terminal(status: AutomationRunStatus) -> bool {
    !matches!(
        status,
        AutomationRunStatus::Queued | AutomationRunStatus::Running
    )
}
