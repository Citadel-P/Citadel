use uuid::Uuid;

/// Fields needed to decide whether a scheduled script is due; code is loaded at enqueue.
#[derive(Debug, Clone)]
pub struct ScheduledScript {
    pub id: Uuid,
    pub schedule_cron: Option<String>,
    pub schedule_time_zone: String,
    pub timeout_seconds: i32,
}

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
