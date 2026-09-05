use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, RwLock};

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use chrono::{DateTime, Utc};
use citadel_application::{
    LicenseSource, LicenseStore, LicenseValidationPersistence, LicenseVerifier, build_state,
};
use citadel_deployments::{DeploymentEntitlementPort, DeploymentError};
use citadel_domain::{
    ActivityEvent, ActivityEventInfo, ActorId, CURRENT_LICENSE_SCHEMA, CitadelInstanceIdentity,
    InstalledLicense, LEGACY_BUSINESS_EDITION, LEGACY_LICENSE_SCHEMA, LICENSE_AUDIENCE,
    LICENSE_ISSUER, LICENSE_PRODUCT, LicenseCapability, LicensePayload, LicenseStatus,
    LicenseVerificationResult, VerifiedLicense,
};
use citadel_identity::{EntitlementService, IdentityError, SYSTEM_ACTOR_ID};
use ed25519_dalek::pkcs8::DecodePublicKey;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use futures_util::future::BoxFuture;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::activity_store::{insert_activity, invalid_activity};

const MAX_LICENSE_BYTES: usize = 64 * 1024;
const MAX_JSON_DEPTH: usize = 16;
const LICENSE_TYPE: &str = "citadel-license+jws";
const LICENSE_ALGORITHM: &str = "Ed25519";
const PRODUCTION_KEY: &str = r#"-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEA1lCChB1fwbhdK3a844AtpzUfCeGwn+o8+Al+OOHrCGQ=
-----END PUBLIC KEY-----"#;
const DEVELOPMENT_KEY: &str = r#"-----BEGIN PUBLIC KEY-----
MCowBQYDK2VwAyEAiKohdkN3GouALrVhfXvUHN/v4vZ4c5FNXVUHGaxxzzI=
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
            ("citadel-license-dev-2026-01", DEVELOPMENT_KEY),
        ])
        .expect("embedded Citadel license public keys are valid")
    }
}

