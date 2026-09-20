use super::*;
impl AutomationService {
    pub async fn run(
        self: &Arc<Self>,
        actor: ActorId,
        id: Uuid,
        trigger: &str,
        args: &Value,
        timeout_seconds: Option<i32>,
        code: Option<&str>,
    ) -> Result<mpsc::Receiver<AutomationProgress>, AutomationError> {
        if let Some(code) = code
            && (trigger != "Test" || code.trim().is_empty() || code.len() > 256 * 1024)
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
        let permit = self.slots.clone().try_acquire_owned().map_err(|_| {
            AutomationError::Conflict(
                "Automation execution slots are busy. Try again later.".into(),
            )
        })?;
        let service = self.clone();
        let trigger = trigger.to_owned();
        let args = args.clone();
        let code = code.map(str::to_owned);
        let (started, ready) = tokio::sync::oneshot::channel();
        // Admission precedes the durable claim. Once admitted, the process owns
        // claim completion even if the request disappears during the DB call.
        if !self.tasks.spawn(
            "automation.direct_run",
            Box::pin(async move {
                let _permit = permit;
                let claim = match service
                    .store
                    .enqueue_for_execution(
                        actor,
                        id,
                        &trigger,
                        &args,
                        timeout_seconds,
                        code.as_deref(),
                    )
                    .await
                {
                    Ok(claim) => claim,
                    Err(error) => {
                        let _ = started.send(Err(error));
                        return Ok(());
                    }
                };
                let (sender, receiver) = mpsc::channel(16);
                let _ = sender.try_send(AutomationProgress::state(&claim.run, "Queued"));
                let _ = started.send(Ok(receiver));
                let cancellation = service.shutdown.child_token();
                let execution = service.execute_claim(&claim, &cancellation, Some(&sender));
                tokio::pin!(execution);
                let outcome = tokio::select! {
                    value = &mut execution => value,
                    () = sender.closed() => {
                        cancellation.cancel();
                        execution.await
                    }
                };
                if outcome.is_err() {
                    // Recovery owns a failed persistence attempt; never report success.
                    progress::send(
                        &sender,
                        AutomationProgress::completed(
                            &claim.run,
                            &AutomationRunResult::failed(
                                None,
                                "Automation completion could not be persisted.".into(),
                            ),
                        ),
                        &cancellation,
                    )
                    .await;
                }
                outcome
            }),
        ) {
            return Err(AutomationError::Conflict("Server is shutting down.".into()));
        }
        ready
            .await
            .map_err(|_| AutomationError::Storage("Automation admission was interrupted.".into()))?
    }
}
