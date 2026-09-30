use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AlertEvent {
    pub id: Uuid,
    pub alert_rule_id: Uuid,
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
    pub actor_id: Option<Uuid>,
    pub actor_name: Option<String>,
    pub actor_type: Option<String>,
    pub resolution_note: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
