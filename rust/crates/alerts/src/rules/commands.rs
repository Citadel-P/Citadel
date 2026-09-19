use crate::configuration::{enabled_status, optional_name};
use crate::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlertRuleConfiguration {
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

impl AlertRuleConfiguration {
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

fn invalid_cooldown(value: Option<i32>) -> bool {
    value.is_some_and(|value| !(10..=86_400).contains(&value))
}

impl From<&AlertRule> for AlertRuleConfiguration {
    fn from(value: &AlertRule) -> Self {
        Self {
            name: value.name.clone(),
            description: value.description.clone(),
            alert_type: value.alert_type.clone(),
            severity: value.severity.clone(),
            cooldown_seconds: value.cooldown_seconds,
            required_matches: value.required_matches,
            threshold: value.threshold,
            status: value.status.clone(),
            channel_ids: value.channel_ids.clone(),
            limited_to: value.limited_to.clone(),
            quiet_hours: value.quiet_hours.clone(),
        }
    }
}
