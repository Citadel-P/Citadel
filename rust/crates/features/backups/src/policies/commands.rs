use crate::*;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupPolicyConfiguration {
    pub name: String,
    pub description: Option<String>,
    pub source: crate::spec::BackupSourceSpec,
    pub backup_repository_id: Uuid,
    pub enabled: bool,
    pub cron: Option<String>,
    pub time_zone: Option<String>,
    pub webhook: Option<citadel_primitives::WebhookConfig>,
    pub keep_last_successful: Option<i32>,
    pub timeout_seconds: Option<i32>,
    pub alert_on_failure: bool,
    pub run_as_actor_id: Option<Uuid>,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
}

impl BackupPolicyConfiguration {
    pub fn validate(&mut self, actor: ActorId) -> Result<(), BackupError> {
        validate_name(&mut self.name, "Backup Policy")?;
        if self.backup_repository_id.is_nil() {
            return Err(BackupError::Validation(
                "Backup Repository is required.".into(),
            ));
        }
        self.source.validate()?;
        if let Some(webhook) = &mut self.webhook {
            webhook.secret =
                citadel_primitives::normalization::optional_text(webhook.secret.take());
            webhook.branch_filter =
                citadel_primitives::normalization::optional_text(webhook.branch_filter.take());
            webhook
                .validate()
                .map_err(|e| BackupError::Validation(e.into()))?;
            if webhook.enabled && webhook.secret.is_none() {
                return Err(BackupError::Validation(
                    "An enabled Webhook requires a secret.".into(),
                ));
            }
        }
        let keep = self.keep_last_successful.unwrap_or(14);
        if !(1..=1000).contains(&keep) {
            return Err(BackupError::Validation(
                "Backup retention must be between 1 and 1000 snapshots.".into(),
            ));
        }
        let timeout = self.timeout_seconds.unwrap_or(14400);
        if !(60..=86400).contains(&timeout) {
            return Err(BackupError::Validation(
                "Backup timeout must be between 60 and 86400 seconds.".into(),
            ));
        }
        self.keep_last_successful = Some(keep);
        self.timeout_seconds = Some(timeout);
        if self
            .cron
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        {
            let zone = self.time_zone.as_deref().unwrap_or("UTC");
            if CronSchedule::parse(self.cron.as_deref().unwrap_or_default(), zone).is_err() {
                return Err(BackupError::Validation(
                    "Backup schedule requires a valid five-field cron expression and time zone."
                        .into(),
                ));
            }
            self.time_zone = Some(zone.to_owned());
        } else {
            self.cron = None;
            self.time_zone = Some(self.time_zone.take().unwrap_or_else(|| "UTC".to_owned()));
        }
        self.run_as_actor_id = Some(self.run_as_actor_id.unwrap_or(actor.value()));
        if self.tag_ids.len() > 100 {
            return Err(BackupError::Validation(
                "A Backup Policy cannot have more than 100 Tags.".to_owned(),
            ));
        }
        self.tag_ids.sort_unstable();
        self.tag_ids.dedup();
        Ok(())
    }
}
