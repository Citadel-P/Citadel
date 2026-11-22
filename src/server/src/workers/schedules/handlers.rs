use chrono::{DateTime, Utc};
use citadel_instance::{SchedulePurpose as Purpose, ScheduleReader};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub(crate) struct Handlers {
    pub reader: Arc<dyn ScheduleReader>,
    pub admission: citadel_runtime::admission::AdmissionGate,
    pub git: Arc<citadel_git::GitRepositoryExecutionService>,
    pub images: Arc<citadel_platforms::image_updates::ImageUpdateService>,
    pub deployments: Arc<citadel_deployments::DeploymentService>,
    pub stacks: Arc<citadel_stacks::StackService>,
    pub services: Arc<citadel_swarm_services::SwarmServiceService>,
    pub scripts: Arc<citadel_scripts::ScriptService>,
    pub backups: Arc<citadel_backups::BackupService>,
    pub backups_enabled: bool,
    pub agents: citadel_updates::AgentUpdateService,
    pub checker: citadel_updates::ReleaseCheckService,
    pub instance: Arc<citadel_instance::InstanceService>,
    pub stack_gate: tokio::sync::Mutex<()>,
}

pub(crate) trait Dispatch: Send + Sync + 'static {
    fn apply_policy(&self, _: &mut citadel_instance::ScheduleDefinition) {}
    fn eligible(&self, _: Purpose) -> bool {
        true
    }
    fn refresh_eligibility(&self) -> impl Future<Output = Result<(), String>> + Send {
        async { Ok(()) }
    }
    fn reader(&self) -> &dyn ScheduleReader;
    fn notify(&self);
    fn set_deadline(&self, p: Purpose, at: DateTime<Utc>);
    fn occurrence(
        &self,
        p: Purpose,
        id: uuid::Uuid,
        minute: DateTime<Utc>,
        cancel: &CancellationToken,
    ) -> impl Future<Output = Result<bool, String>> + Send;
    fn batch(
        &self,
        p: Purpose,
        reference: Option<uuid::Uuid>,
        cancel: &CancellationToken,
    ) -> impl Future<Output = Result<(), String>> + Send;
    fn deadline(
        &self,
        p: Purpose,
    ) -> impl Future<Output = Result<Option<DateTime<Utc>>, String>> + Send;
}
impl Dispatch for Handlers {
    fn apply_policy(&self, d: &mut citadel_instance::ScheduleDefinition) {
        self.instance.apply_schedule_policy(d);
    }
    fn eligible(&self, p: Purpose) -> bool {
        self.instance.schedule_blocker(p).is_none()
    }
    async fn refresh_eligibility(&self) -> Result<(), String> {
        let allowed = match self.scripts.ensure_automated_operations().await {
            Ok(()) => Some(true),
            Err(citadel_scripts::ScriptError::LicenseRequired) => Some(false),
            Err(e) => {
                self.instance.observe_automated_operations(None);
                return Err(e.to_string());
            }
        };
        self.instance.observe_automated_operations(allowed);
        Ok(())
    }
    fn reader(&self) -> &dyn ScheduleReader {
        self.reader.as_ref()
    }
    fn notify(&self) {
        self.instance.notify_schedules();
    }
    fn set_deadline(&self, p: Purpose, at: DateTime<Utc>) {
        self.instance.set_schedule_deadline(p, at);
    }
    async fn deadline(&self, p: Purpose) -> Result<Option<DateTime<Utc>>, String> {
        if p == Purpose::Release {
            let settings = self
                .instance
                .installation_settings()
                .await
                .map_err(|e| e.to_string())?;
            return super::super::updates::checker::deadline(
                &self.checker,
                settings.initialized && settings.update_checks_enabled,
                settings.update_check_interval_hours,
            )
            .await;
        }
        self.reader
            .batch_deadline(p)
            .await
            .map_err(|e| e.to_string())
    }
    async fn occurrence(
        &self,
        purpose: Purpose,
        id: uuid::Uuid,
        minute: DateTime<Utc>,
        cancel: &CancellationToken,
    ) -> Result<bool, String> {
        let Some(_permit) = self.admission.try_enter("Scheduled occurrence") else {
            return Err("Core is draining scheduled work.".into());
        };
        if cancel.is_cancelled() {
            return Ok(false);
        }
        match purpose {
            Purpose::Script => self
                .scripts
                .accept_occurrence(id, minute)
                .await
                .map_err(|e| e.to_string()),
            Purpose::Backup if self.backups_enabled => self
                .backups
                .accept_occurrence(id, minute)
                .await
                .map_err(|e| e.to_string()),
            Purpose::Install => self
                .agents
                .accept_occurrence(minute)
                .await
                .map_err(|e| e.to_string()),
            Purpose::Backup => Ok(false),
            Purpose::Release
            | Purpose::Git
            | Purpose::Image
            | Purpose::DeploymentImages
            | Purpose::StackImages
            | Purpose::ServiceImages
            | Purpose::StackGit => unreachable!("batch schedule used as a calendar"),
        }
    }

    async fn batch(
        &self,
        purpose: Purpose,
        reference: Option<uuid::Uuid>,
        cancel: &CancellationToken,
    ) -> Result<(), String> {
        let Some(_permit) = self.admission.enter("Scheduled checks", cancel).await else {
            return Ok(());
        };
        let _batch = self.instance.start_schedule_batch(purpose);
        match purpose {
            Purpose::Git => self
                .git
                .enqueue_due(100)
                .await
                .map(|_| ())
                .map_err(|e| e.to_string()),
            Purpose::Image => {
                super::super::image_updates::sweep(cancel.clone(), self.images.clone())
                    .await
                    .map_err(|e| e.to_string())
            }
            Purpose::DeploymentImages => self
                .deployments
                .run_scheduled_update_checks(
                    cancel,
                    super::super::deployments::report_update_problem,
                )
                .await
                .map_err(|e| e.to_string()),
            Purpose::ServiceImages => self
                .services
                .run_scheduled_update_checks(cancel)
                .await
                .map(|_| ())
                .map_err(|e| e.to_string()),
            Purpose::StackImages | Purpose::StackGit => {
                let _guard = tokio::select! {()=cancel.cancelled()=>return Ok(()),g=self.stack_gate.lock()=>g};
                let _iteration =
                    citadel_runtime::runtime_metrics::RuntimeWork::StackUpdates.start();
                self.stacks
                    .run_scoped_update_check_pass(
                        purpose == Purpose::StackImages,
                        reference,
                        cancel,
                    )
                    .await
                    .map_err(|e| e.to_string())
            }
            Purpose::Release => {
                let settings = self
                    .instance
                    .installation_settings()
                    .await
                    .map_err(|e| e.to_string())?;
                super::super::updates::checker::check(
                    &self.checker,
                    settings.initialized && settings.update_checks_enabled,
                    settings.update_check_interval_hours,
                    cancel,
                )
                .await
            }
            Purpose::Script | Purpose::Backup | Purpose::Install => {
                unreachable!("calendar schedules accept occurrences")
            }
        }
    }
}
