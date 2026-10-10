use std::collections::{BTreeMap, BTreeSet};

use std::sync::Arc;

use base64::Engine;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use chrono::{DateTime, Utc};

use citadel_licensing::LicenseVerifier;

use citadel_licensing::LicenseError;

use citadel_licensing::{
    CURRENT_LICENSE_SCHEMA, CitadelInstanceIdentity, LEGACY_BUSINESS_EDITION,
    LEGACY_LICENSE_SCHEMA, LICENSE_AUDIENCE, LICENSE_ISSUER, LICENSE_PRODUCT, LicenseCapability,
    LicensePayload, LicenseStatus, LicenseVerificationResult, VerifiedLicense,
};

use ed25519_dalek::pkcs8::DecodePublicKey;

use ed25519_dalek::{Signature, Verifier, VerifyingKey};

use serde::Deserialize;

use sha2::{Digest, Sha256};

use uuid::Uuid;

const MAX_LICENSE_BYTES: usize = 64 * 1024;

const MAX_JSON_DEPTH: usize = 16;

const LICENSE_TYPE: &str = "citadel-license+jws";

const LICENSE_ALGORITHM: &str = "Ed25519";

const PRODUCTION_KEY: &str = r#"-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEA1lCChB1fwbhdK3a844AtpzUfCeGwn+o8+Al+OOHrCGQ=
-----END PUBLIC KEY-----"#;

const DEVELOPMENT_KEY: &str = r#"-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEA0soUgSf8Bu4xvD4/BgKfxUyqreZRuYnYi8n/HqejyfE=
-----END PUBLIC KEY-----"#;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProtectedHeader {
    alg: String,
    typ: String,
    kid: String,
}

#[derive(Clone)]
pub struct Ed25519LicenseVerifier {
    keys: Arc<BTreeMap<String, VerifyingKey>>,
}

impl Default for Ed25519LicenseVerifier {
    fn default() -> Self {
        Self::from_public_key_pems([
            ("citadel-license-2026-01", PRODUCTION_KEY),
            ("citadel-license-dev-2026-02", DEVELOPMENT_KEY),
        ])
        .expect("embedded Citadel license public keys are valid")
    }
}

