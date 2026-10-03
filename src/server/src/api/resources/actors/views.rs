use citadel_identity::ActorType;
use serde::Serialize;
use uuid::Uuid;
#[derive(Debug, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActorView {
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "type")]
    #[schema(value_type = crate::api::resources::vocabulary::ActorTypeSchema)]
    pub actor_type: ActorType,
    pub is_enabled: bool,
}

impl From<ActorView> for citadel_identity::ActorDetails {
    fn from(value: ActorView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            actor_type: value.actor_type,
            is_enabled: value.is_enabled,
        }
    }
}

impl From<citadel_identity::ActorDetails> for ActorView {
    fn from(value: citadel_identity::ActorDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
            actor_type: value.actor_type,
            is_enabled: value.is_enabled,
        }
    }
}
