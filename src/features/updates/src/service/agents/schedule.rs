use super::AgentUpdateService;
use crate::*;
use chrono::{Timelike, Utc};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub(super) struct ScheduledCore {
    pub(super) checker: ReleaseCheckService,
    pub(super) installer: Option<UpdateInstallService>,
}

impl AgentUpdateService {
    pub fn with_core(
        mut self,
        checker: ReleaseCheckService,
        installer: Option<UpdateInstallService>,
    ) -> Self {
        self.core = Some(Arc::new(ScheduledCore { checker, installer }));
        self
    }

    pub(super) async fn tick_schedule(
        &self,
        state: AgentUpdateState,
        cancellation: &CancellationToken,
    ) -> Result<(), UpdateError> {
        let mut run = if let Some(run) = state
            .run
            .filter(|r| r.phase != UpdateSchedulePhase::Finished)
        {
            run
        } else {
            if cancellation.is_cancelled()
                || !self.updates_allowed
                || !state.schedule.enabled
                || !self.entitlements.automated_operations().await?
            {
                return Ok(());
            }
            let minute = Utc::now()
                .with_second(0)
                .and_then(|t| t.with_nanosecond(0))
                .ok_or_else(|| UpdateError::Storage("Invalid scheduler clock.".into()))?;
            if !citadel_primitives::schedule::schedule_is_due(
                Some(&state.schedule.cron),
                &state.schedule.time_zone,
                minute,
            ) {
                return Ok(());
            }
            let Some(run) = self
                .repository
                .reserve_schedule(&state.schedule, minute)
                .await?
            else {
                return Ok(());
            };
            self.notifier.changed();
            run
        };
        let result = self.continue_schedule(&mut run, cancellation).await;
        if let Err(error) = &result {
            if cancellation.is_cancelled() {
                return result;
            }
            // Re-read after any lost claim response. Never drop an execution's owner on ambiguity.
            let latest = self
                .repository
                .state()
                .await?
                .run
                .ok_or_else(|| UpdateError::Conflict("Scheduled update disappeared.".into()))?;
            if latest.id == run.id
                && latest.core_operation.is_none()
                && latest.agent_operation.is_none()
            {
                self.finish_schedule(latest, Some(error.to_string()))
                    .await?;
            }
        }
        result
    }

    async fn finish_schedule(
        &self,
        mut run: UpdateScheduleRun,
        message: Option<String>,
    ) -> Result<(), UpdateError> {
        run.phase = UpdateSchedulePhase::Finished;
        run.message = message;
        self.repository.save_run(&run).await?;
        self.notifier.changed();
        Ok(())
    }

