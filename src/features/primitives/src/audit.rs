use crate::ActorId;
use chrono::{DateTime, Utc};
/// Creation attribution for Citadel-managed entities; unrelated to operation ownership.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuditMetadata {
    pub created_at: DateTime<Utc>,
    pub created_by_actor_id: ActorId,
}
