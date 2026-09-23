use super::*;

impl BuildService {
    pub async fn test_pool(
        self: &Arc<Self>,
        id: Uuid,
        actor: ActorId,
    ) -> Result<BuildAgentPool, BuildError> {
        let pool = self.store.get_pool(id).await?;
        if pool.provider != "SelfManagedVm" {
            return Err(BuildError::Conflict(
                "Only self-managed Citadel Agent build pools can be tested.".into(),
            ));
        }
        pool_target(&pool.provider_spec)?;
        let checker = self.pool_checker.clone().ok_or_else(|| {
            BuildError::Validation("Build Pool capability checks are unavailable.".into())
        })?;
        // The bounded task owns both claim and completion. Dropping the HTTP
        // response cannot leave a successfully claimed pool processing forever.
        let service = self.clone();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        if !self.tasks.spawn(
            "build.pool_test",
            Box::pin(async move {
                let result = service.execute_pool_test(id, actor, checker).await;
                let outcome = result.as_ref().map(|_| ()).map_err(Clone::clone);
                let _ = sender.send(result);
                outcome
            }),
        ) {
            return Err(BuildError::Conflict("Server is shutting down.".into()));
        }
        receiver
            .await
            .map_err(|_| BuildError::Storage("Build Pool check was interrupted.".into()))?
    }

    async fn execute_pool_test(
        &self,
        id: Uuid,
        actor: ActorId,
        checker: Arc<dyn BuildPoolChecker>,
    ) -> Result<BuildAgentPool, BuildError> {
        let service = self;
        let claim = service.store.claim_pool_test(id, actor).await?;
        if let Some(notifier) = &service.pool_change {
            notifier();
        }
        let cancellation = service.shutdown.child_token();
        let checked = tokio::select! {
            biased;
            () = cancellation.cancelled() => Ok(Err(BuildError::Conflict("Server is shutting down.".into()))),
            checked = tokio::time::timeout(
            std::time::Duration::from_secs(120),
            checker.check(&claim, &cancellation),
        ) => checked,
        };
        let result = match checked {
            Ok(Ok(result)) => result,
            Ok(Err(error)) => BuildPoolCheck {
                ready: false,
                message: format!("Citadel Agent build capability check failed: {error}"),
            },
            Err(_) => {
                cancellation.cancel();
                BuildPoolCheck {
                    ready: false,
                    message: "Citadel Agent build capability check timed out.".into(),
                }
            }
        };
        let finished = service.store.finish_pool_test(&claim, &result).await;
        if let Some(notifier) = &service.pool_change {
            notifier();
        }
        finished
    }
}
