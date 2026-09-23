use super::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcProviderDetails {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub display_name: String,
    pub issuer: String,
    pub client_id: String,
    pub scopes: String,
    pub enabled: bool,
    pub auto_provision_users: bool,
    pub allow_email_auto_link: bool,
    pub require_email_verified: bool,
    pub allowed_email_domains: Option<String>,
    pub required_claim_name: Option<String>,
    pub required_claim_values: Option<String>,
    pub default_role_id: Option<Uuid>,
    pub has_client_secret: bool,
    pub created_by_actor_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcProvidersDetails {
    pub providers: Vec<OidcProviderDetails>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcLoginProviderDetails {
    pub id: Uuid,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcLoginProvidersDetails {
    pub providers: Vec<OidcLoginProviderDetails>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcDiscoveryDetails {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidcLoginStart {
    pub authorization_url: String,
}

#[derive(Debug, Clone)]
pub struct OidcLoginComplete {
    pub session: SessionTokens,
    pub return_url: String,
}
