use super::*;
impl GitRepositoryExecutionService {
    pub async fn request_sync(
        &self,
        actor_id: ActorId,
        id: Uuid,
        branch: Option<&str>,
    ) -> Result<(), GitRepositoryExecutionError> {
        if let Some(branch) = branch {
            validate_branch_input(branch)?;
        }
        self.store.enqueue_sync(actor_id, id, branch).await
    }

    /// Reuse the durable repository worker rather than fetching concurrently
    /// into its cache. The caller's cancellation stops waiting, not other
    /// consumers of the same repository synchronization.
    pub async fn synchronize_commit(
        &self,
        actor: ActorId,
        id: Uuid,
        branch: &str,
        cancellation: &CancellationToken,
    ) -> Result<String, GitRepositoryExecutionError> {
        validate_branch_input(branch)?;
        if cancellation.is_cancelled() {
            return Err(GitError::Process(citadel_execution::ProcessError::Cancelled).into());
        }
        // Subscribe before enqueue/read: completion racing either await must
        // remain visible. One coalescing channel serves all refs and waiters.
        let completion = self.completion.subscribe();
        // Enqueue changes Healthy/Failed to Pending under the repository lock,
        // and a request arriving during Syncing causes another Pending pass.
        let wait = async {
            self.store.enqueue_apply(actor, id, branch).await?;
            self.changed();
            completion::wait(completion, || self.store.get_ref(id, branch)).await
        };
        completion::bounded(cancellation, wait).await
    }

    /// Read a completed branch observation without scheduling network work.
    pub async fn synchronized_ref(
        &self,
        id: Uuid,
        branch: &str,
    ) -> Result<GitRepositoryRef, GitRepositoryExecutionError> {
        validate_branch_input(branch)?;
        let reference = self
            .store
            .get_ref(id, branch)
            .await?
            .filter(|reference| reference.status == crate::GitRepositoryRefStatus::Healthy)
            .ok_or(GitRepositoryExecutionError::NotSynchronized)?;
        if !reference
            .resolved_commit_sha
            .as_deref()
            .is_some_and(is_full_object_id)
        {
            return Err(GitRepositoryExecutionError::NotSynchronized);
        }
        Ok(reference)
    }

    pub async fn list_refs(
        &self,
        id: Uuid,
    ) -> Result<Vec<GitRepositoryRef>, GitRepositoryExecutionError> {
        self.store.list_refs(id).await
    }

    pub async fn discover_branches(
        &self,
        id: Uuid,
        cancellation: &CancellationToken,
    ) -> Result<Vec<RemoteBranch>, GitRepositoryExecutionError> {
        let source = self.store.get_source(id).await?;
        let remote = self.prepare_remote(&source).await?;
        self.cli
            .list_remote_branches_with_environment(&remote.url, &remote.environment, cancellation)
            .await
            .map_err(Into::into)
    }

