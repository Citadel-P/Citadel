use std::collections::BTreeMap;
use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Duration, Utc};
use citadel_domain::ActorId;
use futures_util::future::BoxFuture;
use getrandom::fill;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use url::Url;
use uuid::Uuid;

use crate::{
    Clock, DEFAULT_OIDC_SCOPES, IdentityError, IdentityService, OidcLoginState, OidcProvider,
    PatchField, SecretProtector, SessionMetadata, SessionTokens, UserAuthentication,
};

const MAXIMUM_CLIENT_SECRET_CHARS: usize = 4_096;
const DEFAULT_RETURN_URL: &str = "/";

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

pub trait OidcProtocol: Send + Sync {
    fn discover<'a>(
        &'a self,
        issuer: &'a str,
    ) -> BoxFuture<'a, Result<OidcDiscovery, IdentityError>>;

    #[allow(clippy::too_many_arguments)]
    fn exchange_and_validate<'a>(
        &'a self,
        provider: &'a OidcProvider,
        discovery: &'a OidcDiscovery,
        client_secret: &'a str,
        code: &'a str,
        redirect_uri: &'a str,
        code_verifier: &'a str,
        nonce: &'a str,
    ) -> BoxFuture<'a, Result<OidcIdentity, IdentityError>>;
}

pub trait OidcStore: Send + Sync {
    fn list(&self) -> BoxFuture<'_, Result<Vec<OidcProvider>, IdentityError>>;
    fn list_enabled(&self) -> BoxFuture<'_, Result<Vec<OidcProvider>, IdentityError>>;
    fn get(&self, id: Uuid) -> BoxFuture<'_, Result<Option<OidcProvider>, IdentityError>>;
    fn role_exists(&self, id: Uuid) -> BoxFuture<'_, Result<bool, IdentityError>>;
    fn create<'a>(
        &'a self,
        provider: &'a OidcProvider,
        actor_id: ActorId,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<OidcProvider, IdentityError>>;
    fn update<'a>(
        &'a self,
        provider: &'a OidcProvider,
        expected_updated_at: DateTime<Utc>,
        actor_id: ActorId,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<OidcProvider, IdentityError>>;
    fn rename<'a>(
        &'a self,
        id: Uuid,
        name: &'a str,
        actor_id: ActorId,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<OidcProvider, IdentityError>>;
    fn update_description<'a>(
        &'a self,
        id: Uuid,
        description: Option<&'a str>,
        actor_id: ActorId,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<OidcProvider, IdentityError>>;
    fn delete(
        &self,
        id: Uuid,
        actor_id: ActorId,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), IdentityError>>;
    fn start_login(
        &self,
        state: OidcLoginState,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), IdentityError>>;
    fn consume_login_state<'a>(
        &'a self,
        state_hash: &'a str,
    ) -> BoxFuture<'a, Result<Option<OidcLoginState>, IdentityError>>;
    fn resolve_identity<'a>(
        &'a self,
        provider: &'a OidcProvider,
        identity: &'a OidcIdentity,
        proposed_name: &'a str,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<UserAuthentication, IdentityError>>;
}

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchOidcProviderRequest {
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
pub struct RenameOidcProviderRequest {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PatchOidcProviderMetadataRequest {
    #[serde(default)]
    pub description: PatchField<String>,
    #[serde(default)]
    pub tags: PatchField<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TestOidcDiscoveryRequest {
    pub provider_id: Option<Uuid>,
    pub issuer: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

impl From<&OidcProvider> for OidcProviderView {
    fn from(provider: &OidcProvider) -> Self {
        Self {
            id: provider.id(),
            name: provider.name().to_owned(),
            description: provider.description().map(str::to_owned),
            display_name: provider.display_name().to_owned(),
            issuer: provider.issuer().to_owned(),
            client_id: provider.client_id().to_owned(),
            scopes: provider.scopes().to_owned(),
            enabled: provider.enabled(),
            auto_provision_users: provider.auto_provision_users(),
            allow_email_auto_link: provider.allow_email_auto_link(),
            require_email_verified: provider.require_email_verified(),
            allowed_email_domains: provider.allowed_email_domains().map(str::to_owned),
            required_claim_name: provider.required_claim_name().map(str::to_owned),
            required_claim_values: provider.required_claim_values().map(str::to_owned),
            default_role_id: provider.default_role_id(),
            has_client_secret: provider.client_secret_ciphertext().is_some(),
            created_by_actor_id: provider.created_by_actor_id().value(),
            created_at: provider.created_at(),
            updated_at: provider.updated_at(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcProvidersView {
    pub providers: Vec<OidcProviderView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcLoginProviderView {
    pub id: Uuid,
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcLoginProvidersView {
    pub providers: Vec<OidcLoginProviderView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OidcDiscoveryView {
    pub issuer: String,
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub jwks_uri: String,
}

impl From<OidcDiscovery> for OidcDiscoveryView {
    fn from(discovery: OidcDiscovery) -> Self {
        Self {
            issuer: discovery.issuer,
            authorization_endpoint: discovery.authorization_endpoint,
            token_endpoint: discovery.token_endpoint,
            jwks_uri: discovery.jwks_uri,
        }
    }
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

#[derive(Clone)]
pub struct OidcService {
    store: Arc<dyn OidcStore>,
    protocol: Arc<dyn OidcProtocol>,
    protector: Arc<dyn SecretProtector>,
    identity: Arc<IdentityService>,
    clock: Arc<dyn Clock>,
    login_state_lifetime: Duration,
}

impl OidcService {
    #[must_use]
    pub fn new(
        store: Arc<dyn OidcStore>,
        protocol: Arc<dyn OidcProtocol>,
        protector: Arc<dyn SecretProtector>,
        identity: Arc<IdentityService>,
        clock: Arc<dyn Clock>,
        login_state_lifetime: Duration,
    ) -> Self {
        Self {
            store,
            protocol,
            protector,
            identity,
            clock,
            login_state_lifetime,
        }
    }

    pub async fn list(&self) -> Result<OidcProvidersView, IdentityError> {
        Ok(OidcProvidersView {
            providers: self.store.list().await?.iter().map(Into::into).collect(),
        })
    }

    pub async fn list_login_providers(&self) -> Result<OidcLoginProvidersView, IdentityError> {
        Ok(OidcLoginProvidersView {
            providers: self
                .store
                .list_enabled()
                .await?
                .into_iter()
                .map(|provider| OidcLoginProviderView {
                    id: provider.id(),
                    display_name: provider.display_name().to_owned(),
                })
                .collect(),
        })
    }

    pub async fn get(&self, id: Uuid) -> Result<OidcProviderView, IdentityError> {
        validate_id(id)?;
        self.store
            .get(id)
            .await?
            .as_ref()
            .map(Into::into)
            .ok_or(IdentityError::NotFound)
    }

    pub async fn create(
        &self,
        request: CreateOidcProviderRequest,
        actor_id: ActorId,
    ) -> Result<OidcProviderView, IdentityError> {
        validate_secret(request.client_secret.as_deref())?;
        validate_default_role(self.store.as_ref(), request.default_role_id).await?;
        let protected_secret = request
            .client_secret
            .as_deref()
            .filter(|secret| !secret.trim().is_empty())
            .map(|secret| self.protector.protect(secret.as_bytes()))
            .transpose()?;
        let now = self.clock.now();
        let provider = OidcProvider::new(
            request.name,
            request.description,
            request.display_name,
            request.issuer,
            request.client_id,
            protected_secret,
            request
                .scopes
                .unwrap_or_else(|| DEFAULT_OIDC_SCOPES.to_owned()),
            request.enabled,
            request.auto_provision_users,
            request.allow_email_auto_link,
            request.require_email_verified,
            request.allowed_email_domains,
            request.required_claim_name,
            request.required_claim_values,
            request.default_role_id,
            actor_id,
            now,
        )
        .map_err(validation)?;
        Ok((&self.store.create(&provider, actor_id, now).await?).into())
    }

    pub async fn patch(
        &self,
        id: Uuid,
        request: PatchOidcProviderRequest,
        actor_id: ActorId,
    ) -> Result<OidcProviderView, IdentityError> {
        validate_id(id)?;
        let mut provider = self.store.get(id).await?.ok_or(IdentityError::NotFound)?;
        let expected_updated_at = provider.updated_at();
        let name = required_patch(request.name, provider.name(), "Name")?;
        let description = optional_patch(request.description, provider.description());
        let display_name = required_patch(
            request.display_name,
            provider.display_name(),
            "Display name",
        )?;
        let issuer = required_patch(request.issuer, provider.issuer(), "Issuer")?;
        let client_id = required_patch(request.client_id, provider.client_id(), "Client ID")?;
        let scopes = required_patch(request.scopes, provider.scopes(), "Scopes")?;
        let enabled = value_patch(request.enabled, provider.enabled(), "Enabled")?;
        let auto_provision_users = value_patch(
            request.auto_provision_users,
            provider.auto_provision_users(),
            "Auto provision users",
        )?;
        let allow_email_auto_link = value_patch(
            request.allow_email_auto_link,
            provider.allow_email_auto_link(),
            "Allow email auto link",
        )?;
        let require_email_verified = value_patch(
            request.require_email_verified,
            provider.require_email_verified(),
            "Require email verified",
        )?;
        let allowed_email_domains = optional_patch(
            request.allowed_email_domains,
            provider.allowed_email_domains(),
        );
        let required_claim_name =
            optional_patch(request.required_claim_name, provider.required_claim_name());
        let required_claim_values = optional_patch(
            request.required_claim_values,
            provider.required_claim_values(),
        );
        let default_role_id = match request.default_role_id {
            PatchField::Missing => provider.default_role_id(),
            PatchField::Null => None,
            PatchField::Value(id) if id.is_nil() => None,
            PatchField::Value(id) => Some(id),
        };
        validate_default_role(self.store.as_ref(), default_role_id).await?;
        let protected_secret = match request.client_secret {
            PatchField::Missing | PatchField::Null => {
                provider.client_secret_ciphertext().map(str::to_owned)
            }
            PatchField::Value(secret) if secret.trim().is_empty() => {
                provider.client_secret_ciphertext().map(str::to_owned)
            }
            PatchField::Value(secret) => {
                validate_secret(Some(&secret))?;
                Some(self.protector.protect(secret.as_bytes())?)
            }
        };
        let now = self.clock.now();
        provider
            .update(
                name,
                description,
                display_name,
                issuer,
                client_id,
                protected_secret,
                scopes,
                enabled,
                auto_provision_users,
                allow_email_auto_link,
                require_email_verified,
                allowed_email_domains,
                required_claim_name,
                required_claim_values,
                default_role_id,
                now,
            )
            .map_err(validation)?;
        Ok((&self
            .store
            .update(&provider, expected_updated_at, actor_id, now)
            .await?)
            .into())
    }

    pub async fn rename(
        &self,
        id: Uuid,
        name: &str,
        actor_id: ActorId,
    ) -> Result<OidcProviderView, IdentityError> {
        validate_id(id)?;
        if name.trim().is_empty() || name.chars().count() > 128 {
            return Err(IdentityError::Validation(
                "Name must contain between 1 and 128 characters.".to_owned(),
            ));
        }
        Ok((&self
            .store
            .rename(id, name.trim(), actor_id, self.clock.now())
            .await?)
            .into())
    }

    pub async fn update_metadata(
        &self,
        id: Uuid,
        request: PatchOidcProviderMetadataRequest,
        actor_id: ActorId,
    ) -> Result<OidcProviderView, IdentityError> {
        validate_id(id)?;
        let description = match request.description {
            PatchField::Missing => return self.get(id).await,
            PatchField::Null => None,
            PatchField::Value(value) => Some(value),
        };
        if description
            .as_ref()
            .is_some_and(|value| value.chars().count() > 600)
        {
            return Err(IdentityError::Validation(
                "Description cannot exceed 600 characters.".to_owned(),
            ));
        }
        Ok((&self
            .store
            .update_description(id, description.as_deref(), actor_id, self.clock.now())
            .await?)
            .into())
    }

    pub async fn delete(&self, id: Uuid, actor_id: ActorId) -> Result<(), IdentityError> {
        validate_id(id)?;
        self.store.delete(id, actor_id, self.clock.now()).await
    }

    pub async fn test_discovery(
        &self,
        request: TestOidcDiscoveryRequest,
    ) -> Result<OidcDiscoveryView, IdentityError> {
        let issuer = match request
            .issuer
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(issuer) => issuer.to_owned(),
            None => {
                let provider_id = request
                    .provider_id
                    .ok_or_else(|| IdentityError::Validation("Issuer is required.".to_owned()))?;
                self.store
                    .get(provider_id)
                    .await?
                    .ok_or(IdentityError::NotFound)?
                    .issuer()
                    .to_owned()
            }
        };
        Ok(self.protocol.discover(&issuer).await?.into())
    }

    pub async fn begin_login(
        &self,
        provider_id: Uuid,
        redirect_uri: &str,
        return_url: Option<&str>,
        allowed_return_origins: &[String],
    ) -> Result<OidcLoginStart, IdentityError> {
        validate_id(provider_id)?;
        validate_absolute_http_url(redirect_uri, "Redirect URI is invalid.")?;
        let provider = self
            .store
            .get(provider_id)
            .await?
            .filter(OidcProvider::enabled)
            .ok_or(IdentityError::NotFound)?;
        let return_url = normalize_return_url(
            return_url.unwrap_or(DEFAULT_RETURN_URL),
            redirect_uri,
            allowed_return_origins,
        )
        .ok_or_else(|| IdentityError::Validation("Return URL is invalid.".to_owned()))?;
        let discovery = self.protocol.discover(provider.issuer()).await?;
        let state = opaque_value(32)?;
        let nonce = opaque_value(32)?;
        let code_verifier = opaque_value(64)?;
        let authorization_url = build_authorization_url(
            &provider,
            &discovery,
            redirect_uri,
            &state,
            &nonce,
            &code_verifier,
        )?;
        let now = self.clock.now();
        self.store
            .start_login(
                OidcLoginState {
                    id: Uuid::now_v7(),
                    provider_id,
                    state_hash: hash_opaque_value(&state),
                    nonce,
                    code_verifier,
                    return_url,
                    created_at: now,
                    expires_at: now + self.login_state_lifetime,
                },
                now,
            )
            .await?;
        Ok(OidcLoginStart { authorization_url })
    }

    pub async fn complete_login(
        &self,
        provider_id: Uuid,
        code: &str,
        state_value: &str,
        redirect_uri: &str,
        metadata: SessionMetadata,
    ) -> Result<OidcLoginComplete, IdentityError> {
        validate_id(provider_id)?;
        if code.trim().is_empty() || state_value.trim().is_empty() {
            return Err(IdentityError::Validation(
                "Missing OIDC callback parameters.".to_owned(),
            ));
        }
        validate_absolute_http_url(redirect_uri, "Redirect URI is invalid.")?;
        let provider = self
            .store
            .get(provider_id)
            .await?
            .filter(OidcProvider::enabled)
            .ok_or(IdentityError::NotFound)?;
        let login_state = self
            .store
            .consume_login_state(&hash_opaque_value(state_value))
            .await?
            .filter(|state| state.provider_id == provider_id && state.expires_at > self.clock.now())
            .ok_or_else(|| {
                IdentityError::Validation("OIDC login state is invalid or expired.".to_owned())
            })?;
        let discovery = self.protocol.discover(provider.issuer()).await?;
        let client_secret = match provider.client_secret_ciphertext() {
            Some(secret) => self.protector.unprotect(secret)?,
            None => zeroize::Zeroizing::new(Vec::new()),
        };
        let client_secret =
            std::str::from_utf8(client_secret.as_slice()).map_err(|_| IdentityError::Credential)?;
        let oidc_identity = self
            .protocol
            .exchange_and_validate(
                &provider,
                &discovery,
                client_secret,
                code,
                redirect_uri,
                &login_state.code_verifier,
                &login_state.nonce,
            )
            .await?;
        validate_provider_policy(&provider, &oidc_identity)?;
        let proposed_name = proposed_user_name(&oidc_identity);
        let user = self
            .store
            .resolve_identity(&provider, &oidc_identity, &proposed_name, self.clock.now())
            .await?;
        if !user.enabled {
            return Err(IdentityError::InvalidCredentials);
        }
        let session = self.identity.issue_session(&user, metadata).await?;
        Ok(OidcLoginComplete {
            session,
            return_url: login_state.return_url,
        })
    }
}

pub fn build_authorization_url(
    provider: &OidcProvider,
    discovery: &OidcDiscovery,
    redirect_uri: &str,
    state: &str,
    nonce: &str,
    code_verifier: &str,
) -> Result<String, IdentityError> {
    let mut url = Url::parse(&discovery.authorization_endpoint).map_err(|_| {
        IdentityError::Validation("OIDC authorization endpoint is invalid.".to_owned())
    })?;
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(code_verifier.as_bytes()));
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", provider.client_id())
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", provider.scopes())
        .append_pair("state", state)
        .append_pair("nonce", nonce)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256");
    Ok(url.into())
}

#[must_use]
pub fn normalize_return_url(
    value: &str,
    redirect_uri: &str,
    allowed_origins: &[String],
) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().any(char::is_control) {
        return None;
    }
    if value.starts_with('/') && !value.starts_with("//") && !value.starts_with("/\\") {
        return Url::parse("https://relative.invalid")
            .ok()?
            .join(value)
            .ok()
            .map(|url| {
                format!(
                    "{}{}",
                    url.path(),
                    url.query()
                        .map_or(String::new(), |query| format!("?{query}"))
                )
            });
    }
    let candidate = Url::parse(value).ok()?;
    let callback = Url::parse(redirect_uri).ok()?;
    if !matches!(candidate.scheme(), "http" | "https") {
        return None;
    }
    if same_origin(&candidate, &callback)
        || allowed_origins
            .iter()
            .any(|origin| Url::parse(origin).is_ok_and(|allowed| same_origin(&candidate, &allowed)))
    {
        Some(candidate.into())
    } else {
        None
    }
}

fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme().eq_ignore_ascii_case(right.scheme())
        && left
            .host_str()
            .zip(right.host_str())
            .is_some_and(|(left, right)| left.eq_ignore_ascii_case(right))
        && left.port_or_known_default() == right.port_or_known_default()
}

fn validate_provider_policy(
    provider: &OidcProvider,
    identity: &OidcIdentity,
) -> Result<(), IdentityError> {
    if provider.require_email_verified() && !identity.email_verified {
        return Err(IdentityError::Validation(
            "OIDC account email is not verified.".to_owned(),
        ));
    }
    if let Some(domains) = provider.allowed_email_domains() {
        let domain = identity
            .email
            .as_deref()
            .and_then(|email| email.rsplit_once('@').map(|(_, domain)| domain))
            .ok_or_else(|| {
                IdentityError::Validation("OIDC account email is required.".to_owned())
            })?;
        if !domains
            .split(',')
            .any(|allowed| allowed.eq_ignore_ascii_case(domain))
        {
            return Err(IdentityError::Validation(
                "OIDC account email domain is not allowed.".to_owned(),
            ));
        }
    }
    if let Some(name) = provider.required_claim_name() {
        let allowed = provider
            .required_claim_values()
            .unwrap_or_default()
            .split(',');
        if !allowed.into_iter().any(|allowed| {
            identity
                .claims
                .get(name)
                .is_some_and(|values| values.iter().any(|value| value == allowed))
        }) {
            return Err(IdentityError::Validation(
                "OIDC account is missing the required claim.".to_owned(),
            ));
        }
    }
    Ok(())
}

fn proposed_user_name(identity: &OidcIdentity) -> String {
    let source = identity
        .name
        .as_deref()
        .or(identity.email.as_deref())
        .unwrap_or(&identity.subject);
    let mut base = source
        .split('@')
        .next()
        .unwrap_or(source)
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned();
    if base.chars().count() < 3 {
        base = format!("oidc-{base}");
    }
    base.chars().take(56).collect()
}

fn opaque_value(bytes: usize) -> Result<String, IdentityError> {
    let mut value = vec![0_u8; bytes];
    fill(&mut value).map_err(|_| IdentityError::Credential)?;
    Ok(URL_SAFE_NO_PAD.encode(value))
}

#[must_use]
pub fn hash_opaque_value(value: &str) -> String {
    Sha256::digest(value.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn validate_id(id: Uuid) -> Result<(), IdentityError> {
    if id.is_nil() {
        Err(IdentityError::Validation(
            "OIDC provider ID must be valid.".to_owned(),
        ))
    } else {
        Ok(())
    }
}

fn validate_secret(secret: Option<&str>) -> Result<(), IdentityError> {
    if secret.is_some_and(|secret| secret.chars().count() > MAXIMUM_CLIENT_SECRET_CHARS) {
        Err(IdentityError::Validation(
            "Client secret cannot exceed 4096 characters.".to_owned(),
        ))
    } else {
        Ok(())
    }
}

async fn validate_default_role(
    store: &dyn OidcStore,
    role_id: Option<Uuid>,
) -> Result<(), IdentityError> {
    if let Some(role_id) = role_id
        && !store.role_exists(role_id).await?
    {
        return Err(IdentityError::Validation(
            "Default role does not exist.".to_owned(),
        ));
    }
    Ok(())
}

fn required_patch(
    patch: PatchField<String>,
    current: &str,
    field: &str,
) -> Result<String, IdentityError> {
    match patch {
        PatchField::Missing => Ok(current.to_owned()),
        PatchField::Null => Err(IdentityError::Validation(format!(
            "{field} must not be null."
        ))),
        PatchField::Value(value) => Ok(value),
    }
}

fn optional_patch(patch: PatchField<String>, current: Option<&str>) -> Option<String> {
    match patch {
        PatchField::Missing => current.map(str::to_owned),
        PatchField::Null => None,
        PatchField::Value(value) => Some(value),
    }
}

fn value_patch<T: Copy>(patch: PatchField<T>, current: T, field: &str) -> Result<T, IdentityError> {
    match patch {
        PatchField::Missing => Ok(current),
        PatchField::Null => Err(IdentityError::Validation(format!(
            "{field} must not be null."
        ))),
        PatchField::Value(value) => Ok(value),
    }
}

fn validate_absolute_http_url(value: &str, message: &str) -> Result<(), IdentityError> {
    if Url::parse(value)
        .is_ok_and(|url| matches!(url.scheme(), "http" | "https") && url.host_str().is_some())
    {
        Ok(())
    } else {
        Err(IdentityError::Validation(message.to_owned()))
    }
}

fn validation(error: impl std::fmt::Display) -> IdentityError {
    IdentityError::Validation(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SYSTEM_ACTOR_ID;

    fn provider() -> OidcProvider {
        OidcProvider::new(
            "github".to_owned(),
            None,
            "GitHub".to_owned(),
            "https://issuer.test".to_owned(),
            "client-id".to_owned(),
            None,
            DEFAULT_OIDC_SCOPES.to_owned(),
            true,
            false,
            true,
            true,
            None,
            None,
            None,
            None,
            ActorId::new(SYSTEM_ACTOR_ID),
            Utc::now(),
        )
        .unwrap()
    }

    #[test]
    fn authorization_request_contains_pkce_and_oidc_parameters() {
        let url = build_authorization_url(
            &provider(),
            &OidcDiscovery {
                issuer: "https://issuer.test".to_owned(),
                authorization_endpoint: "https://issuer.test/authorize".to_owned(),
                token_endpoint: "https://issuer.test/token".to_owned(),
                jwks_uri: "https://issuer.test/jwks".to_owned(),
            },
            "https://citadel.test/api/v1/authentication/oidc/00000000-0000-0000-0000-000000000001/callback",
            "state-value", "nonce-value", "deterministic-code-verifier",
        ).unwrap();
        let url = Url::parse(&url).unwrap();
        let query = url.query_pairs().into_owned().collect::<BTreeMap<_, _>>();
        assert_eq!(query["response_type"], "code");
        assert_eq!(query["client_id"], "client-id");
        assert_eq!(query["scope"], DEFAULT_OIDC_SCOPES);
        assert_eq!(query["state"], "state-value");
        assert_eq!(query["nonce"], "nonce-value");
        assert_eq!(query["code_challenge_method"], "S256");
        assert_eq!(
            query["code_challenge"],
            URL_SAFE_NO_PAD.encode(Sha256::digest(b"deterministic-code-verifier"))
        );
    }

    #[test]
    fn return_url_accepts_only_root_relative_or_trusted_origins() {
        let callback = "https://citadel.test/api/v1/authentication/oidc/00000000-0000-0000-0000-000000000001/callback";
        let allowed = vec!["http://localhost:5173".to_owned()];
        assert_eq!(
            normalize_return_url("https://citadel.test/login", callback, &allowed).as_deref(),
            Some("https://citadel.test/login")
        );
        assert_eq!(
            normalize_return_url("http://localhost:5173/login", callback, &allowed).as_deref(),
            Some("http://localhost:5173/login")
        );
        assert_eq!(
            normalize_return_url("/login", callback, &allowed).as_deref(),
            Some("/login")
        );
        for rejected in [
            "https://evil.test/login",
            "//evil.test/login",
            "javascript:alert(1)",
            "login",
            "/\\evil.test/login",
        ] {
            assert_eq!(
                normalize_return_url(rejected, callback, &allowed),
                None,
                "{rejected}"
            );
        }
    }

    #[test]
    fn claim_policy_supports_string_and_array_values() {
        let mut provider = provider();
        let now = Utc::now();
        provider
            .update(
                provider.name().to_owned(),
                None,
                provider.display_name().to_owned(),
                provider.issuer().to_owned(),
                provider.client_id().to_owned(),
                None,
                provider.scopes().to_owned(),
                true,
                false,
                true,
                true,
                Some("example.com".to_owned()),
                Some("groups".to_owned()),
                Some("operators,admins".to_owned()),
                None,
                now,
            )
            .unwrap();
        let identity = OidcIdentity {
            subject: "subject".to_owned(),
            email: Some("user@example.com".to_owned()),
            email_verified: true,
            name: None,
            claims: BTreeMap::from([(
                "groups".to_owned(),
                vec!["users".to_owned(), "admins".to_owned()],
            )]),
        };
        assert!(validate_provider_policy(&provider, &identity).is_ok());
    }
}