    async fn continue_schedule(
        &self,
        run: &mut UpdateScheduleRun,
        cancellation: &CancellationToken,
    ) -> Result<(), UpdateError> {
        use UpdateSchedulePhase::*;
        // Owned replacements settle before this method; policy only stops new components.
        if let Some(id) = run.core_operation {
            let installer = self
                .core
                .as_ref()
                .and_then(|c| c.installer.as_ref())
                .ok_or_else(|| {
                    UpdateError::Unavailable(
                        "Scheduled Core recovery requires the registered installer.".into(),
                    )
                })?;
            if installer.active().await?.is_some() {
                return Ok(());
            }
            let operation = installer.operation(id).await?;
            if operation.phase != UpdatePhase::Succeeded {
                return self
                    .finish_schedule(
                        run.clone(),
                        Some("Core update did not succeed. Agents were not started.".into()),
                    )
                    .await;
            }
            if operation.plan
                != *run
                    .core_plan
                    .as_ref()
                    .ok_or_else(|| UpdateError::Conflict("Missing pinned Core plan.".into()))?
            {
                return Err(UpdateError::Conflict(
                    "Scheduled Core execution identity changed.".into(),
                ));
            }
            if operation.plan.target.version != self.core_version {
                return self.finish_schedule(run.clone(), Some("Running Core does not match the verified scheduled target. Agents were not started.".into())).await;
            }
        }
        if let Some(id) = run.agent_operation {
            let operation = self.repository.operation(id).await?;
            if !operation.finished {
                return Ok(());
            }
            let failed = operation.targets.iter().any(|t| {
                matches!(
                    t.phase,
                    AgentUpdatePhase::Failed | AgentUpdatePhase::NotStarted
                )
            });
            return self.finish_schedule(run.clone(), failed.then(|| "Agent updates did not all complete. Review the individual outcomes; any successful Core update is retained.".into())).await;
        }
        if !self.updates_allowed || !self.entitlements.automated_operations().await? {
            return self
                .finish_schedule(
                    run.clone(),
                    Some("Scheduled updates paused by server policy or Team entitlement.".into()),
                )
                .await;
        }
        if cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        if run.phase == Preparing {
            let mut version = self.core_version.clone();
            if run.schedule.include_core {
                let core = self.core.as_ref().ok_or_else(|| {
                    UpdateError::Unavailable("Core installation is not configured.".into())
                })?;
                let installer = core.installer.as_ref().ok_or_else(|| {
                    UpdateError::Unavailable("Core installation is not configured.".into())
                })?;
                let checked = core
                    .checker
                    .check_now(
                        ReleaseCheckPolicy {
                            initialized: true,
                            checks_allowed: self.updates_allowed,
                        },
                        cancellation,
                    )
                    .await?;
                if let Some(error) = checked.error {
                    return Err(UpdateError::Unavailable(error));
                }
                if let Some(release) = checked.available {
                    let plan = installer
                        .prepare_scheduled(&release.version, cancellation)
                        .await?;
                    version = plan.target.version.clone();
                    run.core_plan = Some(plan);
                }
            }
            // Pin the release and installation identities, but create no Agent execution before Core.
            if !run.schedule.targets.is_empty() {
                let release = self.runtime.release(&version, cancellation).await?;
                if release.version != version {
                    return Err(UpdateError::Conflict(
                        "Agent release does not match the scheduled Core version.".into(),
                    ));
                }
                run.targets = self
                    .prepare_targets(&run.schedule.targets, &release, true, cancellation)
                    .await?;
                run.release = Some(release);
            }
            run.phase = if run.core_plan.is_some() {
                Core
            } else {
                Agents
            };
            *run = self.repository.save_run(run).await?;
            self.notifier.changed();
        }
        if run.phase == Core {
            if run.core_operation.is_none() {
                if !self.entitlements.automated_operations().await? {
                    return self
                        .finish_schedule(
                            run.clone(),
                            Some("Team is required for scheduled updates.".into()),
                        )
                        .await;
                }
                let installer = self
                    .core
                    .as_ref()
                    .and_then(|c| c.installer.as_ref())
                    .ok_or_else(|| {
                        UpdateError::Unavailable("Core installation is not configured.".into())
                    })?;
                installer
                    .schedule(
                        run.clone(),
                        self.repository.clone(),
                        self.entitlements.clone(),
                    )
                    .await?;
                return Ok(());
            }
            run.phase = Agents;
            *run = self.repository.save_run(run).await?;
            self.notifier.changed();
        }
        if run.phase == Agents {
            if run.targets.is_empty() {
                return self.finish_schedule(run.clone(), None).await;
            }
            let release = run
                .release
                .as_ref()
                .ok_or_else(|| UpdateError::Conflict("Missing pinned Agent release.".into()))?;
            if release.version != self.core_version {
                return self.finish_schedule(run.clone(), Some("Running Core does not match the pinned scheduled release. Agents were not started.".into())).await;
            }
            if !self.entitlements.automated_operations().await? {
                return self
                    .finish_schedule(
                        run.clone(),
                        Some("Team is required for scheduled updates.".into()),
                    )
                    .await;
            }
            // The existing Agent executor rechecks these frozen identities and eligibility per target.
            self.repository.save_release(Some(release), None).await?;
            self.repository.claim_agents(run).await?;
            self.notifier.changed();
        }
        Ok(())
    }
}
