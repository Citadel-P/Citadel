use crate::*;
use citadel_domain::ActorId;
use citadel_git::repositories::webhooks::RepoWebhookConfig;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutomationActionConfiguration {
    pub name: String,
    pub description: Option<String>,
    pub code: String,
    pub default_args_json: Option<String>,
    pub enabled: bool,
    pub schedule_enabled: bool,
    pub schedule_cron: Option<String>,
    pub schedule_time_zone: Option<String>,
    pub webhook: Option<RepoWebhookConfig>,
    pub timeout_seconds: Option<i32>,
    pub alert_on_failure: bool,
    pub run_as_actor_id: Option<Uuid>,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
}

impl AutomationActionConfiguration {
    pub fn validate(&mut self, default_actor: ActorId) -> Result<(), AutomationError> {
        self.name = self.name.trim().to_owned();
        if self.name.is_empty() || self.name.chars().count() > 128 {
            return Err(AutomationError::Validation(
                "Automation Action name must contain between 1 and 128 characters.".to_owned(),
            ));
        }
        if self
            .description
            .as_ref()
            .is_some_and(|value| value.chars().count() > 600)
        {
            return Err(AutomationError::Validation(
                "Automation Action description cannot exceed 600 characters.".to_owned(),
            ));
        }
        if self.code.trim().is_empty() || self.code.len() > 256 * 1024 {
            return Err(AutomationError::Validation(
                "Automation code must contain between 1 byte and 256 KiB.".to_owned(),
            ));
        }
        let args = self.default_args_json.as_deref().unwrap_or("{}");
        if args.len() > 64 * 1024 {
            return Err(AutomationError::Validation(
                "Automation Action default arguments cannot exceed 64 KiB.".to_owned(),
            ));
        }
        if !serde_json::from_str::<Value>(args).is_ok_and(|value| value.is_object()) {
            return Err(AutomationError::Validation(
                "Default arguments must be a JSON object.".to_owned(),
            ));
        }
        let timeout = self.timeout_seconds.unwrap_or(300);
        if !(1..=86_400).contains(&timeout) {
            return Err(AutomationError::Validation(
                "Automation timeout must be between 1 and 86400 seconds.".to_owned(),
            ));
        }
        if self.schedule_enabled
            && self
                .schedule_cron
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            return Err(AutomationError::Validation(
                "Enabled schedules require a cron expression.".to_owned(),
            ));
        }
        if self.tag_ids.len() > 100 {
            return Err(AutomationError::Validation(
                "At most 100 tags can be assigned.".into(),
            ));
        }
        self.tag_ids.sort_unstable();
        self.tag_ids.dedup();
        if self
            .schedule_cron
            .as_ref()
            .is_some_and(|value| value.len() > 128)
            || self
                .schedule_time_zone
                .as_ref()
                .is_some_and(|value| value.len() > 128)
            || self.run_as_actor_id.is_some_and(|id| id.is_nil())
        {
            return Err(AutomationError::Validation("Schedule fields must be at most 128 characters and Run-as Actor must not be empty.".into()));
        }
        if let Some(webhook) = &self.webhook {
            webhook
                .validate()
                .map_err(|error| AutomationError::Validation(error.to_string()))?;
        }
        self.default_args_json = Some(args.to_owned());
        self.timeout_seconds = Some(timeout);
        self.schedule_time_zone = Some(
            self.schedule_time_zone
                .take()
                .unwrap_or_else(|| "UTC".to_owned()),
        );
        self.run_as_actor_id = Some(self.run_as_actor_id.unwrap_or(default_actor.value()));
        Ok(())
    }
}