impl Ed25519LicenseVerifier {
    pub fn from_public_key_pems<'a>(
        keys: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Result<Self, LicenseError> {
        let keys = keys
            .into_iter()
            .map(|(id, pem)| {
                VerifyingKey::from_public_key_pem(pem)
                    .map(|key| (id.to_owned(), key))
                    .map_err(|_| LicenseError::InvalidKey)
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        Ok(Self {
            keys: Arc::new(keys),
        })
    }
}

impl LicenseVerifier for Ed25519LicenseVerifier {
    fn verify(
        &self,
        raw_license: &str,
        identity: &CitadelInstanceIdentity,
        now: DateTime<Utc>,
    ) -> LicenseVerificationResult {
        verify_license(raw_license, identity.instance_id, now, &self.keys)
    }
}

fn verify_license(
    raw_license: &str,
    instance_id: Uuid,
    now: DateTime<Utc>,
    keys: &BTreeMap<String, VerifyingKey>,
) -> LicenseVerificationResult {
    let normalized = raw_license
        .trim_matches(|character| matches!(character, ' ' | '\t' | '\r' | '\n' | '\x0c'));
    if normalized.is_empty()
        || normalized.len() > MAX_LICENSE_BYTES
        || !normalized.is_ascii()
        || normalized.chars().any(char::is_whitespace)
    {
        return invalid("LICENSE_INVALID", "The license format is invalid.");
    }
    let mut segments = normalized.split('.');
    let (Some(header_segment), Some(payload_segment), Some(signature_segment)) =
        (segments.next(), segments.next(), segments.next())
    else {
        return invalid("LICENSE_INVALID", "The license format is invalid.");
    };
    if segments.next().is_some()
        || header_segment.is_empty()
        || payload_segment.is_empty()
        || signature_segment.is_empty()
    {
        return invalid("LICENSE_INVALID", "The license format is invalid.");
    }
    let Ok(header_bytes) = URL_SAFE_NO_PAD.decode(header_segment.as_bytes()) else {
        return invalid("LICENSE_INVALID", "The protected header is invalid.");
    };
    let Ok(header) = serde_json::from_slice::<ProtectedHeader>(&header_bytes) else {
        return invalid("LICENSE_INVALID", "The protected header is invalid.");
    };
    if header.alg != LICENSE_ALGORITHM {
        return invalid(
            "LICENSE_UNSUPPORTED_ALGORITHM",
            "The license signing algorithm is not supported.",
        );
    }
    if header.typ != LICENSE_TYPE {
        return invalid(
            "LICENSE_UNSUPPORTED_TYPE",
            "The license protected type is not supported.",
        );
    }
    if header.kid.is_empty() || header.kid.len() > 128 {
        return invalid("LICENSE_INVALID", "The license signing key ID is invalid.");
    }
    let Some(verifying_key) = keys.get(&header.kid) else {
        return failure(
            LicenseStatus::UnknownSigningKey,
            "LICENSE_UNKNOWN_SIGNING_KEY",
            "The license was signed by an unknown key.",
        );
    };
    let Ok(signature_bytes) = URL_SAFE_NO_PAD.decode(signature_segment.as_bytes()) else {
        return invalid(
            "LICENSE_INVALID_SIGNATURE",
            "The license signature is invalid.",
        );
    };
    let Ok(signature) = Signature::from_slice(&signature_bytes) else {
        return invalid(
            "LICENSE_INVALID_SIGNATURE",
            "The license signature is invalid.",
        );
    };
    if verifying_key
        .verify(
            format!("{header_segment}.{payload_segment}").as_bytes(),
            &signature,
        )
        .is_err()
    {
        return invalid(
            "LICENSE_INVALID_SIGNATURE",
            "The license signature is invalid.",
        );
    }
    let Ok(payload_bytes) = URL_SAFE_NO_PAD.decode(payload_segment.as_bytes()) else {
        return invalid("LICENSE_INVALID", "The license payload is invalid.");
    };
    let Ok(payload_json) = serde_json::from_slice::<serde_json::Value>(&payload_bytes) else {
        return invalid("LICENSE_INVALID", "The license payload is invalid.");
    };
    if json_depth(&payload_json) > MAX_JSON_DEPTH || !timestamps_are_utc(&payload_json) {
        return invalid("LICENSE_INVALID", "The license payload is invalid.");
    }
    let Ok(payload) = serde_json::from_slice::<LicensePayload>(&payload_bytes) else {
        return invalid("LICENSE_INVALID", "The license payload is invalid.");
    };
    if !matches!(
        payload.schema,
        LEGACY_LICENSE_SCHEMA | CURRENT_LICENSE_SCHEMA
    ) {
        return failure(
            LicenseStatus::UnsupportedSchema,
            "LICENSE_UNSUPPORTED_SCHEMA",
            "The license schema is not supported.",
        );
    }
    if payload.instance_id != instance_id {
        return failure(
            LicenseStatus::InstanceMismatch,
            "LICENSE_INSTANCE_MISMATCH",
            "The license belongs to a different Citadel instance.",
        );
    }
    if let Err(message) = validate_payload(&payload) {
        return invalid("LICENSE_INVALID", message);
    }

    let (known_capabilities, warnings) = effective_capabilities(&payload);
    let status = temporal_status(&payload, now);
    LicenseVerificationResult {
        status,
        license: Some(VerifiedLicense {
            raw_license: normalized.to_owned(),
            fingerprint: hex_digest(normalized.as_bytes()),
            payload,
            known_capabilities,
            warnings,
        }),
        error_code: None,
        error_message: None,
    }
}

fn validate_payload(payload: &LicensePayload) -> Result<(), &'static str> {
    let supported_edition = match payload.schema {
        LEGACY_LICENSE_SCHEMA => payload.edition == LEGACY_BUSINESS_EDITION,
        CURRENT_LICENSE_SCHEMA => matches!(payload.edition.as_str(), "Team" | "Enterprise"),
        _ => false,
    };
    if payload.product != LICENSE_PRODUCT
        || payload.issuer != LICENSE_ISSUER
        || payload.audience != LICENSE_AUDIENCE
        || !supported_edition
        || payload.license_id.trim().is_empty()
        || payload.license_id.len() > 128
        || payload
            .replaced_license_id
            .as_ref()
            .is_some_and(|id| id.trim().is_empty() || id.len() > 128 || id == &payload.license_id)
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
        return Err("The license payload is invalid.");
    }
    let mut capabilities = BTreeSet::new();
    if payload.capabilities.iter().any(|capability| {
        capability.trim().is_empty()
            || capability.len() > 128
            || !capabilities.insert(capability.as_str())
    }) {
        return Err("The license capabilities are invalid.");
    }
    if payload.schema == LEGACY_LICENSE_SCHEMA && payload.limits.values().any(|limit| *limit < 0) {
        return Err("The legacy license limits are invalid.");
    }
    Ok(())
}

