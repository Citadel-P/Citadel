use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bytes::BytesMut;
use citadel_identity::OidcProvider;
use citadel_identity::{IdentityError, OidcDiscovery, OidcIdentity, OidcProtocol};
use futures_util::StreamExt;
use futures_util::future::BoxFuture;
use jsonwebtoken::jwk::{Jwk, JwkSet, KeyAlgorithm, KeyOperations, PublicKeyUse};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use reqwest::header::CONTENT_TYPE;
use serde::Deserialize;
use serde_json::Value;
use subtle::ConstantTimeEq;
use tokio::sync::RwLock;
use url::Url;

const MAXIMUM_RESPONSE_BYTES: usize = 1024 * 1024;
const MAXIMUM_CACHE_ENTRIES: usize = 32;
const CACHE_LIFETIME: Duration = Duration::from_secs(5 * 60);

#[derive(Clone)]
pub struct OidcHttpProtocol {
    client: reqwest::Client,
    cache: Arc<RwLock<ProtocolCache>>,
}

#[derive(Default)]
struct ProtocolCache {
    discovery: HashMap<String, Cached<OidcDiscovery>>,
    jwks: HashMap<String, Cached<JwkSet>>,
}

struct Cached<T> {
    expires_at: Instant,
    value: T,
}

#[derive(Debug, Deserialize)]
struct DiscoveryDocument {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    jwks_uri: String,
}

#[derive(Debug, Deserialize)]
struct TokenDocument {
    id_token: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

impl OidcHttpProtocol {
    pub fn new(timeout: Duration) -> Result<Self, IdentityError> {
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(protocol_error)?;
        Ok(Self {
            client,
            cache: Arc::new(RwLock::new(ProtocolCache::default())),
        })
    }

    async fn discover_uncached(&self, issuer: &str) -> Result<OidcDiscovery, IdentityError> {
        let normalized = normalize_issuer(issuer)?;
        if let Some(value) = self.cached_discovery(&normalized).await {
            return Ok(value);
        }
        let endpoint = format!("{normalized}/.well-known/openid-configuration");
        let response = self
            .client
            .get(&endpoint)
            .send()
            .await
            .map_err(protocol_error)?;
        if !response.status().is_success() {
            return Err(IdentityError::Validation(format!(
                "OIDC discovery failed with status code {}.",
                response.status().as_u16()
            )));
        }
        let document: DiscoveryDocument = serde_json::from_slice(&bounded_body(response).await?)
            .map_err(|_| {
                IdentityError::Validation("OIDC discovery response is not valid JSON.".to_owned())
            })?;
        if document.issuer.trim_end_matches('/') != normalized {
            return Err(IdentityError::Validation(
                "Discovery issuer does not match the configured issuer.".to_owned(),
            ));
        }
        for endpoint in [
            &document.authorization_endpoint,
            &document.token_endpoint,
            &document.jwks_uri,
        ] {
            absolute_http_url(endpoint).ok_or_else(|| {
                IdentityError::Validation(
                    "OIDC discovery endpoints must be absolute URLs.".to_owned(),
                )
            })?;
        }
        let discovery = OidcDiscovery {
            issuer: document.issuer,
            authorization_endpoint: document.authorization_endpoint,
            token_endpoint: document.token_endpoint,
            jwks_uri: document.jwks_uri,
        };
        self.cache_discovery(normalized, discovery.clone()).await;
        Ok(discovery)
    }

    async fn cached_discovery(&self, issuer: &str) -> Option<OidcDiscovery> {
        self.cache
            .read()
            .await
            .discovery
            .get(issuer)
            .filter(|cached| cached.expires_at > Instant::now())
            .map(|cached| cached.value.clone())
    }

    async fn cache_discovery(&self, issuer: String, value: OidcDiscovery) {
        let mut cache = self.cache.write().await;
        retain_bounded(&mut cache.discovery);
        cache.discovery.insert(
            issuer,
            Cached {
                expires_at: Instant::now() + CACHE_LIFETIME,
                value,
            },
        );
    }

    async fn jwks(&self, uri: &str, force_refresh: bool) -> Result<JwkSet, IdentityError> {
        if !force_refresh
            && let Some(value) = self
                .cache
                .read()
                .await
                .jwks
                .get(uri)
                .filter(|cached| cached.expires_at > Instant::now())
                .map(|cached| cached.value.clone())
        {
            return Ok(value);
        }
        absolute_http_url(uri)
            .ok_or_else(|| IdentityError::Validation("OIDC JWKS URI is invalid.".to_owned()))?;
        let response = self.client.get(uri).send().await.map_err(protocol_error)?;
        if !response.status().is_success() {
            return Err(IdentityError::Validation(format!(
                "OIDC JWKS request failed with status code {}.",
                response.status().as_u16()
            )));
        }
        let keys: JwkSet =
            serde_json::from_slice(&bounded_body(response).await?).map_err(|_| {
                IdentityError::Validation("OIDC JWKS response is not valid JSON.".to_owned())
            })?;
        if keys.keys.is_empty() {
            return Err(IdentityError::Validation(
                "OIDC JWKS response contains no signing keys.".to_owned(),
            ));
        }
        let mut cache = self.cache.write().await;
        retain_bounded(&mut cache.jwks);
        cache.jwks.insert(
            uri.to_owned(),
            Cached {
                expires_at: Instant::now() + CACHE_LIFETIME,
                value: keys.clone(),
            },
        );
        Ok(keys)
    }

