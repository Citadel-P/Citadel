use chrono::{DateTime, Utc};
use serde::Serialize;
use uuid::Uuid;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OidcProviderView {
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

impl From<OidcProviderView> for citadel_identity::OidcProviderDetails {
    fn from(value: OidcProviderView) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            display_name: value.display_name,
            issuer: value.issuer,
            client_id: value.client_id,
            scopes: value.scopes,
            enabled: value.enabled,
            auto_provision_users: value.auto_provision_users,
            allow_email_auto_link: value.allow_email_auto_link,
            require_email_verified: value.require_email_verified,
            allowed_email_domains: value.allowed_email_domains,
            required_claim_name: value.required_claim_name,
            required_claim_values: value.required_claim_values,
            default_role_id: value.default_role_id,
            has_client_secret: value.has_client_secret,
            created_by_actor_id: value.created_by_actor_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<citadel_identity::OidcProviderDetails> for OidcProviderView {
    fn from(value: citadel_identity::OidcProviderDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            display_name: value.display_name,
            issuer: value.issuer,
            client_id: value.client_id,
            scopes: value.scopes,
            enabled: value.enabled,
            auto_provision_users: value.auto_provision_users,
            allow_email_auto_link: value.allow_email_auto_link,
            require_email_verified: value.require_email_verified,
            allowed_email_domains: value.allowed_email_domains,
            required_claim_name: value.required_claim_name,
            required_claim_values: value.required_claim_values,
            default_role_id: value.default_role_id,
            has_client_secret: value.has_client_secret,
            created_by_actor_id: value.created_by_actor_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OidcProvidersView {
    pub providers: Vec<OidcProviderView>,
}

impl From<OidcProvidersView> for citadel_identity::OidcProvidersDetails {
    fn from(value: OidcProvidersView) -> Self {
        Self {
            providers: value
                .providers
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

impl From<citadel_identity::OidcProvidersDetails> for OidcProvidersView {
    fn from(value: citadel_identity::OidcProvidersDetails) -> Self {
        Self {
            providers: value
                .providers
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OidcLoginProviderView {
    pub id: Uuid,
    pub display_name: String,
}

impl From<OidcLoginProviderView> for citadel_identity::OidcLoginProviderDetails {
    fn from(value: OidcLoginProviderView) -> Self {
        Self {
            id: value.id,
            display_name: value.display_name,
        }
    }
}

impl From<citadel_identity::OidcLoginProviderDetails> for OidcLoginProviderView {
    fn from(value: citadel_identity::OidcLoginProviderDetails) -> Self {
        Self {
            id: value.id,
            display_name: value.display_name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct OidcLoginProvidersView {
    pub providers: Vec<OidcLoginProviderView>,
}

impl From<OidcLoginProvidersView> for citadel_identity::OidcLoginProvidersDetails {
    fn from(value: OidcLoginProvidersView) -> Self {
        Self {
            providers: value
                .providers
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

impl From<citadel_identity::OidcLoginProvidersDetails> for OidcLoginProvidersView {
    fn from(value: citadel_identity::OidcLoginProvidersDetails) -> Self {
        Self {
            providers: value
                .providers
                .into_iter()
                .map(|item| item.into())
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcDiscoveryView {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_uri: String,
}

impl From<OidcDiscoveryView> for citadel_identity::OidcDiscoveryDetails {
    fn from(value: OidcDiscoveryView) -> Self {
        Self {
            issuer: value.issuer,
            authorization_endpoint: value.authorization_endpoint,
            token_endpoint: value.token_endpoint,
            jwks_uri: value.jwks_uri,
        }
    }
}

impl From<citadel_identity::OidcDiscoveryDetails> for OidcDiscoveryView {
    fn from(value: citadel_identity::OidcDiscoveryDetails) -> Self {
        Self {
            issuer: value.issuer,
            authorization_endpoint: value.authorization_endpoint,
            token_endpoint: value.token_endpoint,
            jwks_uri: value.jwks_uri,
        }
    }
}
