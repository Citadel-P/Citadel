use crate::{
    InstallationPolicy, InstallationSettings, InstanceError, InstanceRepository,
    PatchInstallationSettings,
};
use citadel_identity::{ActorPrincipal, Clock};
use citadel_primitives::PatchField;
use std::sync::Arc;

pub struct InstanceService {
    repository: Arc<dyn InstanceRepository>,
    jobs: Option<Arc<dyn crate::InstanceJobsReader>>,
    schedules: Option<Arc<dyn crate::ScheduleReader>>,
    schedule_changes: tokio::sync::watch::Sender<crate::ScheduleActivity>,
    automation: tokio::sync::watch::Sender<Option<bool>>,
    schedule_deadlines: std::sync::RwLock<
        std::collections::BTreeMap<crate::SchedulePurpose, chrono::DateTime<chrono::Utc>>,
    >,
    job_history: Option<Arc<dyn crate::JobHistoryRepository>>,
    restart: Option<Arc<crate::InstanceRestartService>>,
    clock: Arc<dyn Clock>,
    installation_policy: InstallationPolicy,
    installation_changes: tokio::sync::watch::Sender<()>,
}
impl InstanceService {
    pub fn with_schedules(mut self, reader: Arc<dyn crate::ScheduleReader>) -> Self {
        self.schedules = Some(reader);
        self
    }
    pub fn observe_automated_operations(&self, allowed: Option<bool>) {
        if self.automation.send_if_modified(|old| {
            if *old == allowed {
                return false;
            }
            *old = allowed;
            true
        }) {
            self.notify_schedules();
        }
    }
    pub fn schedule_blocker(
        &self,
        p: crate::SchedulePurpose,
    ) -> Option<crate::SchedulePolicyBlocker> {
        use crate::SchedulePolicyBlocker::{Blocked, EligibilityUnknown};
        use crate::SchedulePurpose::*;
        let policy = &self.installation_policy;
        match p {
            Script if !policy.running.scripts_enabled => {
                Some(Blocked("Script execution is disabled in Core settings."))
            }
            Backup if !policy.running.backups_enabled => {
                Some(Blocked("Backups are disabled in Core settings."))
            }
            Install | Release if !policy.update_checks_allowed => {
                Some(Blocked("Updates are disabled in Core settings."))
            }
            Script | Backup | Install => match *self.automation.borrow() {
                Some(true) => None,
                Some(false) => Some(Blocked("Requires Automated Operations.")),
                None => Some(EligibilityUnknown),
            },
            _ => None,
        }
    }
    pub fn apply_schedule_policy(&self, row: &mut crate::ScheduleDefinition) {
        self.apply_schedule_blocker(row, self.schedule_blocker(row.purpose));
    }
    fn apply_schedule_blocker(
        &self,
        row: &mut crate::ScheduleDefinition,
        blocker: Option<crate::SchedulePolicyBlocker>,
    ) {
        if !row.enabled {
            return;
        }
        if let Some(crate::SchedulePolicyBlocker::Blocked(reason)) = blocker {
            row.blocker = Some(reason.into());
        } else if row.script_timeout_seconds.is_some_and(|seconds| {
            seconds < 1
                || seconds as u32
                    > self
                        .installation_policy
                        .running
                        .scripts_maximum_timeout_seconds
        }) {
            row.blocker =
                Some("The script timeout exceeds the limit configured for this Core.".into());
        } else if row.blocker.is_none() {
            row.blocker = blocker.map(|reason| reason.reason().into());
        }
    }
    pub fn notify_schedules(&self) {
        self.schedule_changes
            .send_modify(|state| state.catalog_revision = state.catalog_revision.wrapping_add(1));
    }
    /// The scheduler admits at most one batch of each purpose at a time.
    pub fn start_schedule_batch(
        &self,
        purpose: crate::SchedulePurpose,
    ) -> crate::ScheduleBatchGuard<'_> {
        self.schedule_changes.send_modify(|state| {
            state.active_batches.insert(purpose);
        });
        crate::ScheduleBatchGuard {
            purpose,
            changes: &self.schedule_changes,
        }
    }
    pub fn schedule_changes(
        &self,
        principal: &ActorPrincipal,
    ) -> Result<tokio::sync::watch::Receiver<crate::ScheduleActivity>, InstanceError> {
        if !principal.is_administrator() {
            return Err(InstanceError::Forbidden);
        }
        Ok(self.schedule_changes.subscribe())
    }
    pub fn set_schedule_deadline(
        &self,
        p: crate::SchedulePurpose,
        at: chrono::DateTime<chrono::Utc>,
    ) {
        self.schedule_deadlines
            .write()
            .expect("schedule deadlines")
            .insert(p, at);
        self.notify_schedules();
    }
    fn catalog_policy(&self, attention_only: bool) -> crate::ScheduleCatalogPolicy {
        crate::ScheduleCatalogPolicy {
            blockers: crate::SchedulePurpose::ALL
                .into_iter()
                .filter_map(|p| self.schedule_blocker(p).map(|reason| (p, reason)))
                .collect(),
            max_script_timeout: self
                .installation_policy
                .running
                .scripts_maximum_timeout_seconds,
            attention_only,
        }
    }
    pub async fn schedule_counts(
        &self,
        principal: &ActorPrincipal,
        search: &str,
    ) -> Result<Vec<crate::ScheduleCount>, InstanceError> {
        self.schedule_catalog(principal)?
            .counts(search.trim(), &self.catalog_policy(false))
            .await
    }

    fn schedule_catalog(
        &self,
        principal: &ActorPrincipal,
    ) -> Result<&dyn crate::ScheduleReader, InstanceError> {
        if !principal.is_administrator() {
            return Err(InstanceError::Forbidden);
        }
        self.schedules
            .as_deref()
            .ok_or_else(|| InstanceError::Conflict("Schedule catalog unavailable.".into()))
    }

    pub async fn upcoming_schedules(
        &self,
        principal: &ActorPrincipal,
        tags: &[String],
        limit: u8,
        now: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<crate::ScheduleDefinition>, InstanceError> {
        let reader = self.schedule_catalog(principal)?;
        if !(1..=10).contains(&limit) {
            return Err(InstanceError::Validation(
                "Choose between one and ten schedules.".into(),
            ));
        }
        let fixed = self
            .schedule_deadlines
            .read()
            .expect("schedule deadlines")
            .clone();
        reader
            .upcoming(
                tags,
                usize::from(limit),
                now,
                fixed,
                &self.catalog_policy(false),
            )
            .await
    }
    pub async fn schedules(
        &self,
        principal: &ActorPrincipal,
        resource: Option<&str>,
        search: &str,
        page: u32,
        page_size: u32,
        attention_only: bool,
    ) -> Result<Vec<crate::ScheduleDefinition>, InstanceError> {
        let reader = self.schedule_catalog(principal)?;
        if page == 0 || !(1..=200).contains(&page_size) {
            return Err(InstanceError::Validation(
                "Expected a positive page and a page size between 1 and 200.".into(),
            ));
        }
        let purposes: Vec<_> = crate::SchedulePurpose::ALL
            .into_iter()
            .filter(|p| resource.is_none_or(|r| p.resource() == r))
            .collect();
        if purposes.is_empty() {
            return Err(InstanceError::Validation(
                "Unknown schedule resource.".into(),
            ));
        }
        let policy = self.catalog_policy(attention_only);
        let mut rows = reader
            .page(
                &purposes,
                search.trim(),
                i64::from(page - 1) * i64::from(page_size),
                i64::from(page_size),
                &policy,
            )
            .await?;
        let deadlines = self.schedule_deadlines.read().expect("schedule deadlines");
        for row in &mut rows {
            if row.purpose.fixed_interval().is_some() {
                row.anchor = deadlines.get(&row.purpose).copied();
            }
            self.apply_schedule_blocker(row, policy.blockers.get(&row.purpose).copied());
        }
        Ok(rows)
    }

    pub fn with_job_history(mut self, repository: Arc<dyn crate::JobHistoryRepository>) -> Self {
        self.job_history = Some(repository);
        self
    }
    pub async fn job_history(
        &self,
        principal: &ActorPrincipal,
        job: Option<&str>,
        hours: u32,
    ) -> Result<crate::JobHistory, InstanceError> {
        if !principal.is_administrator() {
            return Err(InstanceError::Forbidden);
        }
        if !(1..=168).contains(&hours)
            || job.is_some_and(|v| {
                v.len() > 128 || !v.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            })
        {
            return Err(InstanceError::Validation(
                "Choose a job and a history range between 1 and 168 hours.".into(),
            ));
        }
        self.job_history
            .as_ref()
            .ok_or_else(|| InstanceError::Conflict("Job history is unavailable.".into()))?
            .read(job, hours)
            .await
    }

    #[must_use]
    pub fn with_jobs(mut self, reader: Arc<dyn crate::InstanceJobsReader>) -> Self {
        self.jobs = Some(reader);
        self
    }

    pub fn jobs(&self, principal: &ActorPrincipal) -> Result<crate::JobsSnapshot, InstanceError> {
        if !principal.is_administrator() {
            return Err(InstanceError::Forbidden);
        }
        self.jobs
            .as_ref()
            .map(|r| r.snapshot())
            .ok_or_else(|| InstanceError::Conflict("Jobs diagnostics are unavailable.".into()))
    }
    pub fn job_changes(
        &self,
        principal: &ActorPrincipal,
    ) -> Result<tokio::sync::watch::Receiver<u64>, InstanceError> {
        if !principal.is_administrator() {
            return Err(InstanceError::Forbidden);
        }
        self.jobs
            .as_ref()
            .map(|r| r.changes())
            .ok_or_else(|| InstanceError::Conflict("Jobs diagnostics are unavailable.".into()))
    }

    #[must_use]
    pub fn new(repository: Arc<dyn InstanceRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            repository,
            jobs: None,
            schedules: None,
            schedule_changes: tokio::sync::watch::channel(Default::default()).0,
            automation: tokio::sync::watch::channel(None).0,
            schedule_deadlines: Default::default(),
            job_history: None,
            restart: None,
            clock,
            installation_policy: Default::default(),
            installation_changes: tokio::sync::watch::channel(()).0,
        }
    }

    #[must_use]
    pub fn with_installation_policy(mut self, policy: InstallationPolicy) -> Self {
        self.installation_policy = policy;
        self
    }

    pub fn with_restart(mut self, restart: Arc<crate::InstanceRestartService>) -> Self {
        self.restart = Some(restart);
        self
    }

    pub fn restart(&self) -> Option<&Arc<crate::InstanceRestartService>> {
        self.restart.as_ref()
    }

    pub fn installation_policy(&self) -> InstallationPolicy {
        self.installation_policy.clone()
    }

    pub fn installation_changes(&self) -> tokio::sync::watch::Receiver<()> {
        self.installation_changes.subscribe()
    }

    pub async fn installation_settings(&self) -> Result<InstallationSettings, InstanceError> {
        let mut settings = self.repository.installation_settings().await?;
        settings.update_checks_enabled &= self.installation_policy.update_checks_allowed;
        settings.mfa_policy = settings
            .mfa_policy
            .at_least(self.installation_policy.minimum_mfa_policy);
        Ok(settings)
    }

    pub async fn get_installation_settings(
        &self,
        principal: &ActorPrincipal,
    ) -> Result<InstallationSettings, InstanceError> {
        if !principal.is_administrator() {
            return Err(InstanceError::Forbidden);
        }
        self.installation_settings().await
    }

    pub async fn database_connections(
        &self,
        principal: &ActorPrincipal,
    ) -> Result<crate::DatabaseConnections, InstanceError> {
        if !principal.is_administrator() {
            return Err(InstanceError::Forbidden);
        }
        self.repository.database_connections().await
    }

    pub async fn patch_installation_settings(
        &self,
        principal: &ActorPrincipal,
        mut patch: PatchInstallationSettings,
    ) -> Result<InstallationSettings, InstanceError> {
        if !principal.is_administrator() {
            return Err(InstanceError::Forbidden);
        }
        let enabled = match patch.update_checks_enabled {
            PatchField::Value(value) => Some(value),
            PatchField::Missing => None,
            PatchField::Null => {
                return Err(InstanceError::Validation(
                    "updateChecksEnabled cannot be null.".into(),
                ));
            }
        };
        let policy = match patch.mfa_policy {
            PatchField::Value(value) => Some(value),
            PatchField::Missing => None,
            PatchField::Null => {
                return Err(InstanceError::Validation(
                    "mfaPolicy cannot be null.".into(),
                ));
            }
        };
        let interval = match patch.update_check_interval_hours {
            PatchField::Value(value @ (6 | 12 | 24)) => Some(value),
            PatchField::Missing => None,
            _ => {
                return Err(InstanceError::Validation(
                    "Update check frequency must be 6, 12 or 24 hours.".into(),
                ));
            }
        };
        if matches!(&patch.configuration, PatchField::Value(value) if value.is_empty()) {
            return Err(InstanceError::Validation(
                "At least one configuration setting is required.".into(),
            ));
        }
        if matches!(patch.configuration, PatchField::Null) {
            return Err(InstanceError::Validation(
                "configuration cannot be null.".into(),
            ));
        }
        if enabled.is_none()
            && policy.is_none()
            && interval.is_none()
            && matches!(patch.configuration, PatchField::Missing)
        {
            return Err(InstanceError::Validation(
                "At least one installation setting is required.".into(),
            ));
        }
        self.validate_installation_choices(enabled, policy)?;
        patch.deployment_overrides = self.installation_policy.overrides.clone();
        patch.deployment_core_agent_requires_http =
            self.installation_policy.deployment_core_agent_requires_http;
        self.repository
            .patch_installation_settings(principal, &patch, self.clock.now())
            .await?;
        self.installation_changes.send_replace(());
        self.installation_settings().await
    }

    fn validate_installation_choices(
        &self,
        enabled: Option<bool>,
        policy: Option<citadel_identity::MfaPolicy>,
    ) -> Result<(), InstanceError> {
        if enabled == Some(true) && !self.installation_policy.update_checks_allowed {
            return Err(InstanceError::Validation(
                "Update checks are disabled by server configuration.".into(),
            ));
        }
        if policy.is_some_and(|policy| {
            policy.at_least(self.installation_policy.minimum_mfa_policy) != policy
        }) {
            return Err(InstanceError::Validation(
                "The MFA policy cannot be weaker than server configuration.".into(),
            ));
        }
        Ok(())
    }
}
