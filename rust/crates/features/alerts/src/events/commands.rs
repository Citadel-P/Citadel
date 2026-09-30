use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct NewAlertEvent {
    pub alert_rule_id: Uuid,
    pub alert_type: String,
    pub severity: crate::AlertSeverity,
    pub info: Value,
    pub resource_id: Option<Uuid>,
    pub resource_name: String,
    pub resource_type: String,
    pub deduplication_key: String,
}
