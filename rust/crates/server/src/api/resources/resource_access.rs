use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceAccessInput {
    #[schema(value_type = crate::api::resources::vocabulary::ResourceTypeSchema)]
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    #[schema(value_type = crate::api::resources::vocabulary::PermissionLevelSchema)]
    pub permission_level: PermissionLevel,
    #[serde(default)]
    #[schema(value_type = Vec<crate::api::resources::vocabulary::SpecificPermissionSchema>)]
    pub specific_permissions: Vec<SpecificPermission>,
}

impl From<ResourceAccessInput> for citadel_identity::ResourceAccessInput {
    fn from(value: ResourceAccessInput) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

impl From<citadel_identity::ResourceAccessInput> for ResourceAccessInput {
    fn from(value: citadel_identity::ResourceAccessInput) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
        }
    }
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceAccessView {
    #[schema(value_type = crate::api::resources::vocabulary::ResourceTypeSchema)]
    pub resource_type: ResourceType,
    pub resource_id: Uuid,
    #[schema(required = true)]
    pub resource_name: Option<String>,
    #[schema(value_type = crate::api::resources::vocabulary::PermissionLevelSchema)]
    pub permission_level: PermissionLevel,
    #[schema(value_type = Option<Vec<crate::api::resources::vocabulary::SpecificPermissionSchema>>, required = true)]
    pub specific_permissions: Option<Vec<SpecificPermission>>,
    #[schema(required = true)]
    pub id: Option<Uuid>,
}

impl From<ResourceAccessView> for citadel_identity::ResourceAccessDetails {
    fn from(value: ResourceAccessView) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            resource_name: value.resource_name,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
            id: value.id,
        }
    }
}

impl From<citadel_identity::ResourceAccessDetails> for ResourceAccessView {
    fn from(value: citadel_identity::ResourceAccessDetails) -> Self {
        Self {
            resource_type: value.resource_type,
            resource_id: value.resource_id,
            resource_name: value.resource_name,
            permission_level: value.permission_level,
            specific_permissions: value.specific_permissions,
            id: value.id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn user_and_team_routes_share_access_contracts() {
        let doc = serde_json::to_value(crate::openapi::document(false)).unwrap();
        let schemas = &doc["components"]["schemas"];
        for (resource, owner) in [("users", "User"), ("teams", "Team")] {
            let path = format!("/api/v1/{resource}/{{id}}/resource-accesses");
            for method in ["post", "delete"] {
                assert_eq!(
                    doc["paths"][&path][method]["requestBody"]["content"]["application/json"]["schema"]
                        ["$ref"],
                    "#/components/schemas/ResourceAccessInput"
                );
            }
            for request in [
                format!("Create{owner}Request"),
                format!("Patch{owner}Request"),
            ] {
                assert!(
                    schemas[&request]["properties"]["resourceAccesses"]
                        .to_string()
                        .contains("#/components/schemas/ResourceAccessInput")
                );
            }
            assert!(
                schemas[format!("{owner}View")]["properties"]["resourceAccesses"]
                    .to_string()
                    .contains("#/components/schemas/ResourceAccessView")
            );
            assert!(schemas.get(format!("{owner}ResourceAccessInput")).is_none());
            assert!(schemas.get(format!("{owner}ResourceAccessView")).is_none());
        }
        assert!(schemas.get("UserResourceAccessRequest").is_none());
        // Nullable fields remain present in the response contract.
        for field in ["id", "resourceName", "specificPermissions"] {
            assert!(
                schemas["ResourceAccessView"]["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!(field))
            );
        }
    }

    #[test]
    fn access_input_defaults_specifics_and_preserves_permission_vocabulary() {
        let resource_id = Uuid::now_v7();
        let input: ResourceAccessInput = serde_json::from_value(json!({
            "resourceType": "Deployment",
            "resourceId": resource_id,
            "permissionLevel": "Read"
        }))
        .unwrap();
        let domain: citadel_identity::ResourceAccessInput = input.into();
        assert_eq!(domain.resource_type, ResourceType::Deployment);
        assert_eq!(domain.permission_level, PermissionLevel::Read);
        assert!(domain.specific_permissions.is_empty());
        assert_eq!(
            serde_json::to_value(ResourceAccessInput::from(domain)).unwrap(),
            json!({
                "resourceType": "Deployment", "resourceId": resource_id,
                "permissionLevel": "Read", "specificPermissions": []
            })
        );
    }
}
