use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AlertRule {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
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

impl AlertRule {
    pub fn snapshot(&self) -> citadel_activities::AlertRuleActivitySnapshot {
        citadel_activities::AlertRuleActivitySnapshot {
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
