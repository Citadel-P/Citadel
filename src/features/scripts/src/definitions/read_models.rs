use uuid::Uuid;

/// Fields needed to decide whether a scheduled script is due; code is loaded at enqueue.
#[derive(Debug, Clone)]
pub struct ScheduledScript {
    pub id: Uuid,
    pub schedule_cron: Option<String>,
    pub schedule_time_zone: String,
    pub timeout_seconds: i32,
}