pub(crate) fn temporal_status(payload: &LicensePayload, now: DateTime<Utc>) -> LicenseStatus {
    if now < payload.not_before {
        LicenseStatus::NotYetValid
    } else if now <= payload.expires_at {
        LicenseStatus::Valid
    } else if payload.grace_until.is_some_and(|grace| now <= grace) {
        LicenseStatus::GracePeriod
    } else {
        LicenseStatus::Expired
    }
}

fn effective_capabilities(payload: &LicensePayload) -> (BTreeSet<LicenseCapability>, Vec<String>) {
    if payload.schema == LEGACY_LICENSE_SCHEMA {
        return (
            LicenseCapability::ALL.iter().copied().collect(),
            vec![
                "This license uses the legacy Business schema and should be replaced at renewal."
                    .to_owned(),
            ],
        );
    }
    let mut known = BTreeSet::new();
    let mut warnings = Vec::new();
    for key in &payload.capabilities {
        if let Some(capability) = LicenseCapability::from_license_key(key) {
            known.insert(capability);
        } else {
            warnings.push(format!(
                "The signed capability '{key}' is not implemented by this Citadel version."
            ));
        }
    }
    if !payload.limits.is_empty() {
        warnings.push("Schema-2 license limits are ignored.".to_owned());
    }
    (known, warnings)
}

fn timestamps_are_utc(payload: &serde_json::Value) -> bool {
    ["issuedAt", "notBefore", "expiresAt"].iter().all(|field| {
        payload
            .get(field)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|value| value.ends_with('Z'))
    }) && payload.get("graceUntil").is_none_or(|value| {
        value.is_null()
            || value
                .as_str()
                .is_some_and(|timestamp| timestamp.ends_with('Z'))
    })
}

fn json_depth(value: &serde_json::Value) -> usize {
    match value {
        serde_json::Value::Array(values) => {
            1 + values.iter().map(json_depth).max().unwrap_or_default()
        }
        serde_json::Value::Object(values) => {
            1 + values.values().map(json_depth).max().unwrap_or_default()
        }
        _ => 1,
    }
}

fn failure(
    status: LicenseStatus,
    error_code: &'static str,
    message: impl Into<String>,
) -> LicenseVerificationResult {
    LicenseVerificationResult {
        status,
        license: None,
        error_code: Some(error_code),
        error_message: Some(message.into()),
    }
}

