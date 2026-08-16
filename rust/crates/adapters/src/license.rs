use std::collections::HashSet;
use std::sync::RwLock;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Utc};
use citadel_application::{EntitlementService, IdentityError};
use ed25519_dalek::pkcs8::DecodePublicKey;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use futures_util::future::BoxFuture;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use uuid::Uuid;

const MAX_LICENSE_BYTES: usize = 64 * 1024;
const CUSTOM_ACCESS_CONTROL: &str = "custom-access-control";
const PRODUCTION_KEY: &str = r#"-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEA1lCChB1fwbhdK3a844AtpzUfCeGwn+o8+Al+OOHrCGQ=
-----END PUBLIC KEY-----"#;
const DEVELOPMENT_KEY: &str = r#"-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAiKohdkN3GouALrVhfXvUHN/v4vZ4c5FNXVUHGaxxzzI=
-----END PUBLIC KEY-----"#;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProtectedHeader {
    alg: String,
    typ: String,
    kid: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LicensePayload {
    schema: u8,
    product: String,
    issuer: String,
    audience: String,
    license_id: String,
    replaced_license_id: Option<String>,
    customer: LicenseCustomer,
    edition: String,
    instance_id: Uuid,
    issued_at: DateTime<Utc>,
    not_before: DateTime<Utc>,
    expires_at: DateTime<Utc>,
    grace_until: Option<DateTime<Utc>>,
    #[serde(default)]
    limits: std::collections::HashMap<String, i64>,
    #[serde(default)]
    capabilities: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct LicenseCustomer {
    id: String,
    name: String,
}

#[derive(Debug, Clone)]
struct VerifiedStaticLicense {
    fingerprint: String,
    payload: LicensePayload,
}

pub struct PostgresLicenseEntitlementService {
    pool: PgPool,
    cached: RwLock<Option<VerifiedStaticLicense>>,
}

impl PostgresLicenseEntitlementService {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            cached: RwLock::new(None),
        }
    }

    pub fn invalidate(&self) {
        *self.cached.write().expect("license cache lock poisoned") = None;
    }

    async fn effective_custom_access_control(&self) -> Result<bool, IdentityError> {
        let row = sqlx::query(
            r#"
SELECT license.rawlicense, license.fingerprint, identity.instanceid
FROM installedlicenses license
JOIN citadelinstanceidentity identity ON identity.id = 1
WHERE license.id = 1
"#,
        )
        .fetch_optional(&self.pool)
        .await
        .map_err(|error| IdentityError::Storage(error.to_string()))?;
        let Some(row) = row else {
            return Ok(false);
        };
        let fingerprint: String = row
            .try_get("fingerprint")
            .map_err(|error| IdentityError::Storage(error.to_string()))?;
        let instance_id: Uuid = row
            .try_get("instanceid")
            .map_err(|error| IdentityError::Storage(error.to_string()))?;

        let cached = self
            .cached
            .read()
            .expect("license cache lock poisoned")
            .as_ref()
            .filter(|cached| cached.fingerprint == fingerprint)
            .cloned();
        let verified = if let Some(cached) = cached {
            cached
        } else {
            let raw: String = row
                .try_get("rawlicense")
                .map_err(|error| IdentityError::Storage(error.to_string()))?;
            let verified = match verify_license(&raw, instance_id) {
                Ok(verified) => verified,
                Err(IdentityError::Credential) => return Ok(false),
                Err(error) => return Err(error),
            };
            if verified.fingerprint != fingerprint {
                return Ok(false);
            }
            *self.cached.write().expect("license cache lock poisoned") = Some(verified.clone());
            verified
        };

        Ok(is_temporally_active(&verified.payload, Utc::now())
            && effective_capabilities(&verified.payload).contains(CUSTOM_ACCESS_CONTROL))
    }
}

impl EntitlementService for PostgresLicenseEntitlementService {
    fn custom_access_control_enabled(&self) -> BoxFuture<'_, Result<bool, IdentityError>> {
        Box::pin(async move { self.effective_custom_access_control().await })
    }
}

