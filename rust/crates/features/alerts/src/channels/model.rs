use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AlertChannel {
    pub id: Uuid,
    pub name: String,
    pub alert_destination: String,
    pub url: String,
    pub is_active: bool,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
}
