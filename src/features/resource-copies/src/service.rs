use crate::*;
use citadel_primitives::ActorId;
use std::sync::Arc;
use uuid::Uuid;

pub struct CopyService {
    backups: Arc<citadel_backups::BackupService>,
    store: Arc<dyn CopyRepository>,
    storage: Arc<dyn CopyStorage>,
    on_change: Arc<dyn Fn(CopyKind, Uuid) + Send + Sync>,
}
impl CopyService {
    pub fn new(
        store: Arc<dyn CopyRepository>,
        storage: Arc<dyn CopyStorage>,
        backups: Arc<citadel_backups::BackupService>,
        on_change: Arc<dyn Fn(CopyKind, Uuid) + Send + Sync>,
    ) -> Self {
        Self {
            store,
            storage,
            backups,
            on_change,
        }
    }
    /// Read-only eligibility for resource list and detail models; never creates a review
    /// or contacts Docker. Full preview remains authoritative at submission.
    pub async fn eligibility(
        &self,
        actor: ActorId,
        kind: CopyKind,
        id: Uuid,
    ) -> Result<CopyEligibility, CopyError> {
        let source = match self.store.source(actor, kind, id).await {
            Ok(source) => source,
            Err(CopyError::NotFound) => {
                return Ok(CopyEligibility::unavailable(
                    "Copying data requires access to the source, its bindings and target platform, and permission to create this resource.",
                ));
            }
            Err(error) => return Err(error),
        };
        let volumes = match planning::storage(&source, &mut source.spec.clone()) {
            Ok(volumes) => volumes,
            Err(CopyError::Validation(reason)) => return Ok(CopyEligibility::unavailable(reason)),
            Err(error) => return Err(error),
        };
        if volumes.is_empty() {
            return Ok(CopyEligibility::unavailable(
                "No named volumes configured. Use Configuration only for this resource.",
            ));
        }
        if volumes.len() > 100 {
            return Ok(CopyEligibility::unavailable(
                "A copy supports at most 100 volumes.",
            ));
        }
        if !self
            .store
            .current_data_allowed(actor, source.platform_id)
            .await?
        {
            let snapshots = self.store.snapshots(actor, &source).await?;
            if !volumes.iter().all(|(volume, node)| {
                snapshots
                    .iter()
                    .any(|s| &s.volume == volume && &s.docker_node_id == node)
            }) {
                return Ok(CopyEligibility::unavailable(
                    "Copying current data requires the Platform Download capability. Alternatively, each volume needs an available backup you can restore.",
                ));
            }
        }
        Ok(CopyEligibility {
            status: CopyEligibilityStatus::Available,
            reason: None,
        })
    }
    pub async fn targets(
        &self,
        actor: ActorId,
        kind: CopyKind,
        source: Uuid,
    ) -> Result<CopyTargets, CopyError> {
        self.store.targets(actor, kind, source).await
    }
    pub async fn preview(
        &self,
        actor: ActorId,
        request: CopyRequest,
    ) -> Result<CopyPreview, CopyError> {
        planning::validate_request(&request)?;
        let source = self
            .store
            .source(actor, request.kind, request.source_id)
            .await?;
        if request.data_mode == CopyDataMode::CurrentData
            && !self
                .store
                .current_data_allowed(actor, source.platform_id)
                .await?
        {
            return Err(CopyError::Validation("Copying current data requires the Platform Download capability. Select Use an existing backup if you have Restore access to backups of these volumes.".into()));
        }
        let mut snapshots = if request.data_mode == CopyDataMode::ExistingBackup {
            self.store.snapshots(actor, &source).await?
        } else {
            Vec::new()
        };
        let source_node = match &source.spec {
            CopySpec::SwarmService(spec) => planning::pinned_node(spec),
            _ => None,
        };
        if request
            .destination_platform_id
            .is_some_and(|id| id != source.platform_id)
            || request
                .destination_node_id
                .as_deref()
                .is_some_and(|node| Some(node) != source_node)
        {
            snapshots.retain(|s| s.supports_cross_platform_restore);
        }
        let mut plan = planning::plan(Uuid::now_v7(), source, request, snapshots)?;
        self.store.prepare_destination(actor, &mut plan).await?;
        if plan.target_platform() != plan.source.platform_id {
            plan.preview.warnings.push("The destination uses its own networks, images and external services. Swarm networks, secrets and configs are matched by name; their contents are not copied. Review these references before Apply.".into());
        }
        if plan.moves_storage() && matches!(plan.spec, CopySpec::SwarmService(_)) {
            plan.preview.warnings.push("The copy is pinned to the selected destination node. Other placement constraints are retained; review them before Apply.".into());
        }
        // Only complete reviews can be submitted. An incomplete preview remains
        // useful for choosing snapshots, without accumulating durable drafts.
        if plan.preview.blockers.is_empty() {
            let cancel = tokio_util::sync::CancellationToken::new();
            let _guard = cancel.clone().drop_guard();
            self.storage.check(&plan, None, &cancel).await?;
            self.store.save_review(actor, &plan).await?;
        }
        Ok(plan.preview)
    }
    pub async fn start(&self, actor: ActorId, id: Uuid) -> Result<CopyProgress, CopyError> {
        self.store.start(actor, id).await
    }
    pub async fn get(&self, actor: ActorId, id: Uuid) -> Result<CopyProgress, CopyError> {
        self.store.get(actor, id).await
    }
    pub async fn cleanup(&self, actor: ActorId, id: Uuid) -> Result<CopyProgress, CopyError> {
        self.store.cleanup(actor, id).await
    }
    pub async fn cancel(&self, actor: ActorId, id: Uuid) -> Result<CopyProgress, CopyError> {
        let progress = self.store.cancel(actor, id).await?;
        self.cancel_children(&progress).await?;
        Ok(progress)
    }
    async fn cancel_children(&self, progress: &CopyProgress) -> Result<(), CopyError> {
        if progress.status == CopyStatus::Cancelling
            && progress.data_mode == CopyDataMode::ExistingBackup
        {
            for restore in &progress.restores {
                if matches!(restore.status.as_str(), "Queued" | "Preparing" | "Running") {
                    self.backups
                        .cancel_restore(restore.id)
                        .await
                        .map_err(|e| CopyError::Storage(e.to_string()))?;
                }
            }
        }
        Ok(())
    }
    /// Called by the single execution lane, never concurrently with another
    /// current-data execution. Recovery also handles a committed claim whose
    /// acknowledgement was lost; an ambiguous mutation is never replayed.
    pub async fn process_current_queue(
        &self,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<(), CopyError> {
        self.store.recover_current().await?;
        for (actor, id) in self.store.pending().await? {
            if cancel.is_cancelled() {
                break;
            }
            // Return immediately on a claim error. The next iteration recovers
            // any ambiguous commit before another job can execute.
            if let Some((plan, volume)) = self.store.claim_current(actor, id).await? {
                self.process_current(actor, id, plan, volume, cancel)
                    .await?;
                break;
            }
        }
        Ok(())
    }

    async fn process_current(
        &self,
        actor: ActorId,
        id: Uuid,
        plan: CopyPlan,
        volume: CopyRestore,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<(), CopyError> {
        let execution_cancel = cancel.child_token();
        let _guard = execution_cancel.clone().drop_guard();
        let execution = self.storage.copy_current(&plan, &volume, &execution_cancel);
        tokio::pin!(execution);
        let mut poll = tokio::time::interval(std::time::Duration::from_secs(2));
        let result = loop {
            tokio::select! {
                result = &mut execution => break result.map_err(|e| e.to_string()),
                () = cancel.cancelled() => {
                    execution_cancel.cancel();
                    break execution.await.map_err(|e| e.to_string());
                }
                _ = poll.tick() => {
                    match self.store.get(actor, id).await {
                        Ok(progress) if progress.status == CopyStatus::Restoring => {},
                        _ => { execution_cancel.cancel(); break execution.await.map_err(|e| e.to_string()); }
                    }
                }
            }
        };
        // Keep the execution result until its idempotent completion write is
        // acknowledged. Retrying persistence must never rerun the storage copy.
        loop {
            match self
                .store
                .finish_current(actor, id, volume.id, result.clone())
                .await
            {
                Ok(()) => return Ok(()),
                Err(error) => {
                    tokio::select! {
                        () = cancel.cancelled() => return Err(error),
                        () = tokio::time::sleep(std::time::Duration::from_secs(2)) => {},
                    }
                }
            }
        }
    }
    pub async fn process(
        &self,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<(), CopyError> {
        for (actor, id) in self.store.pending().await? {
            if cancel.is_cancelled() {
                break;
            }
            let current = match self.store.get(actor, id).await {
                Err(error @ CopyError::Storage(_)) => return Err(error),
                result => result,
            };
            if matches!(current, Err(CopyError::NotFound)) {
                // A revoked actor must not leave an authorized cleanup queued
                // indefinitely. No storage is touched after access is revoked.
                self.store
                    .finish_cleanup(
                        actor,
                        id,
                        Err("Cleanup stopped because the initiating actor is unavailable.".into()),
                    )
                    .await?;
            }
            if let Ok(progress) = &current
                && progress.status == CopyStatus::Cleaning
            {
                let result = match self.store.plan(actor, id).await {
                    Ok(plan) => self
                        .storage
                        .cleanup(&plan, &progress.restores, cancel)
                        .await
                        .map_err(|e| e.to_string()),
                    Err(error @ CopyError::Storage(_)) => return Err(error),
                    Err(error) => Err(error.to_string()),
                };
                if cancel.is_cancelled() {
                    break;
                }
                self.store.finish_cleanup(actor, id, result).await?;
                continue;
            }
            let storage_result = if let Ok(current) = current
                && current.status == CopyStatus::Restoring
                && !current.restores.is_empty()
                && current.restores.iter().all(|r| r.status == "Succeeded")
            {
                Some(match self.store.plan(actor, id).await {
                    Ok(plan) => match self
                        .storage
                        .check(&plan, Some(&current.restores), cancel)
                        .await
                    {
                        Err(error @ CopyError::Storage(_)) => return Err(error),
                        result => result.map_err(|error| error.to_string()),
                    },
                    Err(error @ CopyError::Storage(_)) => return Err(error),
                    Err(error) => Err(error.to_string()),
                })
            } else {
                None
            };
            if cancel.is_cancelled() {
                break;
            }
            let progress = self.store.advance(actor, id, storage_result).await?;
            self.cancel_children(&progress).await?;
            if let Some(destination) = progress.destination_id {
                (self.on_change)(progress.kind, destination);
            }
        }
        Ok(())
    }
}
