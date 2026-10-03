use super::*;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchActorEnabledInput {
    pub is_enabled: bool,
}
