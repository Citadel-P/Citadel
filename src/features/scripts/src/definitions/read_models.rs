use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ScriptSummary {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub enabled: bool,
    pub control_state: citadel_primitives::ResourceControlState,
    pub schedule_enabled: bool,
    pub schedule_cron: Option<String>,
    pub tags: Vec<citadel_tags::TagSummary>,
    pub latest_run: Option<crate::ScriptRun>,
}
