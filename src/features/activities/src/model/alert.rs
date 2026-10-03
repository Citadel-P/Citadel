use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AlertRuleActivitySnapshot {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "Type")]
    pub alert_type: String,
    pub severity: String,
    pub cooldown_seconds: Option<i32>,
    pub required_matches: Option<i32>,

    pub threshold: Option<serde_json::Number>,
    pub status: String,
    pub channel_ids: Vec<Uuid>,
    pub limited_to: Vec<Value>,
    pub quiet_hours: Vec<Value>,
}
