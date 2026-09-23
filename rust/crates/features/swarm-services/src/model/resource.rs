use chrono::{DateTime, Utc};

use crate::*;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwarmServiceOperation {
    pub id: Uuid,
    pub kind: String,
    pub state: String,
    pub prepared_at: DateTime<Utc>,
    pub attempted_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result_code: Option<String>,
    pub warnings: Vec<String>,
    pub result_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SwarmService {
    pub id: Uuid,
    pub platform_id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub docker_name: String,
    pub docker_service_id: Option<String>,
    pub spec: SwarmServiceSpec,
    pub health: String,
    pub synchronization_state: String,
    pub control_state: String,
    pub auto_update_state: AutoUpdateState,
    pub applied_image_digest: Option<String>,
    pub has_pending_desired_changes: bool,
    pub has_runtime_drift: bool,
    pub row_version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
