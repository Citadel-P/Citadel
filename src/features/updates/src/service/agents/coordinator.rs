use crate::*;
use chrono::Utc;
use std::{collections::BTreeMap, sync::Arc};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Clone)]
pub struct AgentUpdateService {
    pub(super) core_version: String,
    pub(super) updates_allowed: bool,
    pub(super) repository: Arc<dyn AgentUpdateRepository>,
    pub(super) runtime: Arc<dyn AgentUpdateRuntime>,
    pub(super) entitlements: Arc<dyn AgentUpdateEntitlements>,
    pub(super) notifier: Arc<dyn UpdateChangeNotifier>,
    execution: Arc<tokio::sync::Mutex<()>>,
    pub(super) core: Option<Arc<super::schedule::ScheduledCore>>,
}
impl AgentUpdateService {
    pub fn new(
        core_version: String,
        repository: Arc<dyn AgentUpdateRepository>,
        runtime: Arc<dyn AgentUpdateRuntime>,
        entitlements: Arc<dyn AgentUpdateEntitlements>,
        notifier: Arc<dyn UpdateChangeNotifier>,
    ) -> Self {
        Self {
            core_version,
            updates_allowed: true,
            repository,
            runtime,
            entitlements,
            notifier,
            execution: Arc::new(tokio::sync::Mutex::new(())),
            core: None,
        }
    }
    pub fn with_updates_allowed(mut self, allowed: bool) -> Self {
        self.updates_allowed = allowed;
        self
    }
    pub async fn operation(&self, id: Uuid) -> Result<AgentUpdateOperation, UpdateError> {
        self.repository.operation(id).await
    }
    pub async fn status(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<(AgentUpdateState, Vec<AgentUpdateTarget>), UpdateError> {
        let mut state = self.repository.state().await?;
        state.release = state.release.filter(|r| r.version == self.core_version);
        let targets = self.targets(None, cancellation).await?;
        if targets.is_empty() {
            state.check_error = None;
        }
        Ok((state, targets))
    }
    pub(super) async fn targets(
        &self,
        selected: Option<&[String]>,
        cancellation: &CancellationToken,
    ) -> Result<Vec<AgentUpdateTarget>, UpdateError> {
        let mut targets = self.runtime.targets(selected, cancellation).await?;
        targets.sort_by_key(|t| (!t.connected, t.resource_id));
        // Runtime keys identify an installation; resource aliases never create additional jobs.
        let mut unique: BTreeMap<String, AgentUpdateTarget> = BTreeMap::new();
        for target in targets {
            if let Some(previous) = unique.get(&target.id) {
                if previous.connected
                    && target.connected
                    && (previous.installed_version != target.installed_version
                        || previous.revision != target.revision)
                {
                    return Err(UpdateError::Conflict("Agent identity observations disagree. Refresh the connections before updating.".into()));
                }
            } else {
                unique.insert(target.id.clone(), target);
            }
        }
        Ok(unique.into_values().collect())
    }
    pub async fn check(&self, cancellation: &CancellationToken) -> Result<(), UpdateError> {
        if cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        let observed_at = Utc::now();
        let targets = self.targets(None, cancellation).await?;
        if targets.is_empty() {
            self.repository.save_release(None, None).await?;
            self.notifier.changed();
            // Retain verified release evidence so removed targets can resolve availability alerts.
            if let Some(release) = self.repository.state().await?.release {
                self.repository
                    .observe_agent_availability(&release, &targets, observed_at)
                    .await?;
            }
            return Ok(());
        }
        let release = self.runtime.release(&self.core_version, cancellation).await;
        match release {
            Ok(release) if release.version == self.core_version => {
                self.repository.save_release(Some(&release), None).await?
            }
            Err(UpdateError::Cancelled) => return Err(UpdateError::Cancelled),
            _ => {
                self.repository
                    .save_release(
                        None,
                        Some("The Agent release matching Core could not be verified."),
                    )
                    .await?;
                self.notifier.changed();
                return Ok(());
            }
        }
        self.notifier.changed();
        self.refresh_availability(cancellation).await
    }

    async fn refresh_availability(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<(), UpdateError> {
        let state = self.repository.state().await?;
        if let Some(release) = state
            .release
            .filter(|release| release.version == self.core_version)
        {
            let observed_at = Utc::now();
            let targets = self.targets(None, cancellation).await?;
            self.repository
                .observe_agent_availability(&release, &targets, observed_at)
                .await?;
        }
        Ok(())
    }

    pub async fn save_schedule(
        &self,
        actor: UpdateActor,
        mut schedule: UpdateSchedule,
        cancellation: &CancellationToken,
    ) -> Result<(), UpdateError> {
        self.repository.authorize(actor.actor_id).await?;
        if schedule.enabled && !self.updates_allowed {
            return Err(UpdateError::Unavailable(
                "Updates are disabled by server configuration.".into(),
            ));
        }
        schedule.targets.sort();
        schedule.targets.dedup();
        schedule.validate()?;
        let entitled = self.entitlements.automated_operations().await?;
        let previous = self.repository.state().await?.schedule;
        let disabling = !schedule.enabled
            && UpdateSchedule {
                enabled: false,
                ..previous.clone()
            } == schedule;
        if !entitled && !disabling {
            return Err(UpdateError::Forbidden);
        }
        if !disabling
            && schedule.include_core
            && !previous.include_core
            && self
                .core
                .as_ref()
                .and_then(|core| core.installer.as_ref())
                .is_none()
        {
            return Err(UpdateError::Unavailable(
                "Core installation is not configured on this instance.".into(),
            ));
        }
        if !disabling {
            let targets = self.targets(None, cancellation).await?;
            if schedule
                .targets
                .iter()
                .any(|id| !targets.iter().any(|t| &t.id == id && t.supported))
            {
                return Err(UpdateError::Validation(
                    "Scheduled targets must be registered, supported Agent installations.".into(),
                ));
            }
        }
        self.repository
            .save_schedule(actor, &schedule, entitled)
            .await?;
        self.notifier.changed();
        Ok(())
    }
    pub async fn review(
        &self,
        actor: UpdateActor,
        selected: &[String],
        cancellation: &CancellationToken,
    ) -> Result<AgentUpdateReview, UpdateError> {
        self.repository.authorize(actor.actor_id).await?;
        let review = self
            .prepare(actor.actor_id, selected, false, cancellation)
            .await?;
        self.repository.save_review(actor, &review).await?;
        Ok(review)
    }
    async fn prepare(
        &self,
        actor: citadel_primitives::ActorId,
        selected: &[String],
        scheduled: bool,
        cancellation: &CancellationToken,
    ) -> Result<AgentUpdateReview, UpdateError> {
        if !self.updates_allowed {
            return Err(UpdateError::Unavailable(
                "Updates are disabled by server configuration.".into(),
            ));
        }
        if selected.is_empty() || selected.len() > 100 {
            return Err(UpdateError::Validation(
                "Select between one and 100 Agents.".into(),
            ));
        }
        let state = self.repository.state().await?;
        let release = state
            .release
            .filter(|r| r.version == self.core_version)
            .ok_or_else(|| {
                UpdateError::Conflict("Check releases before reviewing Agent updates.".into())
            })?;
        let targets = self
            .prepare_targets(selected, &release, scheduled, cancellation)
            .await?;
        Ok(AgentUpdateReview {
            id: Uuid::now_v7(),
            actor_id: actor,
            release,
            targets,
            expires_at: Utc::now() + chrono::Duration::minutes(5),
        })
    }
    pub(super) async fn prepare_targets(
        &self,
        selected: &[String],
        release: &AgentUpdateRelease,
        scheduled: bool,
        cancellation: &CancellationToken,
    ) -> Result<Vec<AgentUpdateTarget>, UpdateError> {
        let inventory = self.targets(Some(selected), cancellation).await?;
        let mut targets = Vec::new();
        for id in selected.iter().collect::<std::collections::BTreeSet<_>>() {
            let missing = AgentUpdateTarget {
                id: id.clone(),
                resource_id: Uuid::nil(),
                kind: AgentUpdateKind::Platform,
                name: id.clone(),
                installed_version: None,
                connected: false,
                supported: false,
                reason: Some("Agent is no longer registered.".into()),
                nodes: 0,
                drain_nodes: vec![],
                revision: String::new(),
            };
            let target = inventory
                .iter()
                .find(|t| &t.id == id)
                .or_else(|| scheduled.then_some(&missing))
                .ok_or_else(|| {
                    UpdateError::Conflict(
                        "An Agent registration changed. Review the selection again.".into(),
                    )
                })?;
            if target.eligibility(Some(release)) != AgentUpdateEligibility::Available && !scheduled
            {
                return Err(UpdateError::Conflict(format!(
                    "{} is no longer eligible for an update.",
                    target.name
                )));
            }
            targets.push(target.clone());
        }
        Ok(targets)
    }
    pub async fn approve(
        &self,
        actor: UpdateActor,
        review: Uuid,
        request: Uuid,
    ) -> Result<AgentUpdateOperation, UpdateError> {
        if request.is_nil() {
            return Err(UpdateError::Validation(
                "Request ID must be nonzero.".into(),
            ));
        }
        let operation = self.repository.approve(actor, review, request).await?;
        self.notifier.changed();
        Ok(operation)
    }
    pub async fn tick(&self, cancellation: &CancellationToken) -> Result<(), UpdateError> {
        let Ok(_guard) = self.execution.try_lock() else {
            return Ok(());
        };
        let state = self.repository.state().await?;
        if let Some(mut operation) = state.operation.clone().filter(|op| !op.finished) {
            let mut failed = false;
            for index in 0..operation.targets.len() {
                use AgentUpdatePhase::*;
                if cancellation.is_cancelled() {
                    return Err(UpdateError::Cancelled);
                }
                if operation.targets[index].phase == Failed {
                    failed = true;
                }
                if !matches!(operation.targets[index].phase, Pending | Updating) {
                    continue;
                }
                let target = operation.targets[index].target.clone();
                let outcome = if failed {
                    (
                        NotStarted,
                        Some("Not started after an earlier failure.".into()),
                    )
                } else if operation.targets[index].phase == Updating {
                    // Only reconciliation is allowed after an interrupted dispatch.
                    if self
                        .runtime
                        .verify(&target, &operation.release, operation.id, cancellation)
                        .await?
                    {
                        (Succeeded, None)
                    } else {
                        (Failed, Some("The interrupted Agent update could not be verified. Inspect the Agent and updater helper before retrying.".into()))
                    }
                } else {
                    self.execute_target(&mut operation, index, cancellation)
                        .await?
                };
                // The attempt is still Updating until cleanup is acknowledged. A lost Edge
                // session or shutdown leaves it owned for the next worker tick/restart.
                if operation.targets[index].phase == Updating
                    && let Err(error) = self
                        .runtime
                        .resume(&target, operation.id, cancellation)
                        .await
                {
                    let message = "Agent update recovery required. Waiting for verified helper completion or admission cleanup; ownership is retained.";
                    if !cancellation.is_cancelled()
                        && operation.targets[index].message.as_deref() != Some(message)
                    {
                        operation.targets[index].message = Some(message.into());
                        operation.updated_at = Utc::now();
                        self.repository.save_operation(&operation).await?;
                        self.notifier.changed();
                    }
                    return Err(error);
                }
                if cancellation.is_cancelled() {
                    return Err(UpdateError::Cancelled);
                }
                failed |= outcome.0 == Failed;
                operation.targets[index].phase = outcome.0;
                operation.targets[index].message = outcome.1;
                operation.updated_at = Utc::now();
                operation = self.repository.save_operation(&operation).await?;
                self.notifier.changed();
            }
            operation.finished = true;
            operation.updated_at = Utc::now();
            self.repository.save_operation(&operation).await?;
            self.notifier.changed();
            // Alert observation failures must not retain a completed update claim.
            self.refresh_availability(cancellation).await?;
            return Ok(());
        }
        self.tick_schedule(state, cancellation).await
    }
    async fn execute_target(
        &self,
        operation: &mut AgentUpdateOperation,
        index: usize,
        cancellation: &CancellationToken,
    ) -> Result<(AgentUpdatePhase, Option<String>), UpdateError> {
        use AgentUpdatePhase::*;
        if !self.updates_allowed || operation.release.version != self.core_version {
            return Ok((
                NotStarted,
                Some("Core changed since review. Review Agent updates again.".into()),
            ));
        }
        if operation.scheduled {
            if !self.entitlements.automated_operations().await? {
                return Ok((
                    NotStarted,
                    Some("Team is required to start scheduled updates.".into()),
                ));
            }
        } else if self.repository.authorize(operation.actor_id).await.is_err() {
            return Ok((
                NotStarted,
                Some("The requesting administrator no longer has access.".into()),
            ));
        }
        let target = operation.targets[index].target.clone();
        let current = self
            .targets(Some(std::slice::from_ref(&target.id)), cancellation)
            .await?
            .into_iter()
            .find(|t| t.id == target.id);
        let Some(current) = current else {
            return Ok((
                Skipped,
                Some("Agent registration is no longer available.".into()),
            ));
        };
        let eligibility = current.eligibility(Some(&operation.release));
        if eligibility == AgentUpdateEligibility::Current {
            return Ok((Skipped, Some("Already up to date.".into())));
        }
        if eligibility != AgentUpdateEligibility::Available
            || current.resource_id != target.resource_id
            || current.kind != target.kind
            || current.revision != target.revision
            || current.installed_version != target.installed_version
            || current.drain_nodes != target.drain_nodes
        {
            return Ok((
                Skipped,
                Some("Agent availability or installation changed since review.".into()),
            ));
        }
        operation.targets[index].phase = Updating;
        operation.updated_at = Utc::now();
        *operation = self.repository.save_operation(operation).await?;
        self.notifier.changed();
        let outcome = self
            .runtime
            .install(&target, &operation.release, operation.id, cancellation)
            .await;
        if cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        match outcome {
            Ok(()) => Ok((Succeeded, None)),
            Err(UpdateError::Cancelled) => Err(UpdateError::Cancelled),
            Err(error) => Ok((Failed, Some(error.to_string()))),
        }
    }
}
