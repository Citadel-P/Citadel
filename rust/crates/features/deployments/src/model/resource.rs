use super::*;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagSummary {
    pub id: Uuid,
    pub name: String,
    pub color: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AutoUpdateState {
    pub last_checked_at: DateTime<Utc>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

/// Durable Deployment state, independent of HTTP and authorization presentation.
#[derive(Debug, Clone, PartialEq)]
pub struct Deployment {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub platform_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub created_by_actor_id: Uuid,
    pub status: String,
    pub control_state: String,
    pub row_version: i64,
    pub auto_update_state: Option<AutoUpdateState>,
    pub spec: DeploymentSpec,
}
#[derive(Debug, Clone, thiserror::Error)]
pub enum DeploymentError {
    #[error("{0}")]
    Validation(String),
    #[error("Deployment was not found.")]
    NotFound,
    #[error("This operation is not authorized.")]
    Forbidden,
    #[error("License capability '{0}' is unavailable.")]
    LicenseRequired(&'static str),
    #[error("{0}")]
    Conflict(String),
    #[error("Deployment runtime is unavailable: {0}")]
    Runtime(String),
    #[error("Deployment persistence failed: {0}")]
    Storage(String),
    #[error("Deployment operation was cancelled.")]
    Cancelled,
}
