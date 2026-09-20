use super::*;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use citadel_activities::OidcProviderActivitySnapshot;
use citadel_primitives::ActorId;

pub const DEFAULT_OIDC_SCOPES: &str = "openid profile email";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidcProvider {
    id: Uuid,
    name: String,
    description: Option<String>,
    display_name: String,
    issuer: String,
    client_id: String,
    client_secret_ciphertext: Option<String>,
    scopes: String,
    enabled: bool,
    auto_provision_users: bool,
    allow_email_auto_link: bool,
    require_email_verified: bool,
    allowed_email_domains: Option<String>,
    required_claim_name: Option<String>,
    required_claim_values: Option<String>,
    default_role_id: Option<Uuid>,
    created_by_actor_id: ActorId,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl OidcProvider {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        name: String,
        description: Option<String>,
        display_name: String,
        issuer: String,
        client_id: String,
        client_secret_ciphertext: Option<String>,
        scopes: String,
        enabled: bool,
        auto_provision_users: bool,
        allow_email_auto_link: bool,
        require_email_verified: bool,
        allowed_email_domains: Option<String>,
        required_claim_name: Option<String>,
        required_claim_values: Option<String>,
        default_role_id: Option<Uuid>,
        created_by_actor_id: ActorId,
        now: DateTime<Utc>,
    ) -> Result<Self, OidcProviderValidationError> {
        Self::from_parts(
            Uuid::now_v7(),
            name,
            description,
            display_name,
            issuer,
            client_id,
            client_secret_ciphertext,
            scopes,
            enabled,
            auto_provision_users,
            allow_email_auto_link,
            require_email_verified,
            allowed_email_domains,
            required_claim_name,
            required_claim_values,
            default_role_id,
            created_by_actor_id,
            now,
            now,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_persistence(
        id: Uuid,
        name: String,
        description: Option<String>,
        display_name: String,
        issuer: String,
        client_id: String,
        client_secret_ciphertext: Option<String>,
        scopes: String,
        enabled: bool,
        auto_provision_users: bool,
        allow_email_auto_link: bool,
        require_email_verified: bool,
        allowed_email_domains: Option<String>,
        required_claim_name: Option<String>,
        required_claim_values: Option<String>,
        default_role_id: Option<Uuid>,
        created_by_actor_id: ActorId,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, OidcProviderValidationError> {
        Self::from_parts(
            id,
            name,
            description,
            display_name,
            issuer,
            client_id,
            client_secret_ciphertext,
            scopes,
            enabled,
            auto_provision_users,
            allow_email_auto_link,
            require_email_verified,
            allowed_email_domains,
            required_claim_name,
            required_claim_values,
            default_role_id,
            created_by_actor_id,
            created_at,
            updated_at,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn from_parts(
        id: Uuid,
        name: String,
        description: Option<String>,
        display_name: String,
        issuer: String,
        client_id: String,
        client_secret_ciphertext: Option<String>,
        scopes: String,
        enabled: bool,
        auto_provision_users: bool,
        allow_email_auto_link: bool,
        require_email_verified: bool,
        allowed_email_domains: Option<String>,
        required_claim_name: Option<String>,
        required_claim_values: Option<String>,
        default_role_id: Option<Uuid>,
        created_by_actor_id: ActorId,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, OidcProviderValidationError> {
        let provider = Self {
            id,
            name: name.trim().to_owned(),
            description: normalize_optional(description),
            display_name: display_name.trim().to_owned(),
            issuer: normalize_issuer(&issuer),
            client_id: client_id.trim().to_owned(),
            client_secret_ciphertext,
            scopes: normalize_scopes(&scopes),
            enabled,
            auto_provision_users,
            allow_email_auto_link,
            require_email_verified,
            allowed_email_domains: normalize_csv(allowed_email_domains),
            required_claim_name: normalize_optional(required_claim_name),
            required_claim_values: normalize_csv(required_claim_values),
            default_role_id: default_role_id.filter(|id| !id.is_nil()),
            created_by_actor_id,
            created_at,
            updated_at,
        };
        provider.validate()?;
        Ok(provider)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update(
        &mut self,
        name: String,
        description: Option<String>,
        display_name: String,
        issuer: String,
        client_id: String,
        client_secret_ciphertext: Option<String>,
        scopes: String,
        enabled: bool,
        auto_provision_users: bool,
        allow_email_auto_link: bool,
        require_email_verified: bool,
        allowed_email_domains: Option<String>,
        required_claim_name: Option<String>,
        required_claim_values: Option<String>,
        default_role_id: Option<Uuid>,
        updated_at: DateTime<Utc>,
    ) -> Result<(), OidcProviderValidationError> {
        let replacement = Self::from_parts(
            self.id,
            name,
            description,
            display_name,
            issuer,
            client_id,
            client_secret_ciphertext,
            scopes,
            enabled,
            auto_provision_users,
            allow_email_auto_link,
            require_email_verified,
            allowed_email_domains,
            required_claim_name,
            required_claim_values,
            default_role_id,
            self.created_by_actor_id,
            self.created_at,
            updated_at,
        )?;
        *self = replacement;
        Ok(())
    }

    fn validate(&self) -> Result<(), OidcProviderValidationError> {
        bounded_required(&self.name, 128, "OIDC provider name")?;
        bounded_optional(
            self.description.as_deref(),
            600,
            "OIDC provider description",
        )?;
        bounded_required(&self.display_name, 128, "OIDC provider display name")?;
        bounded_required(&self.client_id, 256, "OIDC provider client id")?;
        bounded_required(&self.scopes, 512, "OIDC provider scopes")?;
        bounded_optional(
            self.allowed_email_domains.as_deref(),
            1_024,
            "Allowed email domains",
        )?;
        bounded_optional(
            self.required_claim_name.as_deref(),
            256,
            "Required claim name",
        )?;
        bounded_optional(
            self.required_claim_values.as_deref(),
            1_024,
            "Required claim values",
        )?;
        let issuer = url::Url::parse(&self.issuer)
            .map_err(|_| OidcProviderValidationError::InvalidIssuer)?;
        if !matches!(issuer.scheme(), "http" | "https") || issuer.host_str().is_none() {
            return Err(OidcProviderValidationError::InvalidIssuer);
        }
        if !self
            .scopes
            .split_ascii_whitespace()
            .any(|scope| scope == "openid")
        {
            return Err(OidcProviderValidationError::MissingOpenIdScope);
        }
        if self
            .allowed_email_domains
            .as_deref()
            .is_some_and(|domains| {
                domains
                    .split(',')
                    .any(|domain| !domain.contains('.') || domain.contains('@'))
            })
        {
            return Err(OidcProviderValidationError::InvalidEmailDomain);
        }
        if self.required_claim_name.is_some() && self.required_claim_values.is_none() {
            return Err(OidcProviderValidationError::MissingRequiredClaimValues);
        }
        Ok(())
    }

    #[must_use]
    pub const fn id(&self) -> Uuid {
        self.id
    }
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }
    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }
    #[must_use]
    pub fn issuer(&self) -> &str {
        &self.issuer
    }
    #[must_use]
    pub fn client_id(&self) -> &str {
        &self.client_id
    }
    #[must_use]
    pub fn client_secret_ciphertext(&self) -> Option<&str> {
        self.client_secret_ciphertext.as_deref()
    }
    #[must_use]
    pub fn scopes(&self) -> &str {
        &self.scopes
    }
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.enabled
    }
    #[must_use]
    pub const fn auto_provision_users(&self) -> bool {
        self.auto_provision_users
    }
    #[must_use]
    pub const fn allow_email_auto_link(&self) -> bool {
        self.allow_email_auto_link
    }
    #[must_use]
    pub const fn require_email_verified(&self) -> bool {
        self.require_email_verified
    }
    #[must_use]
    pub fn allowed_email_domains(&self) -> Option<&str> {
        self.allowed_email_domains.as_deref()
    }
    #[must_use]
    pub fn required_claim_name(&self) -> Option<&str> {
        self.required_claim_name.as_deref()
    }
    #[must_use]
    pub fn required_claim_values(&self) -> Option<&str> {
        self.required_claim_values.as_deref()
    }
    #[must_use]
    pub const fn default_role_id(&self) -> Option<Uuid> {
        self.default_role_id
    }
    #[must_use]
    pub const fn created_by_actor_id(&self) -> ActorId {
        self.created_by_actor_id
    }
    #[must_use]
    pub const fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
    #[must_use]
    pub const fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    #[must_use]
    pub fn activity_snapshot(&self) -> OidcProviderActivitySnapshot {
        OidcProviderActivitySnapshot {
            id: self.id,
            name: self.name.clone(),
            description: self.description.clone(),
            display_name: self.display_name.clone(),
            issuer: self.issuer.clone(),
            client_id: self.client_id.clone(),
            scopes: self.scopes.clone(),
            enabled: self.enabled,
            auto_provision_users: self.auto_provision_users,
            allow_email_auto_link: self.allow_email_auto_link,
            require_email_verified: self.require_email_verified,
            allowed_email_domains: self.allowed_email_domains.clone(),
            required_claim_name: self.required_claim_name.clone(),
            required_claim_values: self.required_claim_values.clone(),
            default_role_id: self.default_role_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidcLoginState {
    pub id: Uuid,
    pub provider_id: Uuid,
    pub state_hash: String,
    pub nonce: String,
    pub code_verifier: String,
    pub return_url: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidcExternalLogin {
    pub id: Uuid,
    pub provider_id: Uuid,
    pub subject: String,
    pub user_id: Uuid,
    pub email: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OidcProviderValidationError {
    #[error("{0} is required.")]
    Required(&'static str),
    #[error("{0} exceeds {1} characters.")]
    TooLong(&'static str, usize),
    #[error("OIDC provider issuer must be an absolute HTTP or HTTPS URL.")]
    InvalidIssuer,
    #[error("OIDC provider scopes must include openid.")]
    MissingOpenIdScope,
    #[error("Allowed email domains must be plain domains such as example.com.")]
    InvalidEmailDomain,
    #[error("Required claim values are required when a required claim name is configured.")]
    MissingRequiredClaimValues,
}

fn bounded_required(
    value: &str,
    maximum: usize,
    name: &'static str,
) -> Result<(), OidcProviderValidationError> {
    if value.is_empty() {
        Err(OidcProviderValidationError::Required(name))
    } else if value.chars().count() > maximum {
        Err(OidcProviderValidationError::TooLong(name, maximum))
    } else {
        Ok(())
    }
}

fn bounded_optional(
    value: Option<&str>,
    maximum: usize,
    name: &'static str,
) -> Result<(), OidcProviderValidationError> {
    if value.is_some_and(|value| value.chars().count() > maximum) {
        Err(OidcProviderValidationError::TooLong(name, maximum))
    } else {
        Ok(())
    }
}

fn normalize_issuer(value: &str) -> String {
    value.trim().trim_end_matches('/').to_owned()
}

fn normalize_scopes(value: &str) -> String {
    value.split_ascii_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalize_optional(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn normalize_csv(value: Option<String>) -> Option<String> {
    let mut values = Vec::new();
    for item in value.as_deref().unwrap_or_default().split(',') {
        let item = item.trim();
        if !item.is_empty()
            && !values
                .iter()
                .any(|current: &&str| current.eq_ignore_ascii_case(item))
        {
            values.push(item);
        }
    }
    (!values.is_empty()).then(|| values.join(","))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SYSTEM_ACTOR_ID;

    #[test]
    fn provider_normalizes_the_existing_persistence_contract() {
        let provider = OidcProvider::new(
            " company ".to_owned(),
            Some("  ".to_owned()),
            " Company SSO ".to_owned(),
            "https://issuer.test/".to_owned(),
            " client ".to_owned(),
            None,
            "openid   profile email".to_owned(),
            true,
            false,
            true,
            true,
            Some("Example.com, example.com".to_owned()),
            None,
            None,
            None,
            ActorId::new(SYSTEM_ACTOR_ID),
            Utc::now(),
        )
        .unwrap();
        assert_eq!(provider.name(), "company");
        assert_eq!(provider.description(), None);
        assert_eq!(provider.issuer(), "https://issuer.test");
        assert_eq!(provider.scopes(), DEFAULT_OIDC_SCOPES);
        assert_eq!(provider.allowed_email_domains(), Some("Example.com"));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidcDiscovery {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_uri: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OidcIdentity {
    pub subject: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub name: Option<String>,
    pub claims: BTreeMap<String, Vec<String>>,
}