    #[allow(clippy::too_many_arguments)]
    async fn exchange_and_validate_inner(
        &self,
        provider: &OidcProvider,
        discovery: &OidcDiscovery,
        client_secret: &str,
        code: &str,
        redirect_uri: &str,
        code_verifier: &str,
        nonce: &str,
    ) -> Result<OidcIdentity, IdentityError> {
        let form = {
            let mut serializer = url::form_urlencoded::Serializer::new(String::new());
            serializer
                .append_pair("grant_type", "authorization_code")
                .append_pair("code", code)
                .append_pair("redirect_uri", redirect_uri)
                .append_pair("client_id", provider.client_id())
                .append_pair("code_verifier", code_verifier);
            if !client_secret.is_empty() {
                serializer.append_pair("client_secret", client_secret);
            }
            serializer.finish()
        };
        let response = self
            .client
            .post(&discovery.token_endpoint)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(form)
            .send()
            .await
            .map_err(protocol_error)?;
        let status = response.status();
        let document: TokenDocument = serde_json::from_slice(&bounded_body(response).await?)
            .map_err(|_| {
                IdentityError::Validation("OIDC token response is not valid JSON.".to_owned())
            })?;
        if !status.is_success() {
            return Err(IdentityError::Validation(
                document
                    .error_description
                    .or(document.error)
                    .unwrap_or_else(|| {
                        format!(
                            "OIDC token endpoint failed with status code {}.",
                            status.as_u16()
                        )
                    }),
            ));
        }
        let id_token = document.id_token.ok_or_else(|| {
            IdentityError::Validation("OIDC token response did not include an ID token.".to_owned())
        })?;
        self.validate_id_token(provider, discovery, &id_token, nonce)
            .await
    }

    async fn validate_id_token(
        &self,
        provider: &OidcProvider,
        discovery: &OidcDiscovery,
        token: &str,
        expected_nonce: &str,
    ) -> Result<OidcIdentity, IdentityError> {
        let header = decode_header(token).map_err(|_| invalid_id_token())?;
        if !matches!(
            header.alg,
            Algorithm::RS256
                | Algorithm::RS384
                | Algorithm::RS512
                | Algorithm::PS256
                | Algorithm::PS384
                | Algorithm::PS512
                | Algorithm::ES256
                | Algorithm::ES384
                | Algorithm::EdDSA
        ) {
            return Err(invalid_id_token());
        }
        let keys = self.jwks(&discovery.jwks_uri, false).await?;
        let key = select_signing_key(&keys, header.kid.as_deref());
        let key = match key {
            Some(key) => key.clone(),
            None => {
                let refreshed = self.jwks(&discovery.jwks_uri, true).await?;
                select_signing_key(&refreshed, header.kid.as_deref())
                    .cloned()
                    .ok_or_else(invalid_id_token)?
            }
        };
        if key
            .common
            .public_key_use
            .as_ref()
            .is_some_and(|key_use| key_use != &PublicKeyUse::Signature)
            || key
                .common
                .key_operations
                .as_ref()
                .is_some_and(|operations| !operations.contains(&KeyOperations::Verify))
            || key
                .common
                .key_algorithm
                .as_ref()
                .is_some_and(|algorithm| !jwk_algorithm_matches(algorithm, header.alg))
        {
            return Err(invalid_id_token());
        }
        let decoding_key = DecodingKey::from_jwk(&key).map_err(|_| invalid_id_token())?;
        let mut validation = Validation::new(header.alg);
        validation.leeway = 120;
        validation.validate_nbf = true;
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
        validation.set_issuer(&[discovery.issuer.as_str()]);
        validation.set_audience(&[provider.client_id()]);
        let claims = decode::<Value>(token, &decoding_key, &validation)
            .map_err(|_| invalid_id_token())?
            .claims;
        let object = claims.as_object().ok_or_else(invalid_id_token)?;
        let token_nonce = object
            .get("nonce")
            .and_then(Value::as_str)
            .ok_or_else(invalid_id_token)?;
        if token_nonce.len() != expected_nonce.len()
            || !bool::from(token_nonce.as_bytes().ct_eq(expected_nonce.as_bytes()))
        {
            return Err(IdentityError::Validation(
                "OIDC ID token nonce is invalid.".to_owned(),
            ));
        }
        validate_authorized_party(object, provider.client_id())?;
        let subject = object
            .get("sub")
            .and_then(Value::as_str)
            .filter(|value| !value.trim().is_empty())
            .ok_or_else(|| {
                IdentityError::Validation("OIDC ID token subject is missing.".to_owned())
            })?
            .to_owned();
        let email = object
            .get("email")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let email_verified = object.get("email_verified").is_some_and(|value| {
            value.as_bool().unwrap_or_else(|| {
                value
                    .as_str()
                    .is_some_and(|value| value.eq_ignore_ascii_case("true"))
            })
        });
        let name = object
            .get("preferred_username")
            .and_then(Value::as_str)
            .or_else(|| object.get("name").and_then(Value::as_str))
            .or(email.as_deref())
            .map(str::to_owned);
        Ok(OidcIdentity {
            subject,
            email,
            email_verified,
            name,
            claims: claim_values(object),
        })
    }
}

impl OidcProtocol for OidcHttpProtocol {
    fn discover<'a>(
        &'a self,
        issuer: &'a str,
    ) -> BoxFuture<'a, Result<OidcDiscovery, IdentityError>> {
        Box::pin(async move { self.discover_uncached(issuer).await })
    }

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
    ) -> BoxFuture<'a, Result<OidcIdentity, IdentityError>> {
        Box::pin(async move {
            self.exchange_and_validate_inner(
                provider,
                discovery,
                client_secret,
                code,
                redirect_uri,
                code_verifier,
                nonce,
            )
            .await
        })
    }
}

