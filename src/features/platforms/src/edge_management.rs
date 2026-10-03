use chrono::{DateTime, Utc};
use futures_util::future::BoxFuture;
use uuid::Uuid;
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EdgeTarget {
    pub resource_type: i32,
    pub resource_id: Uuid,
    pub platform_id: Uuid,
    pub node_id: Option<String>,
}
impl EdgeTarget {
    pub fn platform(id: Uuid) -> Self {
        Self {
            resource_type: 0,
            resource_id: id,
            platform_id: id,
            node_id: None,
        }
    }
    pub fn node(platform: Uuid, node_id: String) -> Self {
        Self {
            node_id: Some(node_id),
            ..Self::platform(platform)
        }
    }
    pub fn build_pool(id: Uuid) -> Self {
        Self {
            resource_type: 1,
            resource_id: id,
            platform_id: Uuid::nil(),
            node_id: None,
        }
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EdgeStatus {
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

#[derive(Debug, thiserror::Error)]
pub enum EdgeManagementError {
    #[error("Edge Agent resource was not found.")]
    NotFound,
    #[error("Edge Agent identity or enrollment is invalid, expired, revoked, or already in use.")]
    Unauthorized,
    #[error("{0}")]
    Invalid(&'static str),
    #[error("Edge Agent persistence failed: {0}")]
    Storage(String),
}
pub type EdgeEnrollment = (Uuid, String, DateTime<Utc>);

pub trait EdgeEnrollmentStore: Send + Sync {
    fn status<'a>(
        &'a self,
        target: &'a EdgeTarget,
    ) -> BoxFuture<'a, Result<EdgeStatus, EdgeManagementError>>;
    fn create_enrollment<'a>(
        &'a self,
        target: &'a EdgeTarget,
        actor: Uuid,
    ) -> BoxFuture<'a, Result<EdgeEnrollment, EdgeManagementError>>;
    fn revoke<'a>(
        &'a self,
        target: &'a EdgeTarget,
    ) -> BoxFuture<'a, Result<(), EdgeManagementError>>;
}
pub trait EdgeSessionControl: Send + Sync {
    fn disconnect(&self, target: &EdgeTarget);
    fn disconnect_platform(&self, id: Uuid);
}
pub async fn revoke(
    store: &dyn EdgeEnrollmentStore,
    sessions: &dyn EdgeSessionControl,
    target: &EdgeTarget,
) -> Result<(), EdgeManagementError> {
    store.status(target).await?;
    store.revoke(target).await?;
    sessions.disconnect(target);
    Ok(())
}
