use super::*;
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct IdentityResourceAccessSnapshot {
    #[serde(rename = "ResourceType")]
    pub resource_type: ResourceType,
    #[serde(rename = "ResourceId")]
    pub resource_id: Uuid,
    #[serde(rename = "PermissionLevel")]
    pub permission_level: PermissionLevel,
    #[serde(rename = "SpecificPermissions")]
    pub specific_permissions: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UserActivitySnapshot {
    #[serde(rename = "Email")]
    pub email: String,
    #[serde(rename = "IsEnabled")]
    pub is_enabled: bool,
    #[serde(rename = "TeamIds")]
    pub team_ids: Vec<Uuid>,
    #[serde(rename = "RoleIds")]
    pub role_ids: Vec<Uuid>,
    #[serde(rename = "ResourceAccesses")]
    pub resource_accesses: Vec<IdentityResourceAccessSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TeamActivitySnapshot {
    #[serde(rename = "IsEnabled")]
    pub is_enabled: bool,
    #[serde(rename = "MemberActorIds")]
    pub member_actor_ids: Vec<Uuid>,
    #[serde(rename = "RoleIds")]
    pub role_ids: Vec<Uuid>,
    #[serde(rename = "ResourceAccesses")]
    pub resource_accesses: Vec<IdentityResourceAccessSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RolePermissionActivitySnapshot {
    #[serde(rename = "ResourceType")]
    pub resource_type: ResourceType,
    #[serde(rename = "PermissionLevel")]
    pub permission_level: PermissionLevel,
    #[serde(rename = "SpecificPermissions")]
    pub specific_permissions: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RoleActivitySnapshot {
    #[serde(rename = "RoleType")]
    pub role_type: String,
    #[serde(rename = "Permissions")]
    pub permissions: Vec<RolePermissionActivitySnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServiceAccountResourceAccessSnapshot {
    #[serde(rename = "ResourceType")]
    pub resource_type: ResourceType,
    #[serde(rename = "ResourceId")]
    pub resource_id: Uuid,
    #[serde(rename = "PermissionLevel")]
    pub permission_level: PermissionLevel,
    #[serde(rename = "SpecificPermissions")]
    pub specific_permissions: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ServiceAccountActivitySnapshot {
    #[serde(rename = "Id")]
    pub id: Uuid,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "IsEnabled")]
    pub is_enabled: bool,
    #[serde(rename = "TeamIds")]
    pub team_ids: Vec<Uuid>,
    #[serde(rename = "RoleIds")]
    pub role_ids: Vec<Uuid>,
    #[serde(rename = "ResourceAccesses")]
    pub resource_accesses: Vec<ServiceAccountResourceAccessSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OidcProviderActivitySnapshot {
    #[serde(rename = "Id")]
    pub id: Uuid,
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "DisplayName")]
    pub display_name: String,
    #[serde(rename = "Issuer")]
    pub issuer: String,
    #[serde(rename = "ClientId")]
    pub client_id: String,
    #[serde(rename = "Scopes")]
    pub scopes: String,
    #[serde(rename = "Enabled")]
    pub enabled: bool,
    #[serde(rename = "AutoProvisionUsers")]
    pub auto_provision_users: bool,
    #[serde(rename = "AllowEmailAutoLink")]
    pub allow_email_auto_link: bool,
    #[serde(rename = "RequireEmailVerified")]
    pub require_email_verified: bool,
    #[serde(rename = "AllowedEmailDomains")]
    pub allowed_email_domains: Option<String>,
    #[serde(rename = "RequiredClaimName")]
    pub required_claim_name: Option<String>,
    #[serde(rename = "RequiredClaimValues")]
    pub required_claim_values: Option<String>,
    #[serde(rename = "DefaultRoleId")]
    pub default_role_id: Option<Uuid>,
}
