use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlertChannelInput {
    #[serde(default, deserialize_with = "optional_name")]
    #[schema(nullable)]
    pub name: String,
    pub alert_destination: String,
    pub url: String,
    pub is_active: bool,
}

impl From<AlertChannelInput> for citadel_alerts::AlertChannelConfiguration {
    fn from(value: AlertChannelInput) -> Self {
        Self {
            name: value.name,
            alert_destination: value.alert_destination,
            url: value.url,
            is_active: value.is_active,
        }
    }
}

impl From<citadel_alerts::AlertChannelConfiguration> for AlertChannelInput {
    fn from(value: citadel_alerts::AlertChannelConfiguration) -> Self {
        Self {
            name: value.name,
            alert_destination: value.alert_destination,
            url: value.url,
            is_active: value.is_active,
        }
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

impl From<AlertRuleInput> for citadel_alerts::AlertRuleConfiguration {
    fn from(value: AlertRuleInput) -> Self {
        Self {
            name: value.name,
            description: value.description,
            alert_type: value.alert_type,
            severity: value.severity,
            cooldown_seconds: value.cooldown_seconds,
            required_matches: value.required_matches,
            threshold: value.threshold,
            status: value.status,
            channel_ids: value.channel_ids,
            limited_to: value.limited_to,
            quiet_hours: value.quiet_hours,
        }
    }
}

impl From<citadel_alerts::AlertRuleConfiguration> for AlertRuleInput {
    fn from(value: citadel_alerts::AlertRuleConfiguration) -> Self {
        Self {
            name: value.name,
            description: value.description,
            alert_type: value.alert_type,
            severity: value.severity,
            cooldown_seconds: value.cooldown_seconds,
            required_matches: value.required_matches,
            threshold: value.threshold,
            status: value.status,
            channel_ids: value.channel_ids,
            limited_to: value.limited_to,
            quiet_hours: value.quiet_hours,
        }
    }
}

fn optional_name<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(deserializer)?.unwrap_or_default())
}

fn enabled_status() -> String {
    "Enabled".into()
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
    pub(crate) alert_destination: String,
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
