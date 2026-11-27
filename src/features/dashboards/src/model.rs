use crate::DashboardConfiguration;
use citadel_primitives::ActorId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum DashboardVisibility {
    Private,
    InstanceShared,
    Restricted,
}

/// Audiences an administrator can select for a non-private dashboard.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum DashboardSharingMode {
    InstanceShared,
    Restricted,
}

impl From<DashboardSharingMode> for DashboardVisibility {
    fn from(mode: DashboardSharingMode) -> Self {
        match mode {
            DashboardSharingMode::InstanceShared => Self::InstanceShared,
            DashboardSharingMode::Restricted => Self::Restricted,
        }
    }
}

#[derive(Clone, Copy)]
pub struct DashboardViewer {
    pub actor: ActorId,
    pub administrator: bool,
    pub read_only: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSummary {
    pub id: Uuid,
    pub name: String,
    pub revision: i64,
    pub visibility: DashboardVisibility,
    pub can_edit: bool,
    pub can_delete: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedDashboard {
    pub summary: DashboardSummary,
    // Keep unsupported stored schema versions intact for read-only clients.
    pub configuration: serde_json::Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DashboardContent {
    pub name: String,
    pub configuration: DashboardConfiguration,
}

#[derive(Debug, thiserror::Error)]
pub enum DashboardError {
    #[error("{0}")]
    Invalid(String),
    #[error("Dashboard is unavailable.")]
    Unavailable,
    #[error("You do not have permission to change this dashboard.")]
    Forbidden,
    #[error("Restricted dashboards require custom access control.")]
    CustomAccessRequired,
    #[error("This dashboard changed. Your edits are still available.")]
    Conflict,
    #[error("Dashboard storage failed: {0}")]
    Storage(String),
}
