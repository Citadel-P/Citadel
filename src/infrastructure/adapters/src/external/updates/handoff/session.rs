use super::super::{
    InstallationLock, JournalDirectory, bootstrap::verify_handoff_source, docker::LocalDocker,
};
use super::ExecutionJournal;
use super::{
    evidence::{HelperAcknowledgement, HelperLaunchIntent, binding, conflict},
    runtime::private_home,
    verification,
};
use citadel_execution::ProcessRunner;
use citadel_updates::{UPDATE_PROTOCOL, UpdateError, UpdateJournal, UpdatePhase};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

/// Owned by the detached helper until its checkpoint/replacement/recovery path
/// completes. Dropping releases only the OS lock, never durable maintenance.
pub(super) struct OwnedUpdateHelper {
    pub(super) state: JournalDirectory,
    pub(super) owner: InstallationLock,
    pub(super) intent: HelperLaunchIntent,
    pub(super) acknowledgement: HelperAcknowledgement,
}
impl OwnedUpdateHelper {
    /// Executor identity comes from the helper's own runtime context. This is
    /// internal orchestration, never a browser-supplied container identifier.
    pub async fn acknowledge(
        state: JournalDirectory,
        intent: HelperLaunchIntent,
        executor_id: &str,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<Self, UpdateError> {
        let owner = state.existing_lock()?;
        intent.verify_saved(&state)?;
        let binding = binding(&state, &intent)?;
        if executor_id != binding.helper_container_id {
            return Err(conflict());
        }
        let home = private_home()?;
        let docker = LocalDocker {
            process,
            cancellation,
            home: home.path(),
            deadline: Instant::now() + Duration::from_secs(120),
        };
        let image = verification::helper(&docker, &intent, &binding).await?;
        verify_handoff_source(
            &docker,
            &intent.registration,
            Some(&binding.helper_container_id),
        )
        .await?;
        let core = verification::container(&docker, &intent.registration.core_container_id).await?;
        if core["Image"] != image.image {
            return Err(conflict());
        }
        state.verify_lock(&owner)?;
        intent.verify_saved(&state)?;
        if cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        let mut draining = intent.operation.clone();
        draining.advance(0, UpdatePhase::Draining, chrono::Utc::now())?;
        let journal = UpdateJournal {
            protocol: UPDATE_PROTOCOL,
            operation: draining.clone(),
            helper_container_id: binding.helper_container_id.clone(),
            history: vec![intent.operation.clone(), draining],
            verified_target: None,
            completion_acknowledged: false,
            maintenance_released: false,
        };
        journal.validate_for(&intent.operation)?;
        let acknowledgement = HelperAcknowledgement {
            binding,
            started_at: image.started_at,
        };
        state.create(
            &format!("{}.json", intent.operation.id),
            &ExecutionJournal {
                journal,
                acknowledgement: acknowledgement.clone(),
                prepared: None,
                stop: None,
                replacement: None,
                submitted: false,
                target: None,
            },
        )?;
        Ok(Self {
            state,
            owner,
            intent,
            acknowledgement,
        })
    }

    /// Keep this session alive through checkpoint preparation. A successful
    /// return means a clean Core exit was observed, not target readiness.
    pub async fn stop_source(
        &self,
        process: &dyn ProcessRunner,
        budget: Duration,
        cancellation: &CancellationToken,
    ) -> Result<UpdateJournal, UpdateError> {
        if budget.is_zero() || budget > Duration::from_secs(600) {
            return Err(UpdateError::Validation(
                "Shutdown observation budget must be between zero and ten minutes.".into(),
            ));
        }
        self.check_grant()?;
        let home = private_home()?;
        let docker = LocalDocker {
            process,
            cancellation,
            home: home.path(),
            deadline: Instant::now() + budget,
        };
        let image =
            verification::helper(&docker, &self.intent, &self.acknowledgement.binding).await?;
        verify_handoff_source(
            &docker,
            &self.intent.registration,
            Some(&self.acknowledgement.binding.helper_container_id),
        )
        .await?;
        if image.started_at != self.acknowledgement.started_at {
            return Err(conflict());
        }
        let core_id = &self.intent.registration.core_container_id;
        let core = verification::container(&docker, core_id).await?;
        if core["Image"] != image.image
            || core["State"]["StartedAt"]
                .as_str()
                .is_none_or(str::is_empty)
            || core["RestartCount"].as_u64().is_none()
        {
            return Err(conflict());
        }
        let mut journal = self.state.journal(self.intent.operation.id)?;
        journal.validate_for(&self.intent.operation)?;
        if journal.operation.phase != UpdatePhase::Draining
            || journal.operation.revision != 1
            || journal.operation.checkpoint.is_some()
            || journal.verified_target.is_some()
            || journal.completion_acknowledged
            || journal.maintenance_released
            || journal.helper_container_id != self.acknowledgement.binding.helper_container_id
        {
            return Err(conflict());
        }
        self.check_grant()?;
        if cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        let mut execution = ExecutionJournal::read(&self.state, self.intent.operation.id)?;
        if execution.stop.is_some() || execution.journal != journal {
            return Err(conflict());
        }
        execution.stop = Some(super::checkpoint::SourceProcess::from_container(&core)?);
        execution.save(&self.state, &self.owner)?;
        // Docker's -1 timeout disables daemon-side SIGKILL escalation. The client
        // is bounded; interruption can leave a daemon request in flight, so an
        // error is not a retry/resume signal. Stop intent prohibits blind replay.
        docker
            .run_with_timeout(
                &[
                    "container",
                    "stop",
                    "--signal",
                    "SIGTERM",
                    "--timeout",
                    "-1",
                    core_id,
                ],
                budget,
            )
            .await?;
        loop {
            self.check_grant()?;
            let helper =
                verification::helper(&docker, &self.intent, &self.acknowledgement.binding).await?;
            if helper.started_at != self.acknowledgement.started_at {
                return Err(conflict());
            }
            let database =
                verification::container(&docker, &self.intent.registration.database_container_id)
                    .await?;
            verification::database_running(&database)?;
            let current = verification::container(&docker, core_id).await?;
            if current["Image"] != core["Image"]
                || current["State"]["StartedAt"] != core["State"]["StartedAt"]
                || current["RestartCount"] != core["RestartCount"]
            {
                return Err(conflict());
            }
            if current["State"]["Running"] == false {
                if current["State"]["Status"] != "exited"
                    || current["State"]["ExitCode"] != 0
                    || current["State"]["OOMKilled"] != false
                    || current["State"]["Error"] != ""
                    || current["State"]["Restarting"] != false
                    || current["State"]["Paused"] != false
                {
                    return Err(conflict());
                }
                break;
            }
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(UpdateError::Cancelled),
                () = tokio::time::sleep(Duration::from_millis(250)) => {}
            }
        }
        self.check_grant()?;
        if self.state.journal(self.intent.operation.id)? != journal {
            return Err(conflict());
        }
        journal
            .operation
            .advance(1, UpdatePhase::BackingUp, chrono::Utc::now())?;
        journal.history.push(journal.operation.clone());
        journal.validate_for(&self.intent.operation)?;
        if cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        self.save_journal(journal.clone())?;
        Ok(journal)
    }

    pub(super) fn check_grant(&self) -> Result<(), UpdateError> {
        self.state.verify_lock(&self.owner)?;
        self.intent.verify_saved(&self.state)?;
        super::preparation::check_prepared_target(
            &self.state,
            &self.intent,
            &self.acknowledgement,
            false,
        )?;
        if binding(&self.state, &self.intent)? != self.acknowledgement.binding {
            return Err(conflict());
        }
        if ExecutionJournal::read(&self.state, self.intent.operation.id)?.acknowledgement
            != self.acknowledgement
            || self
                .state
                .read::<HelperAcknowledgement>(&self.intent.file("shutdown-grant"))?
                != self.acknowledgement
        {
            return Err(conflict());
        }
        Ok(())
    }
}
