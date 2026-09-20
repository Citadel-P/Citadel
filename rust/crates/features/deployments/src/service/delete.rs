use super::*;

impl DeploymentService {
    pub async fn delete(
        &self,
        actor_id: ActorId,
        administrator: bool,
        ids: Vec<Uuid>,
    ) -> Result<(), DeploymentError> {
        let ids = unique_ids(&ids);
        if ids.is_empty() || ids.iter().any(Uuid::is_nil) {
            return Err(DeploymentError::Validation(
                "Deployment IDs must not be empty.".to_owned(),
            ));
        }
        if ids.len() > 100 {
            return Err(DeploymentError::Validation(
                "At most 100 Deployments can be deleted at once.".to_owned(),
            ));
        }
        let claims = self
            .store
            .claim_delete(actor_id, administrator, &ids)
            .await?;
        let store = Arc::clone(&self.store);
        let runtime = Arc::clone(&self.runtime);
        let notifier = Arc::clone(&self.notifier);
        let shutdown = self.shutdown.clone();
        let timeout = self.delete_timeout;
        let rejected_claims = claims.clone();
        let Some(task) = self.spawn_result("deployment.delete", async move {
            let cancellation = shutdown.child_token();
            let result = tokio::select! {
                biased;
                () = shutdown.cancelled() => Err(DeploymentError::Cancelled),
                result = tokio::time::timeout(
                    timeout,
                    delete_claimed(runtime.as_ref(), &claims, &cancellation),
                ) => result.unwrap_or_else(|_| {
                    Err(DeploymentError::Runtime(
                        "Deployment deletion timed out.".to_owned(),
                    ))
                }),
            };
            cancellation.cancel();
            if let Err(error) = result {
                return Err(release_after_failure(store.as_ref(), &claims, error).await);
            }
            if let Err(error) = store.complete_delete(actor_id, &claims).await {
                return Err(release_after_failure(store.as_ref(), &claims, error).await);
            }
            for claim in &claims {
                notifier.changed(claim.id, "deleted");
            }
            Ok(())
        }) else {
            return Err(release_after_failure(
                self.store.as_ref(),
                &rejected_claims,
                DeploymentError::Cancelled,
            )
            .await);
        };
        task.await.map_err(|error| {
            DeploymentError::Runtime(format!("delete worker failed unexpectedly: {error}"))
        })?
    }
}
async fn release_after_failure(
    store: &dyn DeploymentRepository,
    claims: &[DeletionClaim],
    operation_error: DeploymentError,
) -> DeploymentError {
    match store.release_delete(claims).await {
        Ok(()) => operation_error,
        Err(release_error) => DeploymentError::Storage(format!(
            "{operation_error}; deletion claim release also failed: {release_error}"
        )),
    }
}

async fn delete_claimed(
    runtime: &dyn DeploymentRuntime,
    claims: &[DeletionClaim],
    cancellation: &CancellationToken,
) -> Result<(), DeploymentError> {
    for claim in claims {
        for container_id in &claim.docker_container_ids {
            runtime
                .delete_container(claim.platform_id, container_id, cancellation)
                .await?;
        }
    }
    Ok(())
}
