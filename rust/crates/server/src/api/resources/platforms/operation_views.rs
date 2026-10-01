//! Native contracts for runtime operations that are not inventory projections.
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EdgeEnrollmentView {
    pub enrollment_id: Uuid,
    pub platform_id: Uuid,
    pub token: String,
    pub expires_at_utc: DateTime<Utc>,
    pub instructions: EdgeInstructionsView,
}
#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EdgeInstructionsView {
    pub core_url: String,
    pub environment: BTreeMap<String, String>,
    pub agent_image: String,
    pub docker_run_command: String,
}
#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EdgeStatusView {
    pub connection_status: String,
    pub last_connected_at_utc: Option<DateTime<Utc>>,
    pub last_disconnected_at_utc: Option<DateTime<Utc>>,
    pub last_heartbeat_at_utc: Option<DateTime<Utc>>,
    pub last_seen_version: Option<String>,
    pub last_seen_hostname: Option<String>,
    pub agent_fingerprint: Option<String>,
    pub protocol_version: Option<i32>,
    pub revoked_at_utc: Option<DateTime<Utc>>,
    pub enrollment_expires_at_utc: Option<DateTime<Utc>>,
}
impl From<citadel_platforms::edge_management::EdgeStatus> for EdgeStatusView {
    fn from(value: citadel_platforms::edge_management::EdgeStatus) -> Self {
        Self {
            connection_status: value.connection_status,
            last_connected_at_utc: value.last_connected_at_utc,
            last_disconnected_at_utc: value.last_disconnected_at_utc,
            last_heartbeat_at_utc: value.last_heartbeat_at_utc,
            last_seen_version: value.last_seen_version,
            last_seen_hostname: value.last_seen_hostname,
            agent_fingerprint: value.agent_fingerprint,
            protocol_version: value.protocol_version,
            revoked_at_utc: value.revoked_at_utc,
            enrollment_expires_at_utc: value.enrollment_expires_at_utc,
        }
    }
}

#[derive(Serialize, utoipa::ToSchema)]
pub struct ExposedPortsView {
    pub ports: Vec<String>,
}
#[derive(Serialize, utoipa::ToSchema)]
pub struct DeleteImagesView {
    pub items: Vec<DeleteImageView>,
}
#[derive(Serialize, utoipa::ToSchema)]
pub struct DeleteImageView {
    pub result: BTreeMap<String, String>,
}
#[derive(Serialize, utoipa::ToSchema)]
pub struct ConfigContentView {
    pub content: String,
}

pub struct StatsHours;
impl utoipa::PartialSchema for StatsHours {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        utoipa::openapi::schema::ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::Type::Integer)
            .enum_values(Some(citadel_platforms::StatsWindow::HOURS))
            .default(Some(serde_json::json!(24)))
            .into()
    }
}
impl utoipa::ToSchema for StatsHours {}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServiceInspectionView {
    pub id: String,
    pub version_index: u64,
    pub name: String,
    pub mode: String,
    pub image: String,
    pub running_task_count: i32,
    pub desired_task_count: i32,
    pub update_state: String,
    pub update_message: Option<String>,
    pub ports: Vec<String>,
    pub network_ids: Vec<String>,
    pub secret_ids: Vec<String>,
    pub config_ids: Vec<String>,
    pub labels: BTreeMap<String, String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NodeInspectionView {
    pub id: String,
    pub version_index: i64,
    pub hostname: String,
    pub role: String,
    pub is_leader: bool,
    pub reachability: String,
    pub status: String,
    pub status_message: Option<String>,
    pub availability: String,
    pub engine_version: String,
    pub operating_system: String,
    pub architecture: String,
    pub address: String,
    pub labels: BTreeMap<String, String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub running_task_count: i32,
    pub desired_task_count: i32,
}
