use citadel_identity::PatchField;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BeginLoginQuery {
    pub(crate) return_url: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct CallbackQuery {
    pub(crate) code: Option<String>,
    pub(crate) state: Option<String>,
    pub(crate) error: Option<String>,
    pub(crate) error_description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateOidcProviderRequest {
    pub name: String,
    pub description: Option<String>,
    pub display_name: String,
    pub issuer: String,
    pub client_id: String,
    pub client_secret: Option<String>,
    pub scopes: Option<String>,
    pub enabled: bool,
    pub auto_provision_users: bool,
    pub allow_email_auto_link: bool,
    pub require_email_verified: bool,
    pub allowed_email_domains: Option<String>,
    pub required_claim_name: Option<String>,
    pub required_claim_values: Option<String>,
    pub default_role_id: Option<Uuid>,
}

impl From<CreateOidcProviderRequest> for citadel_identity::CreateOidcProvider {
    fn from(value: CreateOidcProviderRequest) -> Self {
        Self {
            name: value.name,
            description: value.description,
            display_name: value.display_name,
            issuer: value.issuer,
            client_id: value.client_id,
            client_secret: value.client_secret,
            scopes: value.scopes,
            enabled: value.enabled,
            auto_provision_users: value.auto_provision_users,
            allow_email_auto_link: value.allow_email_auto_link,
            require_email_verified: value.require_email_verified,
            allowed_email_domains: value.allowed_email_domains,
            required_claim_name: value.required_claim_name,
            required_claim_values: value.required_claim_values,
            default_role_id: value.default_role_id,
        }
    }
}

impl From<citadel_identity::CreateOidcProvider> for CreateOidcProviderRequest {
    fn from(value: citadel_identity::CreateOidcProvider) -> Self {
        Self {
            name: value.name,
            description: value.description,
            display_name: value.display_name,
            issuer: value.issuer,
            client_id: value.client_id,
            client_secret: value.client_secret,
            scopes: value.scopes,
            enabled: value.enabled,
            auto_provision_users: value.auto_provision_users,
            allow_email_auto_link: value.allow_email_auto_link,
            require_email_verified: value.require_email_verified,
            allowed_email_domains: value.allowed_email_domains,
            required_claim_name: value.required_claim_name,
            required_claim_values: value.required_claim_values,
            default_role_id: value.default_role_id,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchOidcProviderRequest {
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub name: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub description: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub display_name: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub issuer: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub client_id: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub client_secret: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub scopes: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<bool>, required = false)]
    pub enabled: PatchField<bool>,
    #[serde(default)]
    #[schema(value_type = Option<bool>, required = false)]
    pub auto_provision_users: PatchField<bool>,
    #[serde(default)]
    #[schema(value_type = Option<bool>, required = false)]
    pub allow_email_auto_link: PatchField<bool>,
    #[serde(default)]
    #[schema(value_type = Option<bool>, required = false)]
    pub require_email_verified: PatchField<bool>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub allowed_email_domains: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub required_claim_name: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub required_claim_values: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<Uuid>, required = false)]
    pub default_role_id: PatchField<Uuid>,
}

impl From<PatchOidcProviderRequest> for citadel_identity::PatchOidcProvider {
    fn from(value: PatchOidcProviderRequest) -> Self {
        Self {
            name: value.name,
            description: value.description,
            display_name: value.display_name,
            issuer: value.issuer,
            client_id: value.client_id,
            client_secret: value.client_secret,
            scopes: value.scopes,
            enabled: value.enabled,
            auto_provision_users: value.auto_provision_users,
            allow_email_auto_link: value.allow_email_auto_link,
            require_email_verified: value.require_email_verified,
            allowed_email_domains: value.allowed_email_domains,
            required_claim_name: value.required_claim_name,
            required_claim_values: value.required_claim_values,
            default_role_id: value.default_role_id,
        }
    }
}

impl From<citadel_identity::PatchOidcProvider> for PatchOidcProviderRequest {
    fn from(value: citadel_identity::PatchOidcProvider) -> Self {
        Self {
            name: value.name,
            description: value.description,
            display_name: value.display_name,
            issuer: value.issuer,
            client_id: value.client_id,
            client_secret: value.client_secret,
            scopes: value.scopes,
            enabled: value.enabled,
            auto_provision_users: value.auto_provision_users,
            allow_email_auto_link: value.allow_email_auto_link,
            require_email_verified: value.require_email_verified,
            allowed_email_domains: value.allowed_email_domains,
            required_claim_name: value.required_claim_name,
            required_claim_values: value.required_claim_values,
            default_role_id: value.default_role_id,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameOidcProviderRequest {
    pub id: Uuid,
    pub name: String,
}

impl From<RenameOidcProviderRequest> for citadel_identity::RenameOidcProvider {
    fn from(value: RenameOidcProviderRequest) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<citadel_identity::RenameOidcProvider> for RenameOidcProviderRequest {
    fn from(value: citadel_identity::RenameOidcProvider) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchOidcProviderMetadataRequest {
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub description: PatchField<String>,
    #[serde(default)]
    #[schema(value_type = Option<Vec<String>>, required = false)]
    pub tags: PatchField<Vec<String>>,
}

impl From<PatchOidcProviderMetadataRequest> for citadel_identity::PatchOidcProviderMetadata {
    fn from(value: PatchOidcProviderMetadataRequest) -> Self {
        Self {
            description: value.description,
            tags: value.tags,
        }
    }
}

impl From<citadel_identity::PatchOidcProviderMetadata> for PatchOidcProviderMetadataRequest {
    fn from(value: citadel_identity::PatchOidcProviderMetadata) -> Self {
        Self {
            description: value.description,
            tags: value.tags,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TestOidcDiscoveryRequest {
    pub provider_id: Option<Uuid>,
    pub issuer: Option<String>,
}

impl From<TestOidcDiscoveryRequest> for citadel_identity::TestOidcDiscovery {
    fn from(value: TestOidcDiscoveryRequest) -> Self {
        Self {
            provider_id: value.provider_id,
            issuer: value.issuer,
        }
    }
}

impl From<citadel_identity::TestOidcDiscovery> for TestOidcDiscoveryRequest {
    fn from(value: citadel_identity::TestOidcDiscovery) -> Self {
        Self {
            provider_id: value.provider_id,
            issuer: value.issuer,
        }
    }
}
