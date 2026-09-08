#![forbid(unsafe_code)]

pub mod configuration_patch;
mod quiet_hours;
mod rule_metadata;
mod windows_time_zones;
pub use quiet_hours::is_in_quiet_hours;
pub use rule_metadata::{RenameAlertRuleInput, description_patch};

use std::sync::atomic::{AtomicI64, Ordering};

use chrono::{DateTime, Utc};
use citadel_domain::ActorId;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertChannelInput {
    #[serde(default, deserialize_with = "optional_name")]
    pub name: String,
    pub alert_destination: String,
    pub url: String,
    pub is_active: bool,
}

impl AlertChannelInput {
    pub fn validate(&mut self) -> Result<(), AlertError> {
        self.name = self.name.trim().to_owned();
        if self.name.is_empty() {
            self.name = self.alert_destination.clone();
        }
        self.url = self.url.trim().to_owned();
        if self.name.is_empty() || self.name.chars().count() > 128 {
            return Err(AlertError::Validation(
                "Alert Channel name must contain between 1 and 128 characters.".into(),
            ));
        }
        if !matches!(
            self.alert_destination.as_str(),
            "Generic"
                | "Bark"
                | "Discord"
                | "Gotify"
                | "Google_Chat"
                | "IFTTT"
                | "Join"
                | "Lark"
                | "Mattermost"
                | "Matrix"
                | "Ntfy"
                | "OpsGenie"
                | "Pushbullet"
                | "Pushover"
                | "Rocketchat"
                | "Signal"
                | "Slack"
                | "Teams"
                | "Telegram"
                | "WeCom"
                | "Zulip_Chat"
        ) {
            return Err(AlertError::Validation(
                "Alert destination is invalid.".into(),
            ));
        }
        if self.url.is_empty() || self.url.chars().count() > 4096 || !self.url.contains("://") {
            return Err(AlertError::Validation(
                "Alert Channel URL must be a non-empty Shoutrrr URL no longer than 4096 characters."
                    .into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertChannelView {
    pub id: Uuid,
    pub name: String,
    pub alert_destination: String,
    pub url: String,
    pub is_active: bool,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertRuleInput {
    #[serde(default, deserialize_with = "optional_name")]
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub alert_type: String,
    pub severity: String,
    pub cooldown_seconds: Option<i32>,
    pub required_matches: Option<i32>,
    pub threshold: Option<f64>,
    #[serde(default = "enabled_status")]
    pub status: String,
    #[serde(default)]
    pub channel_ids: Vec<Uuid>,
    #[serde(default)]
    pub limited_to: Vec<Value>,
    #[serde(default)]
    pub quiet_hours: Vec<Value>,
}

impl AlertRuleInput {
    // The create contract reports all field errors. Merged PATCH input still
    // uses the domain invariants below, matching the existing API distinction.
    pub fn validate_create(&mut self) -> Result<(), AlertError> {
        let mut errors = std::collections::BTreeMap::new();
        if invalid_cooldown(self.cooldown_seconds) {
            errors.insert(
                "CooldownSeconds".into(),
                vec!["Cooldown must be between 10s and 24h.".into()],
            );
        }
        if self.is_threshold_rule() {
            if self.required_matches.is_none() {
                errors.insert(
                    "RequiredMatches".into(),
                    vec!["'Required Matches' must not be empty.".into()],
                );
            }
            if self.threshold.is_none() {
                errors.insert(
                    "Threshold".into(),
                    vec!["'Threshold' must not be empty.".into()],
                );
            }
        } else {
            if self.required_matches.is_some() {
                errors.insert(
                    "RequiredMatches".into(),
                    vec!["Non-threshold alerts must not define RequiredMatches.".into()],
                );
            }
            if self.threshold.is_some() {
                errors.insert(
                    "Threshold".into(),
                    vec!["Non-threshold alerts must not define Threshold.".into()],
                );
            }
        }
        if !errors.is_empty() {
            return Err(AlertError::FieldValidation(errors));
        }
        self.validate()
    }

    fn is_threshold_rule(&self) -> bool {
        matches!(
            self.alert_type.as_str(),
            "PlatformCpuHigh" | "PlatformRamHigh" | "PlatformDiskHigh"
        )
    }

    pub fn snapshot(&self, id: Uuid) -> citadel_domain::AlertRuleActivitySnapshot {
        citadel_domain::AlertRuleActivitySnapshot {
            id,
            name: self.name.clone(),
            description: self.description.clone(),
            alert_type: self.alert_type.clone(),
            severity: self.severity.clone(),
            cooldown_seconds: self.cooldown_seconds,
            required_matches: self.required_matches,
            threshold: self.threshold.and_then(serde_json::Number::from_f64),
            status: self.status.clone(),
            channel_ids: self.channel_ids.clone(),
            limited_to: self.limited_to.clone(),
            quiet_hours: self.quiet_hours.clone(),
        }
    }
    pub fn validate(&mut self) -> Result<(), AlertError> {
        self.name = self.name.trim().to_owned();
        if self.name.is_empty() {
            self.name = self.alert_type.clone();
        }
        if self.name.is_empty() || self.name.chars().count() > 128 {
            return Err(AlertError::Validation(
                "Alert Rule name must contain between 1 and 128 characters.".into(),
            ));
        }
        if self.alert_type.trim().is_empty() {
            return Err(AlertError::Validation("Alert type is required.".into()));
        }
        if !matches!(self.severity.as_str(), "Info" | "Warning" | "Critical") {
            return Err(AlertError::Validation("Alert severity is invalid.".into()));
        }
        if !matches!(self.status.as_str(), "Enabled" | "Disabled") {
            return Err(AlertError::Validation(
                "Alert Rule status is invalid.".into(),
            ));
        }
        if invalid_cooldown(self.cooldown_seconds) {
            return Err(AlertError::InvalidCooldown);
        }
        if self
            .required_matches
            .is_some_and(|value| !(1..=10_000).contains(&value))
        {
            return Err(AlertError::Validation(
                "Required matches is invalid.".into(),
            ));
        }
        if self.threshold.is_some_and(|value| !value.is_finite()) {
            return Err(AlertError::Validation("Threshold must be finite.".into()));
        }
        let threshold_rule = self.is_threshold_rule();
        if threshold_rule && (self.threshold.is_none() || self.required_matches.is_none()) {
            return Err(AlertError::Validation(
                "Threshold alerts require RequiredMatches and Threshold.".into(),
            ));
        }
        if !threshold_rule && (self.threshold.is_some() || self.required_matches.is_some()) {
            return Err(AlertError::Validation(
                "Non-threshold alerts must not define RequiredMatches or Threshold.".into(),
            ));
        }
        let mut unique = std::collections::HashSet::new();
        if self
            .channel_ids
            .iter()
            .any(|id| id.is_nil() || !unique.insert(*id))
        {
            return Err(AlertError::Validation(
                "Alert Channel IDs must be valid and unique.".into(),
            ));
        }
        quiet_hours::validate(&self.quiet_hours)?;
        Ok(())
    }
}

fn optional_name<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

fn invalid_cooldown(value: Option<i32>) -> bool {
    value.is_some_and(|value| !(10..=86_400).contains(&value))
}

fn enabled_status() -> String {
    "Enabled".into()
}

pub trait AlertEntitlements: Send + Sync {
    fn advanced_alerting(&self) -> BoxFuture<'_, Result<bool, AlertError>>;
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertRuleView {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub alert_type: String,
    pub severity: String,
    pub cooldown_seconds: Option<i32>,
    pub required_matches: Option<i32>,
    pub threshold: Option<f64>,
    pub status: String,
    pub channel_ids: Vec<Uuid>,
    pub limited_to: Vec<Value>,
    pub quiet_hours: Vec<Value>,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl AlertRuleView {
    pub fn snapshot(&self) -> citadel_domain::AlertRuleActivitySnapshot {
        citadel_domain::AlertRuleActivitySnapshot {
            id: self.id,
            name: self.name.clone(),
            description: self.description.clone(),
            alert_type: self.alert_type.clone(),
            severity: self.severity.clone(),
            cooldown_seconds: self.cooldown_seconds,
            required_matches: self.required_matches,
            threshold: self.threshold.and_then(serde_json::Number::from_f64),
            status: self.status.clone(),
            channel_ids: self.channel_ids.clone(),
            limited_to: self.limited_to.clone(),
            quiet_hours: self.quiet_hours.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertEventView {
    pub id: Uuid,
    pub alert_rule_id: Uuid,
    #[serde(rename = "type")]
    pub alert_type: String,
    pub severity: String,
    pub status: String,
    pub message: String,
    pub info: Value,
    pub resource_id: Option<Uuid>,
    pub resource_name: String,
    pub resource_type: String,
    pub acknowledged_by_actor_id: Option<Uuid>,
    pub acknowledged_at: Option<DateTime<Utc>>,
    pub resolved_by_actor_id: Option<Uuid>,
    pub resolved_at: Option<DateTime<Utc>>,
    pub resolution_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewAlertEvent {
    pub alert_rule_id: Uuid,
    pub alert_type: String,
    pub severity: String,
    pub info: Value,
    pub resource_id: Option<Uuid>,
    pub resource_name: String,
    pub resource_type: String,
    pub deduplication_key: String,
}

#[derive(Debug, Clone)]
pub struct AlertObservation {
    pub alert_type: String,
    pub info: Value,
    pub resource_id: Uuid,
    pub resource_name: String,
    pub resource_type: String,
    pub deduplication_component: String,
    pub observed_at: DateTime<Utc>,
    pub value: Option<f64>,
    pub matched: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AlertEventFilter {
    pub resource_id: Option<Uuid>,
    pub alert_type: Option<String>,
    pub resource_type: Option<String>,
    pub unresolved_only: bool,
    pub page: i32,
    pub page_size: i32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertEventPage {
    pub items: Vec<AlertEventView>,
    pub total_count: i64,
    pub page: i32,
    pub page_size: i32,
}

pub trait AlertStore: Send + Sync {
    fn list_channels(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<AlertChannelView>, AlertError>>;
    fn get_channel(&self, id: Uuid) -> BoxFuture<'_, Result<AlertChannelView, AlertError>>;
    fn create_channel<'a>(
        &'a self,
        actor: ActorId,
        input: &'a AlertChannelInput,
    ) -> BoxFuture<'a, Result<AlertChannelView, AlertError>>;
    fn update_channel<'a>(
        &'a self,
        id: Uuid,
        patch: &'a Value,
    ) -> BoxFuture<'a, Result<AlertChannelView, AlertError>>;
    fn delete_channels<'a>(&'a self, ids: &'a [Uuid]) -> BoxFuture<'a, Result<(), AlertError>>;
    fn list_rules(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<AlertRuleView>, AlertError>>;
    fn get_rule(&self, id: Uuid) -> BoxFuture<'_, Result<AlertRuleView, AlertError>>;
    fn create_rule<'a>(
        &'a self,
        actor: ActorId,
        input: &'a AlertRuleInput,
    ) -> BoxFuture<'a, Result<AlertRuleView, AlertError>>;
    fn update_rule<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        patch: &'a Value,
    ) -> BoxFuture<'a, Result<AlertRuleView, AlertError>>;
    fn delete_rules<'a>(&'a self, ids: &'a [Uuid]) -> BoxFuture<'a, Result<(), AlertError>>;
    fn rename_rule<'a>(
        &'a self,
        actor: ActorId,
        input: &'a RenameAlertRuleInput,
    ) -> BoxFuture<'a, Result<AlertRuleView, AlertError>>;
    fn update_rule_description<'a>(
        &'a self,
        id: Uuid,
        description: Option<Option<&'a str>>,
    ) -> BoxFuture<'a, Result<AlertRuleView, AlertError>>;
    fn list_events<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        filter: &'a AlertEventFilter,
    ) -> BoxFuture<'a, Result<AlertEventPage, AlertError>>;
    fn get_event(&self, id: Uuid) -> BoxFuture<'_, Result<AlertEventView, AlertError>>;
    fn unresolved_count(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<i64, AlertError>>;
    fn acknowledge<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), AlertError>>;
    fn resolve<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
        note: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), AlertError>>;
    fn raise<'a>(
        &'a self,
        event: &'a NewAlertEvent,
    ) -> BoxFuture<'a, Result<Option<AlertEventView>, AlertError>>;
    fn process_event<'a>(
        &'a self,
        observation: &'a AlertObservation,
    ) -> BoxFuture<'a, Result<Option<AlertEventView>, AlertError>>;
    fn claim_delivery(
        &self,
        owner: Uuid,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<AlertDeliveryClaim>, AlertError>>;
    fn complete_delivery(&self, id: Uuid, owner: Uuid) -> BoxFuture<'_, Result<bool, AlertError>>;
    fn retry_delivery<'a>(
        &'a self,
        id: Uuid,
        owner: Uuid,
        next_attempt_at: DateTime<Utc>,
        dead_letter: bool,
        error: &'a str,
    ) -> BoxFuture<'a, Result<bool, AlertError>>;
    fn maintain_deliveries(
        &self,
        stale_before: DateTime<Utc>,
        dead_before: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), AlertError>>;
}

pub trait AlertEventSink: Send + Sync {
    fn observe<'a>(
        &'a self,
        observation: &'a AlertObservation,
    ) -> BoxFuture<'a, Result<Option<AlertEventView>, AlertError>>;
}

impl<T> AlertEventSink for T
where
    T: AlertStore + ?Sized,
{
    fn observe<'a>(
        &'a self,
        observation: &'a AlertObservation,
    ) -> BoxFuture<'a, Result<Option<AlertEventView>, AlertError>> {
        self.process_event(observation)
    }
}

pub trait AlertDelivery: Send + Sync {
    fn send<'a>(
        &'a self,
        channel: &'a AlertChannelView,
        event: &'a AlertEventView,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), AlertError>>;
}

#[derive(Debug, Clone)]
pub struct AlertDeliveryClaim {
    pub id: Uuid,
    pub attempt_count: i32,
    pub channel: AlertChannelView,
    pub event: AlertEventView,
}

pub struct AlertDeliveryService {
    store: std::sync::Arc<dyn AlertStore>,
    delivery: std::sync::Arc<dyn AlertDelivery>,
    owner: Uuid,
    stale_after: chrono::Duration,
    maximum_attempts: i32,
    next_maintenance_at: AtomicI64,
}

impl AlertDeliveryService {
    pub fn new(
        store: std::sync::Arc<dyn AlertStore>,
        delivery: std::sync::Arc<dyn AlertDelivery>,
    ) -> Self {
        Self {
            store,
            delivery,
            owner: Uuid::now_v7(),
            stale_after: chrono::Duration::minutes(2),
            maximum_attempts: 8,
            next_maintenance_at: AtomicI64::new(0),
        }
    }

    pub async fn process_one(&self, cancellation: &CancellationToken) -> Result<bool, AlertError> {
        self.maintain_if_due().await?;
        let Some(claim) = self
            .store
            .claim_delivery(self.owner, Utc::now() - self.stale_after)
            .await?
        else {
            return Ok(false);
        };
        match self
            .delivery
            .send(&claim.channel, &claim.event, cancellation)
            .await
        {
            Ok(()) => {
                self.store.complete_delivery(claim.id, self.owner).await?;
            }
            Err(error) => {
                let attempt = claim.attempt_count.saturating_add(1);
                let dead_letter = attempt >= self.maximum_attempts;
                self.store
                    .retry_delivery(
                        claim.id,
                        self.owner,
                        Utc::now() + retry_delay(attempt),
                        dead_letter,
                        &error.to_string(),
                    )
                    .await?;
            }
        }
        Ok(true)
    }

    async fn maintain_if_due(&self) -> Result<(), AlertError> {
        let now = Utc::now();
        let timestamp = now.timestamp();
        let due = self.next_maintenance_at.load(Ordering::Relaxed);
        if timestamp < due
            || self
                .next_maintenance_at
                .compare_exchange(due, timestamp + 60, Ordering::AcqRel, Ordering::Relaxed)
                .is_err()
        {
            return Ok(());
        }
        if let Err(error) = self
            .store
            .maintain_deliveries(now - self.stale_after, now - chrono::Duration::days(30))
            .await
        {
            self.next_maintenance_at.store(0, Ordering::Release);
            return Err(error);
        }
        Ok(())
    }
}

fn retry_delay(attempt: i32) -> chrono::Duration {
    const DELAYS: [i64; 8] = [1, 5, 30, 120, 600, 1_800, 1_800, 1_800];
    let index = usize::try_from(attempt.saturating_sub(1))
        .unwrap_or(DELAYS.len() - 1)
        .min(DELAYS.len() - 1);
    chrono::Duration::seconds(DELAYS[index])
}

#[derive(Debug, thiserror::Error)]
pub enum AlertError {
    #[error("one or more validation errors occurred")]
    FieldValidation(std::collections::BTreeMap<String, Vec<String>>),
    #[error("Cooldown must be between 10s and 24h. (Parameter 'cooldownSeconds')")]
    InvalidCooldown,
    #[error("The provided alert rule does not exist")]
    RuleNotFound,
    #[error("Advanced alerting requires a license")]
    LicenseRequired,
    #[error("{0}")]
    Validation(String),
    #[error("Alert resource was not found")]
    NotFound,
    #[error("{0}")]
    Conflict(String),
    #[error("alert storage failed: {0}")]
    Storage(String),
    #[error("alert delivery failed: {0}")]
    Delivery(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_validation_collects_field_errors_and_preserves_cooldown_boundaries() {
        let mut input: AlertRuleInput = serde_json::from_value(serde_json::json!({
            "type":"PlatformCpuHigh","severity":"Warning","cooldownSeconds":5
        }))
        .unwrap();
        let AlertError::FieldValidation(fields) = input.validate_create().unwrap_err() else {
            panic!("expected field-level validation");
        };
        assert_eq!(
            serde_json::to_value(fields).unwrap(),
            serde_json::json!({
                "CooldownSeconds":["Cooldown must be between 10s and 24h."],
                "RequiredMatches":["'Required Matches' must not be empty."],
                "Threshold":["'Threshold' must not be empty."]
            })
        );
        input.required_matches = Some(3);
        input.threshold = Some(85.0);
        for (cooldown, valid) in [
            (None, true),
            (Some(9), false),
            (Some(10), true),
            (Some(86400), true),
            (Some(86401), false),
        ] {
            input.cooldown_seconds = cooldown;
            assert_eq!(input.validate_create().is_ok(), valid);
            if valid {
                assert!(input.validate().is_ok());
            } else {
                assert!(matches!(input.validate(), Err(AlertError::InvalidCooldown)));
            }
        }
    }

    #[test]
    fn accepts_supported_shoutrrr_urls_and_rejects_unknown_destinations() {
        let mut input = AlertChannelInput {
            name: "ops".into(),
            alert_destination: "Telegram".into(),
            url: "telegram://token@telegram?chats=123".into(),
            is_active: true,
        };
        input.validate().unwrap();
        input.alert_destination = "Unknown".into();
        assert!(matches!(input.validate(), Err(AlertError::Validation(_))));
    }

    #[test]
    fn rejects_duplicate_channel_bindings() {
        let id = Uuid::now_v7();
        let mut input = AlertRuleInput {
            name: "failure".into(),
            description: None,
            alert_type: "BuildRunFailed".into(),
            severity: "Critical".into(),
            cooldown_seconds: Some(60),
            required_matches: None,
            threshold: None,
            status: "Enabled".into(),
            channel_ids: vec![id, id],
            limited_to: vec![],
            quiet_hours: vec![],
        };
        assert!(matches!(input.validate(), Err(AlertError::Validation(_))));
    }

    #[test]
    fn delivery_retry_backoff_is_bounded() {
        assert_eq!(retry_delay(1), chrono::Duration::seconds(1));
        assert_eq!(retry_delay(4), chrono::Duration::minutes(2));
        assert_eq!(retry_delay(i32::MAX), chrono::Duration::minutes(30));
    }
}
