use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct AutomationActionActivitySnapshot {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub code: String,
    pub default_args_json: String,
    pub enabled: bool,
    pub schedule_enabled: bool,
    pub schedule_cron: Option<String>,
    pub schedule_time_zone: String,
    pub webhook: Option<Value>,
    pub timeout_seconds: i32,
    pub alert_on_failure: bool,
    pub run_as_actor_id: Uuid,
}
