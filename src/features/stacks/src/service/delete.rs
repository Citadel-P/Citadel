use super::*;
impl StackService {
    pub async fn delete(
        &self,
        actor: ActorId,
        administrator: bool,
        ids: &[Uuid],
    ) -> Result<(), StackError> {
        if ids.is_empty() || ids.len() > 100 {
            return Err(validation("Between 1 and 100 Stack IDs are required."));
        }
        let mut unique = ids.to_vec();
        unique.sort_unstable();
        unique.dedup();
        let claims = self
            .store
            .claim_delete(actor, administrator, &unique)
            .await?;
        let cancellation = self.shutdown.child_token();
        for (index, claim) in claims.iter().enumerate() {
            if !claim.platform_offline {
                let outcome = tokio::time::timeout(
                    self.timeout.min(Duration::from_secs(120)),
                    self.runtime.delete(claim, &cancellation),
                )
                .await;
                match outcome {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        let pending = claims[index..]
                            .iter()
                            .map(|claim| claim.stack_id)
                            .collect::<Vec<_>>();
                        self.store.release_delete(&pending).await?;
                        return Err(error);
                    }
                    Err(_) => {
                        let unstarted = claims[index + 1..]
                            .iter()
                            .map(|claim| claim.stack_id)
                            .collect::<Vec<_>>();
                        if !unstarted.is_empty() {
                            self.store.release_delete(&unstarted).await?;
                        }
                        return Err(StackError::Runtime(
                        "Timed out while removing Stack runtime resources. Citadel will reconcile the claimed deletion.".to_owned(),
                    ));
                    }
                }
            }
            self.store
                .complete_delete(actor, std::slice::from_ref(claim))
                .await?;
            self.notifier.changed(claim.stack_id, "deleted");
        }
        Ok(())
    }

    pub async fn change_state(
        &self,
        actor: ActorId,
        administrator: bool,
        ids: &[Uuid],
        action: StackAction,
    ) -> Result<(), StackError> {
        let ids = ids
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        if ids.is_empty() || ids.len() > 100 {
            return Err(validation("Between 1 and 100 Stack IDs are required."));
        }
        let Ok(_permit) = self.operations.clone().acquire_owned().await else {
            return Err(StackError::Cancelled);
        };
        let claims = self.store.claim_state(actor, administrator, &ids).await?;
        let cancellation = self.shutdown.child_token();
        for (index, claim) in claims.iter().enumerate() {
            let outcome = tokio::time::timeout(
                self.timeout.min(Duration::from_secs(120)),
                self.runtime.change_state(
                    claim.platform_id,
                    &claim.project_name,
                    orchestration(&claim.platform_type),
                    action,
                    &cancellation,
                ),
            )
            .await;
            match outcome {
                Ok(Ok(container_ids)) => {
                    self.store
                        .complete_state(actor, claim, state_action_status(action), &container_ids)
                        .await?;
                    self.notifier.changed(claim.stack_id, "stateChanged");
                }
                Ok(Err(error)) => {
                    let ambiguous = matches!(error, StackError::Runtime(_) | StackError::Cancelled);
                    let release_from = if ambiguous { index + 1 } else { index };
                    if release_from < claims.len() {
                        self.store.release_state(&claims[release_from..]).await?;
                    }
                    return Err(error);
                }
                Err(_) => {
                    if index + 1 < claims.len() {
                        self.store.release_state(&claims[index + 1..]).await?;
                    }
                    return Err(StackError::Runtime(
                        "The Stack state operation timed out. Citadel will reconcile its runtime outcome."
                            .to_owned(),
                    ));
                }
            }
        }
        Ok(())
    }
}