impl Ed25519LicenseVerifier {
    pub fn from_public_key_pems<'a>(
        keys: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> Result<Self, IdentityError> {
        let keys = keys
            .into_iter()
            .map(|(id, pem)| {
                VerifyingKey::from_public_key_pem(pem)
                    .map(|key| (id.to_owned(), key))
                    .map_err(|_| IdentityError::Credential)
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

#[derive(Clone)]
pub struct PostgresLicenseStore {
    pool: PgPool,
}

impl PostgresLicenseStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl LicenseStore for PostgresLicenseStore {
    fn load(&self, now: DateTime<Utc>) -> BoxFuture<'_, Result<LicenseSource, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let identity = get_or_create_identity(&mut transaction, now).await?;
            let installed = load_installed(&mut transaction, false).await?;
            transaction.commit().await.map_err(storage)?;
            Ok(LicenseSource {
                identity,
                installed,
            })
        })
    }

    fn install<'a>(
        &'a self,
        expected_fingerprint: Option<&'a str>,
        installed: &'a InstalledLicense,
        actor_id: ActorId,
        info: &'a ActivityEventInfo,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<bool, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            lock_license_mutations(&mut transaction).await?;
            let identity = get_or_create_identity(&mut transaction, now).await?;
            let current = load_installed(&mut transaction, true).await?;
            if current.as_ref().map(|license| license.fingerprint.as_str()) != expected_fingerprint
            {
                return Ok(false);
            }
            sqlx::query(
                r#"
INSERT INTO installedlicenses (
    id, rawlicense, fingerprint, installedat, installedbyactorid,
    lastvalidatedat, lastvalidationstatus, lastvalidationerrorcode)
VALUES (1, $1, $2, $3, $4, $5, $6, $7)
ON CONFLICT (id) DO UPDATE SET
    rawlicense = EXCLUDED.rawlicense,
    fingerprint = EXCLUDED.fingerprint,
    installedat = EXCLUDED.installedat,
    installedbyactorid = EXCLUDED.installedbyactorid,
    lastvalidatedat = EXCLUDED.lastvalidatedat,
    lastvalidationstatus = EXCLUDED.lastvalidationstatus,
    lastvalidationerrorcode = EXCLUDED.lastvalidationerrorcode
"#,
            )
            .bind(&installed.raw_license)
            .bind(&installed.fingerprint)
            .bind(installed.installed_at)
            .bind(installed.installed_by_actor_id.map(ActorId::value))
            .bind(installed.last_validated_at)
            .bind(
                installed
                    .last_validation_status
                    .map(LicenseStatus::as_database_str),
            )
            .bind(&installed.last_validation_error_code)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
            insert_license_activity(&mut transaction, &identity, actor_id, info.clone(), now)
                .await?;
            transaction.commit().await.map_err(storage)?;
            Ok(true)
        })
    }

    fn remove<'a>(
        &'a self,
        expected_fingerprint: &'a str,
        actor_id: ActorId,
        info: &'a ActivityEventInfo,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<bool, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            lock_license_mutations(&mut transaction).await?;
            let identity = get_or_create_identity(&mut transaction, now).await?;
            let current = load_installed(&mut transaction, true).await?;
            if current.as_ref().map(|license| license.fingerprint.as_str())
                != Some(expected_fingerprint)
            {
                return Ok(false);
            }
            sqlx::query("DELETE FROM installedlicenses WHERE id = 1")
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            insert_license_activity(&mut transaction, &identity, actor_id, info.clone(), now)
                .await?;
            transaction.commit().await.map_err(storage)?;
            Ok(true)
        })
    }

    fn persist_validation<'a>(
        &'a self,
        expected_fingerprint: &'a str,
        status: LicenseStatus,
        error_code: Option<&'a str>,
        transition_activity: Option<&'a ActivityEventInfo>,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<LicenseValidationPersistence, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            lock_license_mutations(&mut transaction).await?;
            let identity = get_or_create_identity(&mut transaction, now).await?;
            let current = load_installed(&mut transaction, true).await?;
            let Some(current) = current else {
                return Ok(LicenseValidationPersistence::SourceChanged);
            };
            if current.fingerprint != expected_fingerprint {
                return Ok(LicenseValidationPersistence::SourceChanged);
            }
            let transitioned = current.last_validation_status != Some(status);
            sqlx::query(
                "UPDATE installedlicenses SET lastvalidatedat = $1, lastvalidationstatus = $2, lastvalidationerrorcode = $3 WHERE id = 1",
            )
            .bind(now)
            .bind(status.as_database_str())
            .bind(error_code)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
            if transitioned && let Some(info) = transition_activity {
                insert_license_activity(
                    &mut transaction,
                    &identity,
                    ActorId::new(SYSTEM_ACTOR_ID),
                    info.clone(),
                    now,
                )
                .await?;
            }
            transaction.commit().await.map_err(storage)?;
            Ok(if transitioned {
                LicenseValidationPersistence::Transitioned
            } else {
                LicenseValidationPersistence::Unchanged
            })
        })
    }
}

#[derive(Clone)]
struct CachedVerification {
    fingerprint: String,
    verification: LicenseVerificationResult,
}

pub struct PostgresLicenseEntitlementService {
    store: Arc<PostgresLicenseStore>,
    verifier: Arc<Ed25519LicenseVerifier>,
    cached: RwLock<Option<CachedVerification>>,
}

impl PostgresLicenseEntitlementService {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self::with(
            Arc::new(PostgresLicenseStore::new(pool)),
            Arc::new(Ed25519LicenseVerifier::default()),
        )
    }

    #[must_use]
    pub fn with(store: Arc<PostgresLicenseStore>, verifier: Arc<Ed25519LicenseVerifier>) -> Self {
        Self {
            store,
            verifier,
            cached: RwLock::new(None),
        }
    }

    pub fn invalidate(&self) {
        *self.cached.write().expect("license cache lock poisoned") = None;
    }

    async fn enabled(&self, capability: LicenseCapability) -> Result<bool, IdentityError> {
        let now = Utc::now();
        let source = self.store.load(now).await?;
        let Some(installed) = &source.installed else {
            return Ok(false);
        };
        let cached = self
            .cached
            .read()
            .expect("license cache lock poisoned")
            .as_ref()
            .filter(|cached| cached.fingerprint == installed.fingerprint)
            .cloned();
        let mut verification = if let Some(cached) = cached {
            cached.verification
        } else {
            let verification = self
                .verifier
                .verify(&installed.raw_license, &source.identity, now);
            *self.cached.write().expect("license cache lock poisoned") = Some(CachedVerification {
                fingerprint: installed.fingerprint.clone(),
                verification: verification.clone(),
            });
            verification
        };
        if let Some(license) = &verification.license {
            verification.status = temporal_status(&license.payload, now);
        }
        Ok(
            build_state(&source.identity, Some(installed), Some(&verification))
                .capability_enabled(capability),
        )
    }
}

