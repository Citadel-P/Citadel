use serde::Deserialize;
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PatchActorEnabledInput {
    pub is_enabled: bool,
}

impl From<PatchActorEnabledInput> for citadel_identity::PatchActorEnabledInput {
    fn from(value: PatchActorEnabledInput) -> Self {
        Self {
            is_enabled: value.is_enabled,
        }
    }
}

impl From<citadel_identity::PatchActorEnabledInput> for PatchActorEnabledInput {
    fn from(value: citadel_identity::PatchActorEnabledInput) -> Self {
        Self {
            is_enabled: value.is_enabled,
        }
    }
}
