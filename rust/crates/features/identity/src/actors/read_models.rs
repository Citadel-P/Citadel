use super::*;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActorDetails {
    pub id: Uuid,
    pub name: String,
    #[serde(rename = "type")]
    pub actor_type: ActorType,
    pub is_enabled: bool,
}
