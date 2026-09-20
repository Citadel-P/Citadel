use super::*;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateOidcProvider {
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

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchOidcProvider {
    #[serde(default)]
    pub name: PatchField<String>,
    #[serde(default)]
    pub description: PatchField<String>,
    #[serde(default)]
    pub display_name: PatchField<String>,
    #[serde(default)]
    pub issuer: PatchField<String>,
    #[serde(default)]
    pub client_id: PatchField<String>,
    #[serde(default)]
    pub client_secret: PatchField<String>,
    #[serde(default)]
    pub scopes: PatchField<String>,
    #[serde(default)]
    pub enabled: PatchField<bool>,
    #[serde(default)]
    pub auto_provision_users: PatchField<bool>,
    #[serde(default)]
    pub allow_email_auto_link: PatchField<bool>,
    #[serde(default)]
    pub require_email_verified: PatchField<bool>,
    #[serde(default)]
    pub allowed_email_domains: PatchField<String>,
    #[serde(default)]
    pub required_claim_name: PatchField<String>,
    #[serde(default)]
    pub required_claim_values: PatchField<String>,
    #[serde(default)]
    pub default_role_id: PatchField<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenameOidcProvider {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchOidcProviderMetadata {
    #[serde(default)]
    pub description: PatchField<String>,
    #[serde(default)]
    pub tags: PatchField<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TestOidcDiscovery {
    pub provider_id: Option<Uuid>,
    pub issuer: Option<String>,
}