impl EntitlementService for PostgresLicenseEntitlementService {
    fn custom_access_control_enabled(&self) -> BoxFuture<'_, Result<bool, IdentityError>> {
        Box::pin(async move { self.enabled(LicenseCapability::CustomAccessControl).await })
    }
}

impl DeploymentEntitlementPort for PostgresLicenseEntitlementService {
    fn enabled(
        &self,
        capability: LicenseCapability,
    ) -> BoxFuture<'_, Result<bool, DeploymentError>> {
        Box::pin(async move {
            self.enabled(capability)
                .await
                .map_err(|error| DeploymentError::Storage(error.to_string()))
        })
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

fn temporal_status(payload: &LicensePayload, now: DateTime<Utc>) -> LicenseStatus {
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

pub(crate) async fn get_or_create_identity(
    transaction: &mut Transaction<'_, Postgres>,
    now: DateTime<Utc>,
) -> Result<CitadelInstanceIdentity, IdentityError> {
    let candidate = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO citadelinstanceidentity (id, instanceid, createdat) VALUES (1, $1, $2) ON CONFLICT (id) DO NOTHING",
    )
    .bind(candidate)
    .bind(now)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;
    let row = sqlx::query(
        "SELECT instanceid, createdat FROM citadelinstanceidentity WHERE id = 1 LIMIT 1",
    )
    .fetch_one(&mut **transaction)
    .await
    .map_err(storage)?;
    Ok(CitadelInstanceIdentity {
        instance_id: row.try_get("instanceid").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
    })
}

async fn load_installed(
    transaction: &mut Transaction<'_, Postgres>,
    locked: bool,
) -> Result<Option<InstalledLicense>, IdentityError> {
    let suffix = if locked { " FOR UPDATE" } else { "" };
    let query = format!(
        "SELECT rawlicense, fingerprint, installedat, installedbyactorid, lastvalidatedat, lastvalidationstatus, lastvalidationerrorcode FROM installedlicenses WHERE id = 1{suffix}"
    );
    let row = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .fetch_optional(&mut **transaction)
        .await
        .map_err(storage)?;
    row.map(|row| {
        let last_validation_status: Option<String> =
            row.try_get("lastvalidationstatus").map_err(storage)?;
        Ok(InstalledLicense {
            raw_license: row.try_get("rawlicense").map_err(storage)?,
            fingerprint: row.try_get("fingerprint").map_err(storage)?,
            installed_at: row.try_get("installedat").map_err(storage)?,
            installed_by_actor_id: row
                .try_get::<Option<Uuid>, _>("installedbyactorid")
                .map_err(storage)?
                .map(ActorId::new),
            last_validated_at: row.try_get("lastvalidatedat").map_err(storage)?,
            last_validation_status: last_validation_status
                .as_deref()
                .and_then(LicenseStatus::from_database_str),
            last_validation_error_code: row.try_get("lastvalidationerrorcode").map_err(storage)?,
        })
    })
    .transpose()
}

async fn lock_license_mutations(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), IdentityError> {
    sqlx::query("LOCK TABLE installedlicenses IN SHARE ROW EXCLUSIVE MODE")
        .execute(&mut **transaction)
        .await
        .map_err(storage)?;
    Ok(())
}

async fn insert_license_activity(
    transaction: &mut Transaction<'_, Postgres>,
    identity: &CitadelInstanceIdentity,
    actor_id: ActorId,
    info: ActivityEventInfo,
    now: DateTime<Utc>,
) -> Result<(), IdentityError> {
    let activity = ActivityEvent::new_license_event(identity.instance_id, actor_id, info, now)
        .map_err(invalid_activity)?;
    insert_activity(transaction, &activity).await
}

fn storage(error: impl std::fmt::Display) -> IdentityError {
    IdentityError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

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
            customer: citadel_domain::LicenseCustomer {
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
