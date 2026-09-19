use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
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
impl From<citadel_alerts::AlertChannel> for AlertChannelView {
    fn from(value: citadel_alerts::AlertChannel) -> Self {
        Self {
            id: value.id,
            name: value.name,
            alert_destination: value.alert_destination,
            url: value.url,
            is_active: value.is_active,
            created_by_actor_id: value.created_by_actor_id,
            created_at: value.created_at,
        }
    }
}
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
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
impl From<citadel_alerts::AlertRule> for AlertRuleView {
    fn from(value: citadel_alerts::AlertRule) -> Self {
        Self {
            id: value.id,
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
            created_by_actor_id: value.created_by_actor_id,
            created_at: value.created_at,
        }
    }
}
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct AlertRuleListItem {
    #[serde(flatten)]
    pub rule: AlertRuleView,
    pub channels: Vec<AlertChannelView>,
}
impl From<citadel_alerts::AlertRuleListItem> for AlertRuleListItem {
    fn from(value: citadel_alerts::AlertRuleListItem) -> Self {
        Self {
            rule: value.rule.into(),
            channels: value.channels.into_iter().map(Into::into).collect(),
        }
    }
}
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
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
impl From<citadel_alerts::AlertEvent> for AlertEventView {
    fn from(value: citadel_alerts::AlertEvent) -> Self {
        Self {
            id: value.id,
            alert_rule_id: value.alert_rule_id,
            alert_type: value.alert_type,
            severity: value.severity,
            status: value.status,
            message: value.message,
            info: value.info,
            resource_id: value.resource_id,
            resource_name: value.resource_name,
            resource_type: value.resource_type,
            acknowledged_by_actor_id: value.acknowledged_by_actor_id,
            acknowledged_at: value.acknowledged_at,
            resolved_by_actor_id: value.resolved_by_actor_id,
            resolved_at: value.resolved_at,
            resolution_note: value.resolution_note,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlertEventPage {
    pub items: Vec<AlertEventView>,
    pub total_count: i64,
    pub page: i32,
    pub page_size: i32,
}
impl From<citadel_alerts::AlertEventPage> for AlertEventPage {
    fn from(value: citadel_alerts::AlertEventPage) -> Self {
        Self {
            items: value.items.into_iter().map(Into::into).collect(),
            total_count: value.total_count,
            page: value.page,
            page_size: value.page_size,
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct Channels {
    pub(super) channels: Vec<AlertChannelView>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct Rules {
    pub(super) alert_rules: Vec<AlertRuleListItem>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = server::alerts_http::Events)]
#[serde(rename_all = "camelCase")]
pub(super) struct Events {
    pub(super) paged_result: AlertEventPage,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct Count {
    pub(super) count: i64,
}
