use super::requests::{PlatformConnectorType, PlatformType};
use citadel_primitives::PatchField;
use serde::{Deserialize, Serialize};

/// Omitted fields preserve their stored value. Only address and description allow null.
#[derive(Debug, Default, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformPatch {
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = String, required = false)]
    pub name: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    pub address: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = Option<String>, required = false)]
    pub description: PatchField<String>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[serde(rename = "type")]
    #[schema(value_type = PlatformType, required = false)]
    pub platform_type: PatchField<PlatformType>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = PlatformConnectorType, required = false)]
    pub connector_type: PatchField<PlatformConnectorType>,
    #[serde(default, skip_serializing_if = "PatchField::is_missing")]
    #[schema(value_type = bool, required = false)]
    pub prune_historical_swarm_task_containers: PatchField<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn platform_patch_preserves_omitted_null_and_false_values() {
        for input in [
            json!({}),
            json!({"description":null}),
            json!({"pruneHistoricalSwarmTaskContainers":false}),
            json!({"name":"renamed","address":null}),
        ] {
            let patch: PlatformPatch = serde_json::from_value(input.clone()).unwrap();
            assert_eq!(serde_json::to_value(patch).unwrap(), input);
        }
        assert!(
            serde_json::from_value::<PlatformPatch>(json!({"connectorType":"unknown-provider"}))
                .is_err()
        );
    }
}