fn select_signing_key<'a>(keys: &'a JwkSet, kid: Option<&str>) -> Option<&'a Jwk> {
    match kid {
        Some(kid) => keys.find(kid),
        None if keys.keys.len() == 1 => keys.keys.first(),
        None => None,
    }
}

fn jwk_algorithm_matches(key: &KeyAlgorithm, token: Algorithm) -> bool {
    matches!(
        (key, token),
        (KeyAlgorithm::RS256, Algorithm::RS256)
            | (KeyAlgorithm::RS384, Algorithm::RS384)
            | (KeyAlgorithm::RS512, Algorithm::RS512)
            | (KeyAlgorithm::PS256, Algorithm::PS256)
            | (KeyAlgorithm::PS384, Algorithm::PS384)
            | (KeyAlgorithm::PS512, Algorithm::PS512)
            | (KeyAlgorithm::ES256, Algorithm::ES256)
            | (KeyAlgorithm::ES384, Algorithm::ES384)
            | (KeyAlgorithm::EdDSA, Algorithm::EdDSA)
    )
}

fn validate_authorized_party(
    claims: &serde_json::Map<String, Value>,
    client_id: &str,
) -> Result<(), IdentityError> {
    let audiences = match claims.get("aud") {
        Some(Value::String(value)) => vec![value.as_str()],
        Some(Value::Array(values)) => values.iter().filter_map(Value::as_str).collect(),
        _ => return Err(invalid_id_token()),
    };
    let authorized_party = claims.get("azp").and_then(Value::as_str);
    if (audiences.len() > 1 && authorized_party != Some(client_id))
        || authorized_party.is_some_and(|value| value != client_id)
    {
        return Err(invalid_id_token());
    }
    Ok(())
}

fn claim_values(claims: &serde_json::Map<String, Value>) -> BTreeMap<String, Vec<String>> {
    claims
        .iter()
        .filter_map(|(name, value)| {
            let values = match value {
                Value::String(value) => vec![value.clone()],
                Value::Array(values) => values
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect(),
                _ => Vec::new(),
            };
            (!values.is_empty()).then(|| (name.clone(), values))
        })
        .collect()
}

async fn bounded_body(response: reqwest::Response) -> Result<Vec<u8>, IdentityError> {
    if response
        .content_length()
        .is_some_and(|length| length > MAXIMUM_RESPONSE_BYTES as u64)
    {
        return Err(IdentityError::Validation(
            "OIDC response is too large.".to_owned(),
        ));
    }
    let mut body = BytesMut::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(protocol_error)?;
        if body.len().saturating_add(chunk.len()) > MAXIMUM_RESPONSE_BYTES {
            return Err(IdentityError::Validation(
                "OIDC response is too large.".to_owned(),
            ));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body.to_vec())
}

fn retain_bounded<T>(cache: &mut HashMap<String, Cached<T>>) {
    let now = Instant::now();
    cache.retain(|_, value| value.expires_at > now);
    if cache.len() >= MAXIMUM_CACHE_ENTRIES {
        cache.clear();
    }
}

fn normalize_issuer(issuer: &str) -> Result<String, IdentityError> {
    let issuer = issuer.trim().trim_end_matches('/');
    absolute_http_url(issuer)
        .map(|url| url.to_string().trim_end_matches('/').to_owned())
        .ok_or_else(|| {
            IdentityError::Validation("Issuer must be an absolute HTTP or HTTPS URL.".to_owned())
        })
}

fn absolute_http_url(value: &str) -> Option<Url> {
    Url::parse(value).ok().filter(|url| {
        matches!(url.scheme(), "http" | "https")
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
    })
}

fn invalid_id_token() -> IdentityError {
    IdentityError::Validation("OIDC ID token validation failed.".to_owned())
}

fn protocol_error(error: impl std::fmt::Display) -> IdentityError {
    IdentityError::Validation(format!("OIDC protocol request failed: {error}"))
}
