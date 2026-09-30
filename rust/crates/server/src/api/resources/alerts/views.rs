use super::spec::*;
use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlertChannelView {
    #[schema(required = true)]
    pub capabilities: Option<crate::api::resources::platforms::views::ResourceCapabilitiesView>,
    pub id: Uuid,
    pub name: String,
    pub alert_destination: AlertDestination,
    pub url: String,
    pub is_active: bool,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<citadel_alerts::AlertChannel> for AlertChannelView {
    type Error = citadel_alerts::AlertError;

    fn try_from(value: citadel_alerts::AlertChannel) -> Result<Self, Self::Error> {
        Ok(Self {
            capabilities: None,
            id: value.id,
            name: value.name,
            alert_destination: decode(value.alert_destination.into())?,
            url: value.url,
            is_active: value.is_active,
            created_by_actor_id: value.created_by_actor_id,
            created_at: value.created_at,
        })
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlertRuleView {
    #[schema(required = true)]
    pub capabilities: Option<crate::api::resources::platforms::views::ResourceCapabilitiesView>,
    pub id: Uuid,
    pub name: String,
    #[schema(required = true)]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    #[schema(required = true)]
    pub cooldown_seconds: Option<i32>,
    #[schema(required = true)]
    pub required_matches: Option<i32>,
    #[schema(required = true)]
    pub threshold: Option<f64>,
    pub status: AlertRuleStatus,
    pub channel_ids: Vec<Uuid>,
    pub limited_to: Vec<AlertResourceScope>,
    pub quiet_hours: Vec<AlertQuietHour>,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
}

impl TryFrom<citadel_alerts::AlertRule> for AlertRuleView {
    type Error = citadel_alerts::AlertError;

    fn try_from(value: citadel_alerts::AlertRule) -> Result<Self, Self::Error> {
        Ok(Self {
            capabilities: None,
            id: value.id,
            name: value.name,
            description: value.description,
            alert_type: decode(value.alert_type.into())?,
            severity: decode(value.severity.into())?,
            cooldown_seconds: value.cooldown_seconds,
            required_matches: value.required_matches,
            threshold: value.threshold,
            status: decode(value.status.into())?,
            channel_ids: value.channel_ids,
            limited_to: decode(Value::Array(value.limited_to))?,
            quiet_hours: decode(Value::Array(value.quiet_hours))?,
            created_by_actor_id: value.created_by_actor_id,
            created_at: value.created_at,
        })
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct AlertRuleListItem {
    #[serde(flatten)]
    pub rule: AlertRuleView,
    pub channels: Vec<AlertChannelView>,
}

impl TryFrom<citadel_alerts::AlertRuleListItem> for AlertRuleListItem {
    type Error = citadel_alerts::AlertError;

    fn try_from(value: citadel_alerts::AlertRuleListItem) -> Result<Self, Self::Error> {
        Ok(Self {
            rule: value.rule.try_into()?,
            channels: value
                .channels
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
        })
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlertEventView {
    pub id: Uuid,
    pub alert_rule_id: Uuid,
    #[serde(rename = "type")]
    pub alert_type: AlertType,
    pub severity: AlertSeverity,
    pub status: AlertEventStatus,
    pub message: String,
    /// Event-specific details supplied by the alert emitter.
    pub info: Value,
    #[schema(required = true)]
    pub resource_id: Option<Uuid>,
    pub resource_name: String,
    #[schema(required = true)]
    pub resource_path: Option<String>,
    pub resource_type: AlertResourceType,
    #[schema(required = true)]
    pub acknowledged_by_actor_id: Option<Uuid>,
    #[schema(required = true)]
    pub acknowledged_at: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub resolved_by_actor_id: Option<Uuid>,
    #[schema(required = true)]
    pub resolved_at: Option<DateTime<Utc>>,
    #[schema(required = true)]
    pub actor_id: Option<Uuid>,
    #[schema(required = true)]
    pub actor_name: Option<String>,
    #[schema(value_type = Option<crate::api::resources::vocabulary::ActorTypeSchema>, required = true)]
    pub actor_type: Option<citadel_identity::ActorType>,
    #[schema(required = true)]
    pub resolution_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<citadel_alerts::AlertEvent> for AlertEventView {
    type Error = citadel_alerts::AlertError;

    fn try_from(value: citadel_alerts::AlertEvent) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            alert_rule_id: value.alert_rule_id,
            alert_type: decode(value.alert_type.into())?,
            severity: decode(value.severity.into())?,
            status: decode(value.status.into())?,
            message: value.message,
            info: value.info,
            resource_id: value.resource_id,
            resource_path: alert_resource_path(&value.resource_type, value.resource_id),
            resource_name: value.resource_name,
            resource_type: decode(value.resource_type.into())?,
            acknowledged_by_actor_id: value.acknowledged_by_actor_id,
            acknowledged_at: value.acknowledged_at,
            resolved_by_actor_id: value.resolved_by_actor_id,
            resolved_at: value.resolved_at,
            actor_id: value.actor_id,
            actor_name: value.actor_name,
            actor_type: decode(serde_json::json!(value.actor_type))?,
            resolution_note: value.resolution_note,
            created_at: value.created_at,
            updated_at: value.updated_at,
        })
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

impl TryFrom<citadel_alerts::AlertEventPage> for AlertEventPage {
    type Error = citadel_alerts::AlertError;

    fn try_from(value: citadel_alerts::AlertEventPage) -> Result<Self, Self::Error> {
        Ok(Self {
            items: value
                .items
                .into_iter()
                .map(TryInto::try_into)
                .collect::<Result<_, _>>()?,
            total_count: value.total_count,
            page: value.page,
            page_size: value.page_size,
        })
    }
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Channels {
    pub(crate) capabilities: crate::api::resources::platforms::views::ResourceCapabilitiesView,
    pub(crate) channels: Vec<AlertChannelView>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Rules {
    pub(crate) capabilities: crate::api::resources::platforms::views::ResourceCapabilitiesView,
    pub(crate) alert_rules: Vec<AlertRuleListItem>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = server::alerts_http::Events)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Events {
    pub(crate) paged_result: AlertEventPage,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Count {
    pub(crate) count: i64,
}

fn alert_resource_path(kind: &str, id: Option<Uuid>) -> Option<String> {
    let id = id?;
    let prefix = match kind {
        "Platform" => "platforms",
        "Deployment" => "deployments",
        "Stack" => "stacks",
        "GitRepository" => "git-repos",
        "AutomationAction" => "automation",
        "Build" => "builds",
        "SwarmService" => "swarm-services",
        "License" => return Some("/license".into()),
        _ => return None,
    };
    Some(format!("/{prefix}/edit/{id}"))
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AlertRuleConfig {
    pub id: Uuid,
    pub is_system: bool,
    #[serde(flatten)]
    pub configuration: super::requests::AlertRuleInput,
}

impl TryFrom<citadel_alerts::AlertRule> for AlertRuleConfig {
    type Error = citadel_alerts::AlertError;
    fn try_from(rule: citadel_alerts::AlertRule) -> Result<Self, Self::Error> {
        Ok(Self {
            id: rule.id,
            is_system: rule.created_by_actor_id == Uuid::from_u128(1),
            configuration: citadel_alerts::AlertRuleConfiguration::from(&rule).try_into()?,
        })
    }
}