    pub async fn process_one(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<bool, GitRepositoryExecutionError> {
        let stale_before = Utc::now()
            - chrono::Duration::from_std(self.stale_after)
                .map_err(|error| GitRepositoryExecutionError::Storage(error.to_string()))?;
        let Some(claim) = self.store.claim_next(stale_before).await? else {
            return Ok(false);
        };
        self.changed();
        let result = self.synchronize_claim(&claim, cancellation).await;
        match result {
            Ok(result) => self.store.complete(&claim, &result).await?,
            Err(error) => {
                let message = if claim.repository.has_hooks() {
                    bounded_error(&format!(
                        "{error}. Review hook side effects and run Sync manually before retrying."
                    ))
                } else {
                    bounded_error(&error.to_string())
                };
                self.store.fail(&claim, &message).await?;
            }
        }
        // Persistence has committed before any waiter is awakened. A failed
        // commit emits no success hint; the coarse fallback still recovers.
        self.completion.send_modify(|_| {});
        self.changed();
        Ok(true)
    }

    pub async fn enqueue_due(&self, limit: usize) -> Result<usize, GitRepositoryExecutionError> {
        let count = self.store.enqueue_due(limit.clamp(1, 100)).await?;
        if count > 0 {
            self.changed();
        }
        Ok(count)
    }

    pub async fn receive_webhook(
        &self,
        actor_id: ActorId,
        id: Uuid,
        auth_type: &str,
        headers: &[(String, String)],
        body: &[u8],
    ) -> Result<GitWebhookOutcome, GitRepositoryExecutionError> {
        if body.len() > 1024 * 1024 {
            return Err(GitRepositoryExecutionError::Validation(
                "Webhook payload exceeds the 1 MiB limit.".to_owned(),
            ));
        }
        let original_webhook = self
            .store
            .get_webhook(id)
            .await?
            .filter(|configuration| configuration.enabled)
            .ok_or(GitRepositoryExecutionError::NotFound)?;
        let mut webhook = original_webhook.clone();
        let source = self.store.get_source(id).await?;
        if webhook
            .branch_filter
            .as_deref()
            .is_none_or(|value| value.trim().is_empty())
        {
            webhook.branch_filter = Some(source.default_branch.clone());
        }
        if let Some(reason) =
            crate::repositories::webhooks::evaluate_webhook(&webhook, auth_type, headers, body)
                .map_err(map_webhook_error)?
        {
            return Ok(GitWebhookOutcome::Ignored { reason });
        }
        let payload = serde_json::from_slice(body).unwrap_or(serde_json::Value::Null);
        if !repository_matches(&source.url, &payload) {
            return Ok(GitWebhookOutcome::Ignored {
                reason: "Repository identity mismatch",
            });
        }
        let (_, payload_branch) =
            webhook_branch(webhook.provider.as_str(), headers, body).map_err(map_webhook_error)?;
        let branch = payload_branch
            .as_deref()
            .or(webhook.branch_filter.as_deref());
        self.store
            .enqueue_webhook(
                actor_id,
                id,
                branch.unwrap_or(&source.default_branch),
                &original_webhook,
            )
            .await?;
        self.changed();
        Ok(GitWebhookOutcome::Queued {
            branch: branch.unwrap_or(&source.default_branch).to_owned(),
        })
    }

    pub(super) async fn synchronize_claim(
        &self,
        claim: &GitSyncClaim,
        cancellation: &CancellationToken,
    ) -> Result<SyncResult, GitRepositoryExecutionError> {
        let remote = self.prepare_remote(&claim.repository).await?;
        let cache = self.cache_path(claim.repository.id);
        // Keep a failed initial hook from publishing a cache that would skip On Clone on retry.
        let mut staging = if claim
            .repository
            .on_clone
            .as_ref()
            .is_some_and(|hook| !hook.commands.is_empty())
            && !self
                .cli
                .workspace
                .exists(&cache)
                .await
                .map_err(storage_error)?
        {
            Some(
                self.cli
                    .workspace
                    .stage(&cache)
                    .await
                    .map_err(storage_error)?,
            )
        } else {
            None
        };
        let target = staging
            .as_ref()
            .map_or(cache.as_path(), |staging| staging.path());
        let synchronized = self
            .cli
            .synchronize_with_environment(
                &remote.url,
                target,
                &claim.branch,
                &remote.environment,
                cancellation,
            )
            .await;
        // Remove temporary SSH credentials before executing user commands.
        drop(remote);
        let result = async {
            let result = synchronized?;
            let (name, hook) = if result.cloned {
                ("On Clone", &claim.repository.on_clone)
            } else {
                ("On Pull", &claim.repository.on_pull)
            };
            if let Some(hook) = hook {
                self.cli.run_hook(target, hook, name, cancellation).await?;
            }
            Ok::<_, GitError>(result)
        }
        .await;
        if let Some(staging) = staging.as_mut() {
            if result.is_ok() {
                if let Err(error) = staging.publish(&cache).await {
                    let _ = staging.cleanup().await;
                    return Err(GitError::Io(error).into());
                }
            } else {
                let _ = staging.cleanup().await;
            }
        }
        result.map_err(Into::into)
    }
}
