use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AlertChannel {
    pub id: Uuid,
    pub name: String,
    pub alert_destination: String,
    pub url: String,
    pub is_active: bool,
    pub audit: citadel_primitives::AuditMetadata,
}