fn invalid(error_code: &'static str, message: impl Into<String>) -> LicenseVerificationResult {
    failure(LicenseStatus::Invalid, error_code, message)
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
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn embedded_trust_contains_only_the_production_and_current_development_keys() {
        let verifier = Ed25519LicenseVerifier::default();
        assert!(verifier.keys.contains_key("citadel-license-2026-01"));
        assert!(!verifier.keys.contains_key("citadel-license-dev-2026-01"));
        assert_eq!(verifier.keys.len(), 2);
        assert!(verifier.keys.contains_key("citadel-license-dev-2026-02"));
        assert!(!verifier.keys.contains_key("citadel-web-fixture"));
        assert!(!verifier.keys.contains_key("citadel-web-key-check"));
    }

    #[test]
    fn temporal_status_distinguishes_future_valid_grace_and_expired() {
        let now = Utc::now();
        let mut payload = payload(now);
        payload.not_before = now + chrono::Duration::minutes(1);
        assert_eq!(temporal_status(&payload, now), LicenseStatus::NotYetValid);
        payload.not_before = now - chrono::Duration::minutes(1);
        payload.expires_at = now + chrono::Duration::minutes(1);
        assert_eq!(temporal_status(&payload, now), LicenseStatus::Valid);
        payload.expires_at = now - chrono::Duration::minutes(1);
        payload.grace_until = Some(now + chrono::Duration::minutes(1));
        assert_eq!(temporal_status(&payload, now), LicenseStatus::GracePeriod);
        payload.grace_until = Some(now - chrono::Duration::seconds(1));
        assert_eq!(temporal_status(&payload, now), LicenseStatus::Expired);
    }

    #[test]
    fn schema_two_enables_only_known_explicit_capabilities() {
        let now = Utc::now();
        let mut payload = payload(now);
        payload.capabilities = vec![
            "custom-access-control".to_owned(),
            "future-capability".to_owned(),
        ];

        let (capabilities, warnings) = effective_capabilities(&payload);

        assert_eq!(
            capabilities,
            BTreeSet::from([LicenseCapability::CustomAccessControl])
        );
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn duplicate_capabilities_and_negative_legacy_limits_are_rejected() {
        let now = Utc::now();
        let mut payload = payload(now);
        payload.capabilities = vec!["custom-access-control".to_owned(); 2];
        assert!(validate_payload(&payload).is_err());
        payload.schema = LEGACY_LICENSE_SCHEMA;
        payload.edition = LEGACY_BUSINESS_EDITION.to_owned();
        payload.capabilities.clear();
        payload.limits.insert("users".to_owned(), -1);
        assert!(validate_payload(&payload).is_err());
    }

    #[test]
    fn verifier_accepts_a_valid_schema_two_license_and_rejects_tampering() {
        let now = Utc::now();
        let instance_id = Uuid::now_v7();
        let (license, keys) = signed_license(instance_id, now, "Ed25519", "test-key");

        let verified = verify_license(&license, instance_id, now, &keys);

        assert!(verified.is_accepted());
        assert_eq!(verified.status, LicenseStatus::Valid);
        assert!(
            verified
                .license
                .unwrap()
                .known_capabilities
                .contains(&LicenseCapability::CustomAccessControl)
        );

        let mut tampered = license.into_bytes();
        let payload_index = tampered.iter().position(|byte| *byte == b'.').unwrap() + 1;
        tampered[payload_index] = if tampered[payload_index] == b'A' {
            b'B'
        } else {
            b'A'
        };
        let tampered = String::from_utf8(tampered).unwrap();
        assert_eq!(
            verify_license(&tampered, instance_id, now, &keys).status,
            LicenseStatus::Invalid
        );
    }

    #[test]
    fn verifier_distinguishes_unknown_keys_algorithms_and_instances() {
        let now = Utc::now();
        let instance_id = Uuid::now_v7();
        let (unknown, keys) = signed_license(instance_id, now, "Ed25519", "unknown-key");
        assert_eq!(
            verify_license(&unknown, instance_id, now, &keys).status,
            LicenseStatus::UnknownSigningKey
        );

        let (unsupported, keys) = signed_license(instance_id, now, "EdDSA", "test-key");
        let unsupported = verify_license(&unsupported, instance_id, now, &keys);
        assert_eq!(unsupported.status, LicenseStatus::Invalid);
        assert_eq!(
            unsupported.error_code,
            Some("LICENSE_UNSUPPORTED_ALGORITHM")
        );

        let (license, keys) = signed_license(instance_id, now, "Ed25519", "test-key");
        assert_eq!(
            verify_license(&license, Uuid::now_v7(), now, &keys).status,
            LicenseStatus::InstanceMismatch
        );
    }

    fn signed_license(
        instance_id: Uuid,
        now: DateTime<Utc>,
        algorithm: &str,
        key_id: &str,
    ) -> (String, BTreeMap<String, VerifyingKey>) {
        let signing_key = SigningKey::from_bytes(&[7_u8; 32]);
        let header = serde_json::json!({
            "alg": algorithm,
            "typ": LICENSE_TYPE,
            "kid": key_id,
        });
        let payload = serde_json::json!({
            "schema": 2,
            "product": "citadel",
            "issuer": "citadel-p",
            "audience": "citadel-core",
            "licenseId": "lic-test",
            "customer": { "id": "customer", "name": "Customer" },
            "edition": "Team",
            "instanceId": instance_id,
            "issuedAt": (now - chrono::Duration::days(1)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "notBefore": (now - chrono::Duration::hours(1)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "expiresAt": (now + chrono::Duration::days(1)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "graceUntil": (now + chrono::Duration::days(2)).to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "capabilities": ["custom-access-control"]
        });
        let header = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&header).unwrap());
        let payload = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).unwrap());
        let signing_input = format!("{header}.{payload}");
        let signature = signing_key.sign(signing_input.as_bytes());
        let license = format!(
            "{signing_input}.{}",
            URL_SAFE_NO_PAD.encode(signature.to_bytes())
        );
        let keys = BTreeMap::from([("test-key".to_owned(), signing_key.verifying_key())]);
        (license, keys)
    }

    fn payload(now: DateTime<Utc>) -> LicensePayload {
        LicensePayload {
            schema: CURRENT_LICENSE_SCHEMA,
            product: LICENSE_PRODUCT.to_owned(),
            issuer: LICENSE_ISSUER.to_owned(),
            audience: LICENSE_AUDIENCE.to_owned(),
            license_id: "lic-test".to_owned(),
            replaced_license_id: None,
            customer: citadel_licensing::LicenseCustomer {
                id: "customer".to_owned(),
                name: "Customer".to_owned(),
            },
            edition: "Team".to_owned(),
            instance_id: Uuid::now_v7(),
            issued_at: now - chrono::Duration::days(1),
            not_before: now - chrono::Duration::hours(1),
            expires_at: now + chrono::Duration::days(1),
            grace_until: Some(now + chrono::Duration::days(2)),
            limits: Default::default(),
            capabilities: vec!["custom-access-control".to_owned()],
        }
    }
}