fn verify_license(raw: &str, instance_id: Uuid) -> Result<VerifiedStaticLicense, IdentityError> {
    let normalized =
        raw.trim_matches(|character| matches!(character, ' ' | '\t' | '\r' | '\n' | '\x0c'));
    if normalized.is_empty()
        || normalized.len() > MAX_LICENSE_BYTES
        || !normalized.is_ascii()
        || normalized.chars().any(char::is_whitespace)
    {
        return Err(IdentityError::Credential);
    }
    let mut segments = normalized.split('.');
    let header_segment = segments.next().ok_or(IdentityError::Credential)?;
    let payload_segment = segments.next().ok_or(IdentityError::Credential)?;
    let signature_segment = segments.next().ok_or(IdentityError::Credential)?;
    if segments.next().is_some()
        || header_segment.is_empty()
        || payload_segment.is_empty()
        || signature_segment.is_empty()
    {
        return Err(IdentityError::Credential);
    }
    let header_bytes = URL_SAFE_NO_PAD
        .decode(header_segment.as_bytes())
        .map_err(|_| IdentityError::Credential)?;
    let header: ProtectedHeader =
        serde_json::from_slice(&header_bytes).map_err(|_| IdentityError::Credential)?;
    if header.alg != "Ed25519"
        || header.typ != "citadel-license+jws"
        || header.kid.is_empty()
        || header.kid.len() > 128
    {
        return Err(IdentityError::Credential);
    }
    let public_key = match header.kid.as_str() {
        "citadel-license-2026-01" => PRODUCTION_KEY,
        "citadel-license-dev-2026-01" => DEVELOPMENT_KEY,
        _ => return Err(IdentityError::Credential),
    };
    let verifying_key =
        VerifyingKey::from_public_key_pem(public_key).map_err(|_| IdentityError::Credential)?;
    let signature_bytes = URL_SAFE_NO_PAD
        .decode(signature_segment.as_bytes())
        .map_err(|_| IdentityError::Credential)?;
    let signature =
        Signature::from_slice(&signature_bytes).map_err(|_| IdentityError::Credential)?;
    verifying_key
        .verify(
            format!("{header_segment}.{payload_segment}").as_bytes(),
            &signature,
        )
        .map_err(|_| IdentityError::Credential)?;

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload_segment.as_bytes())
        .map_err(|_| IdentityError::Credential)?;
    let payload_json: serde_json::Value =
        serde_json::from_slice(&payload_bytes).map_err(|_| IdentityError::Credential)?;
    for field in ["issuedAt", "notBefore", "expiresAt"] {
        if !payload_json
            .get(field)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| value.ends_with('Z'))
        {
            return Err(IdentityError::Credential);
        }
    }
    if payload_json
        .get("graceUntil")
        .is_some_and(|value| !value.is_null())
        && !payload_json
            .get("graceUntil")
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| value.ends_with('Z'))
    {
        return Err(IdentityError::Credential);
    }
    let payload: LicensePayload =
        serde_json::from_slice(&payload_bytes).map_err(|_| IdentityError::Credential)?;
    validate_payload(&payload, instance_id)?;
    Ok(VerifiedStaticLicense {
        fingerprint: hex_digest(normalized.as_bytes()),
        payload,
    })
}

fn validate_payload(payload: &LicensePayload, instance_id: Uuid) -> Result<(), IdentityError> {
    let supported_edition = match payload.schema {
        1 => payload.edition == "Business",
        2 => matches!(payload.edition.as_str(), "Team" | "Enterprise"),
        _ => false,
    };
    if payload.product != "citadel"
        || payload.issuer != "citadel-p"
        || payload.audience != "citadel-core"
        || payload.instance_id != instance_id
        || !supported_edition
        || payload.license_id.trim().is_empty()
        || payload.license_id.len() > 128
        || payload
            .replaced_license_id
            .as_ref()
            .is_some_and(|id| id.is_empty() || id.len() > 128 || id == &payload.license_id)
        || payload.customer.id.trim().is_empty()
        || payload.customer.id.len() > 128
        || payload.customer.name.trim().is_empty()
        || payload.customer.name.len() > 256
        || payload.issued_at > payload.not_before
        || payload.issued_at > payload.expires_at
        || payload.not_before > payload.expires_at
        || payload
            .grace_until
            .is_some_and(|grace| grace < payload.expires_at)
        || payload.capabilities.len() > 64
        || payload.limits.len() > 32
    {
        return Err(IdentityError::Credential);
    }
    let mut capabilities = HashSet::new();
    if payload.capabilities.iter().any(|capability| {
        capability.is_empty() || capability.len() > 128 || !capabilities.insert(capability.as_str())
    }) {
        return Err(IdentityError::Credential);
    }
    if payload.schema == 1 && payload.limits.values().any(|limit| *limit < 0) {
        return Err(IdentityError::Credential);
    }
    Ok(())
}

fn is_temporally_active(payload: &LicensePayload, now: DateTime<Utc>) -> bool {
    payload.not_before <= now
        && (now <= payload.expires_at || payload.grace_until.is_some_and(|grace| now <= grace))
}

fn effective_capabilities(payload: &LicensePayload) -> HashSet<&str> {
    if payload.schema == 1 {
        return HashSet::from([
            CUSTOM_ACCESS_CONTROL,
            "automated-operations",
            "advanced-alerting",
            "operational-guardrails",
            "elastic-build-execution",
        ]);
    }
    payload.capabilities.iter().map(String::as_str).collect()
}

fn hex_digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temporal_state_includes_grace_but_not_future_or_expired() {
        let now = Utc::now();
        let mut payload = LicensePayload {
            schema: 2,
            product: "citadel".into(),
            issuer: "citadel-p".into(),
            audience: "citadel-core".into(),
            license_id: "license".into(),
            replaced_license_id: None,
            customer: LicenseCustomer {
                id: "customer".into(),
                name: "Customer".into(),
            },
            edition: "Team".into(),
            instance_id: Uuid::now_v7(),
            issued_at: now - chrono::Duration::days(2),
            not_before: now - chrono::Duration::days(1),
            expires_at: now - chrono::Duration::minutes(1),
            grace_until: Some(now + chrono::Duration::days(1)),
            limits: Default::default(),
            capabilities: vec![CUSTOM_ACCESS_CONTROL.into()],
        };
        assert!(is_temporally_active(&payload, now));
        payload.grace_until = Some(now - chrono::Duration::seconds(1));
        assert!(!is_temporally_active(&payload, now));
    }
}
