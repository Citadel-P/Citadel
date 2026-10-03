use super::spec::*;
use citadel_alerts::{AlertRuleStatus, AlertSeverity, AlertType};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlertChannelInput {
    #[serde(default, deserialize_with = "optional_name")]
    #[schema(nullable)]
    pub name: String,
    pub alert_destination: AlertDestination,
    pub url: String,
    pub is_active: bool,
}

impl From<AlertChannelInput> for citadel_alerts::AlertChannelConfiguration {
    fn from(value: AlertChannelInput) -> Self {
        Self {
            name: value.name,
            alert_destination: value.alert_destination.as_str().to_owned(),
            url: value.url,
            is_active: value.is_active,
        }
    }
}

impl TryFrom<citadel_alerts::AlertChannelConfiguration> for AlertChannelInput {
    type Error = citadel_alerts::AlertError;

    fn try_from(value: citadel_alerts::AlertChannelConfiguration) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            alert_destination: decode(value.alert_destination.into())?,
            url: value.url,
            is_active: value.is_active,
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlertRuleInput {
    #[serde(default, deserialize_with = "optional_name")]
    #[schema(nullable)]
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    #[schema(value_type = crate::api::resources::schema_models::alerts::AlertTypeSchema)]
    pub alert_type: AlertType,
    #[schema(value_type = crate::api::resources::schema_models::alerts::AlertSeveritySchema)]
    pub severity: AlertSeverity,
    pub cooldown_seconds: Option<i32>,
    pub required_matches: Option<i32>,
    pub threshold: Option<f64>,
    #[serde(default = "enabled_status")]
    #[schema(value_type = crate::api::resources::schema_models::alerts::AlertRuleStatusSchema)]
    pub status: AlertRuleStatus,
    #[serde(default)]
    pub channel_ids: Vec<Uuid>,
    #[serde(default)]
    pub limited_to: Vec<AlertResourceScope>,
    #[serde(default)]
    pub quiet_hours: Vec<AlertQuietHour>,
}

impl From<AlertRuleInput> for citadel_alerts::AlertRuleConfiguration {
    fn from(value: AlertRuleInput) -> Self {
        Self {
            name: value.name,
            description: value.description,
            alert_type: value.alert_type.as_str().to_owned(),
            severity: value.severity,
            cooldown_seconds: value.cooldown_seconds,
            required_matches: value.required_matches,
            threshold: value.threshold,
            status: value.status,
            channel_ids: value.channel_ids,
            limited_to: value
                .limited_to
                .into_iter()
                .map(|item| serde_json::json!(item))
                .collect(),
            quiet_hours: value
                .quiet_hours
                .into_iter()
                .map(|item| serde_json::json!(item))
                .collect(),
        }
    }
}

impl TryFrom<citadel_alerts::AlertRuleConfiguration> for AlertRuleInput {
    type Error = citadel_alerts::AlertError;

    fn try_from(value: citadel_alerts::AlertRuleConfiguration) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.name,
            description: value.description,
            alert_type: decode(value.alert_type.into())?,
            severity: value.severity,
            cooldown_seconds: value.cooldown_seconds,
            required_matches: value.required_matches,
            threshold: value.threshold,
            status: value.status,
            channel_ids: value.channel_ids,
            limited_to: decode(Value::Array(value.limited_to))?,
            quiet_hours: decode(Value::Array(value.quiet_hours))?,
        })
    }
}

fn optional_name<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

fn enabled_status() -> AlertRuleStatus {
    AlertRuleStatus::Enabled
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenameAlertRuleInput {
    pub id: uuid::Uuid,
    pub name: String,
}

impl From<RenameAlertRuleInput> for citadel_alerts::RenameAlertRuleInput {
    fn from(value: RenameAlertRuleInput) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Ids {
    pub(crate) ids: Vec<Uuid>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolveInput {
    pub(crate) ids: Vec<Uuid>,
    pub(crate) resolution_note: Option<String>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VerifyInput {
    pub(crate) name: String,
    pub(crate) alert_destination: AlertDestination,
    pub(crate) url: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EventFilter {
    #[serde(alias = "ResourceId")]
    pub(crate) resource_id: Option<Uuid>,
    #[serde(alias = "AlertType")]
    pub(crate) alert_type: Option<String>,
    #[serde(alias = "ResourceType")]
    pub(crate) resource_type: Option<String>,
    #[serde(alias = "UnresolvedOnly")]
    pub(crate) unresolved_only: Option<bool>,
    #[serde(alias = "Page")]
    pub(crate) page: Option<i32>,
    #[serde(alias = "PageSize")]
    pub(crate) page_size: Option<i32>,
}
