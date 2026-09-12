use std::sync::Arc;
use std::time::Duration as StdDuration;

use chrono::{DateTime, Utc};
use citadel_domain::{
    ActivityEventInfo, ActorId, COMMUNITY_EDITION, CitadelInstanceIdentity, InstalledLicense,
    LEGACY_LICENSE_SCHEMA, LICENSE_PRODUCT, LicenseCapability, LicenseState, LicenseStatus,
    LicenseVerificationResult, TEAM_EDITION,
};
use citadel_identity::{Clock, IdentityError};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const LICENSE_REPLACEMENT_MISMATCH: &str =
    "https://citadel.local/problems/license-replacement-mismatch";
pub const LICENSE_REPLACEMENT_NOT_YET_EFFECTIVE: &str =
    "https://citadel.local/problems/license-replacement-not-yet-effective";

pub trait LicenseVerifier: Send + Sync {
    fn verify(
        &self,
        raw_license: &str,
        identity: &CitadelInstanceIdentity,
        now: DateTime<Utc>,
    ) -> LicenseVerificationResult;
}

pub trait LicenseStateNotifier: Send + Sync {
    fn license_state_changed(&self, instance_id: Uuid);
}

#[derive(Debug, Default)]
struct NoopLicenseStateNotifier;

impl LicenseStateNotifier for NoopLicenseStateNotifier {
    fn license_state_changed(&self, _instance_id: Uuid) {}
}

#[derive(Debug, Clone)]
pub struct LicenseSource {
    pub identity: CitadelInstanceIdentity,
    pub installed: Option<InstalledLicense>,
}

pub trait LicenseStore: Send + Sync {
    fn load(&self, now: DateTime<Utc>) -> BoxFuture<'_, Result<LicenseSource, IdentityError>>;

    fn install<'a>(
        &'a self,
        expected_fingerprint: Option<&'a str>,
        installed: &'a InstalledLicense,
        actor_id: ActorId,
        info: &'a ActivityEventInfo,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<bool, IdentityError>>;

    fn remove<'a>(
        &'a self,
        expected_fingerprint: &'a str,
        actor_id: ActorId,
        info: &'a ActivityEventInfo,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<bool, IdentityError>>;

    fn persist_validation<'a>(
        &'a self,
        expected_fingerprint: &'a str,
        status: LicenseStatus,
        error_code: Option<&'a str>,
        transition_activity: Option<&'a ActivityEventInfo>,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<LicenseValidationPersistence, IdentityError>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LicenseValidationPersistence {
    SourceChanged,
    Unchanged,
    Transitioned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LicenseTransitionCheck {
    pub persistence: LicenseValidationPersistence,
    pub instance_id: Option<Uuid>,
    pub status: Option<LicenseStatus>,
    pub next_boundary: Option<DateTime<Utc>>,
}

pub const LICENSE_TRANSITION_MAXIMUM_CHECK_INTERVAL: StdDuration = StdDuration::from_secs(60 * 60);
pub const LICENSE_TRANSITION_BOUNDARY_MARGIN: StdDuration = StdDuration::from_secs(1);

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallLicenseRequest {
    pub license: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LicenseCapabilityView {
    pub capability: LicenseCapability,
    pub enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LicenseView {
    pub status: LicenseStatus,
    pub effective_edition: String,
    pub licensed_edition: Option<String>,
    pub instance_id: Uuid,
    pub license_schema: Option<u8>,
    pub license_id: Option<String>,
    pub replaced_license_id: Option<String>,
    pub customer_id: Option<String>,
    pub customer_name: Option<String>,
    pub fingerprint: Option<String>,
    pub issued_at: Option<DateTime<Utc>>,
    pub not_before: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub grace_until: Option<DateTime<Utc>>,
    pub capabilities: Vec<LicenseCapabilityView>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LicenseEntitlementsView {
    pub status: LicenseStatus,
    pub effective_edition: String,
    pub capabilities: Vec<LicenseCapabilityView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LicenseRequestView {
    pub product: &'static str,
    pub instance_id: Uuid,
    pub core_version: String,
    pub generated_at: DateTime<Utc>,
}

#[derive(Clone)]
pub struct LicenseService {
    store: Arc<dyn LicenseStore>,
    verifier: Arc<dyn LicenseVerifier>,
    clock: Arc<dyn Clock>,
    notifier: Arc<dyn LicenseStateNotifier>,
    core_version: String,
}

impl LicenseService {
    #[must_use]
    pub fn new(
        store: Arc<dyn LicenseStore>,
        verifier: Arc<dyn LicenseVerifier>,
        clock: Arc<dyn Clock>,
        core_version: String,
    ) -> Self {
        Self {
            store,
            verifier,
            clock,
            notifier: Arc::new(NoopLicenseStateNotifier),
            core_version,
        }
    }

    #[must_use]
    pub fn with_notifier(mut self, notifier: Arc<dyn LicenseStateNotifier>) -> Self {
        self.notifier = notifier;
        self
    }

    pub async fn get(&self) -> Result<LicenseView, IdentityError> {
        Ok(to_view(&self.current_state().await?))
    }

    pub async fn entitlements(&self) -> Result<LicenseEntitlementsView, IdentityError> {
        let state = self.current_state().await?;
        Ok(LicenseEntitlementsView {
            status: state.status,
            effective_edition: state.effective_edition.clone(),
            capabilities: capability_views(&state),
        })
    }

    pub async fn request(&self) -> Result<LicenseRequestView, IdentityError> {
        let now = self.clock.now();
        let source = self.store.load(now).await?;
        Ok(LicenseRequestView {
            product: LICENSE_PRODUCT,
            instance_id: source.identity.instance_id,
            core_version: self.core_version.clone(),
            generated_at: now,
        })
    }

    pub async fn install(
        &self,
        raw_license: &str,
        actor_id: ActorId,
    ) -> Result<LicenseView, IdentityError> {
        for _ in 0..3 {
            let now = self.clock.now();
            let source = self.store.load(now).await?;
            let verification = self.verifier.verify(raw_license, &source.identity, now);
            if !verification.is_accepted() {
                return Err(IdentityError::Validation(
                    verification
                        .error_message
                        .unwrap_or_else(|| "License is not valid.".to_owned()),
                ));
            }
            let verified = verification
                .license
                .as_ref()
                .expect("accepted verification contains a license");

            let existing_verification = source.installed.as_ref().map(|installed| {
                self.verifier
                    .verify(&installed.raw_license, &source.identity, now)
            });
            if source.installed.is_some() {
                let existing_id = existing_verification
                    .as_ref()
                    .and_then(|result| result.license.as_ref())
                    .map(|license| license.payload.license_id.as_str());
                if existing_id.is_none()
                    || verified.payload.replaced_license_id.as_deref() != existing_id
                {
                    return Err(IdentityError::TypedConflict {
                        problem_type: LICENSE_REPLACEMENT_MISMATCH,
                        message: "The submitted license does not replace the currently installed license."
                            .to_owned(),
                    });
                }
                if verification.status == LicenseStatus::NotYetValid
                    && existing_verification.as_ref().is_some_and(|result| {
                        matches!(
                            result.status,
                            LicenseStatus::Valid | LicenseStatus::GracePeriod
                        )
                    })
                {
                    return Err(IdentityError::TypedConflict {
                        problem_type: LICENSE_REPLACEMENT_NOT_YET_EFFECTIVE,
                        message:
                            "A future-dated license cannot replace a currently active license."
                                .to_owned(),
                    });
                }
            }

            let new_state = build_state(
                &source.identity,
                Some(&InstalledLicense {
                    raw_license: verified.raw_license.clone(),
                    fingerprint: verified.fingerprint.clone(),
                    installed_at: now,
                    installed_by_actor_id: Some(actor_id),
                    last_validated_at: Some(now),
                    last_validation_status: Some(verification.status),
                    last_validation_error_code: None,
                }),
                Some(&verification),
            );
            let new_snapshot = activity_snapshot(&new_state);
            let info = if let (Some(installed), Some(old_verification)) =
                (&source.installed, &existing_verification)
            {
                let old_state =
                    build_state(&source.identity, Some(installed), Some(old_verification));
                ActivityEventInfo::license_replaced(activity_snapshot(&old_state), new_snapshot)
            } else {
                ActivityEventInfo::license_installed(new_snapshot)
            };
            let installed = InstalledLicense {
                raw_license: verified.raw_license.clone(),
                fingerprint: verified.fingerprint.clone(),
                installed_at: now,
                installed_by_actor_id: Some(actor_id),
                last_validated_at: Some(now),
                last_validation_status: Some(verification.status),
                last_validation_error_code: None,
            };
            if self
                .store
                .install(
                    source
                        .installed
                        .as_ref()
                        .map(|license| license.fingerprint.as_str()),
                    &installed,
                    actor_id,
                    &info,
                    now,
                )
                .await?
            {
                self.notifier
                    .license_state_changed(source.identity.instance_id);
                return Ok(to_view(&new_state));
            }
        }
        Err(IdentityError::Conflict(
            "The installed license changed concurrently. Retry the operation.".to_owned(),
        ))
    }

    pub async fn remove(&self, actor_id: ActorId) -> Result<LicenseView, IdentityError> {
        for _ in 0..3 {
            let now = self.clock.now();
            let source = self.store.load(now).await?;
            let Some(installed) = &source.installed else {
                return Ok(to_view(&LicenseState::community(
                    source.identity.instance_id,
                )));
            };
            let verification = self
                .verifier
                .verify(&installed.raw_license, &source.identity, now);
            let state = build_state(&source.identity, Some(installed), Some(&verification));
            let info = ActivityEventInfo::license_removed(activity_snapshot(&state));
            if self
                .store
                .remove(&installed.fingerprint, actor_id, &info, now)
                .await?
            {
                self.notifier
                    .license_state_changed(source.identity.instance_id);
                return Ok(to_view(&LicenseState::community(
                    source.identity.instance_id,
                )));
            }
        }
        Err(IdentityError::Conflict(
            "The installed license changed concurrently. Retry the operation.".to_owned(),
        ))
    }

    pub async fn current_state(&self) -> Result<LicenseState, IdentityError> {
        let now = self.clock.now();
        let source = self.store.load(now).await?;
        let verification = source.installed.as_ref().map(|installed| {
            self.verifier
                .verify(&installed.raw_license, &source.identity, now)
        });
        Ok(build_state(
            &source.identity,
            source.installed.as_ref(),
            verification.as_ref(),
        ))
    }
}

#[derive(Clone)]
pub struct LicenseTransitionMonitor {
    store: Arc<dyn LicenseStore>,
    verifier: Arc<dyn LicenseVerifier>,
    clock: Arc<dyn Clock>,
}

impl LicenseTransitionMonitor {
    #[must_use]
    pub fn new(
        store: Arc<dyn LicenseStore>,
        verifier: Arc<dyn LicenseVerifier>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            store,
            verifier,
            clock,
        }
    }

    pub async fn check_once(&self) -> Result<LicenseTransitionCheck, IdentityError> {
        let now = self.clock.now();
        let source = self.store.load(now).await?;
        let Some(installed) = &source.installed else {
            return Ok(LicenseTransitionCheck {
                persistence: LicenseValidationPersistence::Unchanged,
                instance_id: Some(source.identity.instance_id),
                status: None,
                next_boundary: None,
            });
        };
        let verification = self
            .verifier
            .verify(&installed.raw_license, &source.identity, now);
        let transition_activity = transition_activity(&source.identity, installed, &verification);
        let persistence = self
            .store
            .persist_validation(
                &installed.fingerprint,
                verification.status,
                verification.error_code,
                transition_activity.as_ref(),
                now,
            )
            .await?;
        let next_boundary = if persistence == LicenseValidationPersistence::SourceChanged {
            Some(now)
        } else {
            next_license_boundary(
                verification
                    .license
                    .as_ref()
                    .map(|license| &license.payload),
                now,
            )
        };
        Ok(LicenseTransitionCheck {
            persistence,
            instance_id: Some(source.identity.instance_id),
            status: Some(verification.status),
            next_boundary,
        })
    }
}

#[must_use]
pub fn license_transition_delay(
    now: DateTime<Utc>,
    next_boundary: Option<DateTime<Utc>>,
) -> StdDuration {
    let Some(next_boundary) = next_boundary else {
        return LICENSE_TRANSITION_MAXIMUM_CHECK_INTERVAL;
    };
    let boundary_delay = if next_boundary > now {
        (next_boundary - now)
            .to_std()
            .unwrap_or_default()
            .saturating_add(LICENSE_TRANSITION_BOUNDARY_MARGIN)
    } else {
        LICENSE_TRANSITION_BOUNDARY_MARGIN
    };
    boundary_delay.min(LICENSE_TRANSITION_MAXIMUM_CHECK_INTERVAL)
}

#[must_use]
pub fn next_license_boundary(
    payload: Option<&citadel_domain::LicensePayload>,
    now: DateTime<Utc>,
) -> Option<DateTime<Utc>> {
    let payload = payload?;
    if now < payload.not_before {
        Some(payload.not_before)
    } else if now <= payload.expires_at {
        Some(payload.expires_at)
    } else {
        payload
            .grace_until
            .filter(|grace_until| now <= *grace_until)
    }
}

#[must_use]
pub fn build_state(
    identity: &CitadelInstanceIdentity,
    installed: Option<&InstalledLicense>,
    verification: Option<&LicenseVerificationResult>,
) -> LicenseState {
    let Some(installed) = installed else {
        return LicenseState::community(identity.instance_id);
    };
    let Some(verification) = verification else {
        return LicenseState::community(identity.instance_id);
    };
    let Some(license) = &verification.license else {
        let mut state = LicenseState::community(identity.instance_id);
        state.status = verification.status;
        state.fingerprint = Some(installed.fingerprint.clone());
        if let Some(message) = &verification.error_message {
            state.warnings.push(message.clone());
        }
        return state;
    };
    let effective = matches!(
        verification.status,
        LicenseStatus::Valid | LicenseStatus::GracePeriod
    );
    LicenseState {
        status: verification.status,
        effective_edition: if effective {
            if license.payload.schema == LEGACY_LICENSE_SCHEMA {
                TEAM_EDITION.to_owned()
            } else {
                license.payload.edition.clone()
            }
        } else {
            COMMUNITY_EDITION.to_owned()
        },
        licensed_edition: Some(license.payload.edition.clone()),
        instance_id: identity.instance_id,
        license_schema: Some(license.payload.schema),
        license_id: Some(license.payload.license_id.clone()),
        replaced_license_id: license.payload.replaced_license_id.clone(),
        customer_id: Some(license.payload.customer.id.clone()),
        customer_name: Some(license.payload.customer.name.clone()),
        fingerprint: Some(license.fingerprint.clone()),
        issued_at: Some(license.payload.issued_at),
        not_before: Some(license.payload.not_before),
        expires_at: Some(license.payload.expires_at),
        grace_until: license.payload.grace_until,
        effective_capabilities: if effective {
            license.known_capabilities.clone()
        } else {
            Default::default()
        },
        warnings: license.warnings.clone(),
    }
}

fn activity_snapshot(state: &LicenseState) -> citadel_domain::LicenseActivitySnapshot {
    citadel_domain::LicenseActivitySnapshot {
        schema: state.license_schema,
        license_id: state.license_id.clone(),
        replaced_license_id: state.replaced_license_id.clone(),
        licensed_edition: state.licensed_edition.clone(),
        effective_edition: state.effective_edition.clone(),
        effective_capabilities: state.effective_capabilities.iter().copied().collect(),
        customer_id: state.customer_id.clone(),
        customer_name: state.customer_name.clone(),
        fingerprint: state.fingerprint.clone(),
        status: state.status,
        expires_at: state.expires_at,
        grace_until: state.grace_until,
    }
}

fn transition_activity(
    identity: &CitadelInstanceIdentity,
    installed: &InstalledLicense,
    verification: &LicenseVerificationResult,
) -> Option<ActivityEventInfo> {
    match verification.status {
        LicenseStatus::GracePeriod => Some(ActivityEventInfo::license_entered_grace_period(
            activity_snapshot(&build_state(identity, Some(installed), Some(verification))),
        )),
        LicenseStatus::Expired => Some(ActivityEventInfo::license_expired(activity_snapshot(
            &build_state(identity, Some(installed), Some(verification)),
        ))),
        LicenseStatus::Invalid
        | LicenseStatus::InstanceMismatch
        | LicenseStatus::UnsupportedSchema
        | LicenseStatus::UnknownSigningKey => Some(ActivityEventInfo::license_validation_failed(
            Some(installed.fingerprint.clone()),
            verification.status,
            verification.error_code.map(str::to_owned),
        )),
        LicenseStatus::Community | LicenseStatus::NotYetValid | LicenseStatus::Valid => None,
    }
}

fn capability_views(state: &LicenseState) -> Vec<LicenseCapabilityView> {
    LicenseCapability::ALL
        .iter()
        .copied()
        .map(|capability| LicenseCapabilityView {
            capability,
            enabled: state.capability_enabled(capability),
        })
        .collect()
}

fn to_view(state: &LicenseState) -> LicenseView {
    LicenseView {
        status: state.status,
        effective_edition: state.effective_edition.clone(),
        licensed_edition: state.licensed_edition.clone(),
        instance_id: state.instance_id,
        license_schema: state.license_schema,
        license_id: state.license_id.clone(),
        replaced_license_id: state.replaced_license_id.clone(),
        customer_id: state.customer_id.clone(),
        customer_name: state.customer_name.clone(),
        fingerprint: state.fingerprint.clone(),
        issued_at: state.issued_at,
        not_before: state.not_before,
        expires_at: state.expires_at,
        grace_until: state.grace_until,
        capabilities: capability_views(state),
        warnings: state.warnings.clone(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use citadel_domain::{
        CURRENT_LICENSE_SCHEMA, LicenseCustomer, LicensePayload, VerifiedLicense,
    };

    use super::*;

    #[test]
    fn inactive_license_preserves_metadata_but_disables_capabilities() {
        let instance_id = Uuid::now_v7();
        let now = Utc::now();
        let identity = CitadelInstanceIdentity {
            instance_id,
            created_at: now,
        };
        let installed = InstalledLicense {
            raw_license: "header.payload.signature".to_owned(),
            fingerprint: "fingerprint".to_owned(),
            installed_at: now,
            installed_by_actor_id: None,
            last_validated_at: Some(now),
            last_validation_status: Some(LicenseStatus::Expired),
            last_validation_error_code: None,
        };
        let verification = LicenseVerificationResult {
            status: LicenseStatus::Expired,
            license: Some(VerifiedLicense {
                raw_license: installed.raw_license.clone(),
                fingerprint: installed.fingerprint.clone(),
                payload: LicensePayload {
                    schema: CURRENT_LICENSE_SCHEMA,
                    product: LICENSE_PRODUCT.to_owned(),
                    issuer: "citadel-p".to_owned(),
                    audience: "citadel-core".to_owned(),
                    license_id: "lic-test".to_owned(),
                    replaced_license_id: None,
                    customer: LicenseCustomer {
                        id: "customer".to_owned(),
                        name: "Customer".to_owned(),
                    },
                    edition: TEAM_EDITION.to_owned(),
                    instance_id,
                    issued_at: now,
                    not_before: now,
                    expires_at: now,
                    grace_until: None,
                    limits: Default::default(),
                    capabilities: vec!["custom-access-control".to_owned()],
                },
                known_capabilities: BTreeSet::from([LicenseCapability::CustomAccessControl]),
                warnings: Vec::new(),
            }),
            error_code: None,
            error_message: None,
        };

        let state = build_state(&identity, Some(&installed), Some(&verification));

        assert_eq!(state.status, LicenseStatus::Expired);
        assert_eq!(state.licensed_edition.as_deref(), Some(TEAM_EDITION));
        assert_eq!(state.effective_edition, COMMUNITY_EDITION);
        assert!(state.effective_capabilities.is_empty());
    }

    #[test]
    fn legacy_business_maps_to_team_and_every_shipped_capability() {
        assert_eq!(citadel_domain::LEGACY_BUSINESS_EDITION, "Business");
        assert_eq!(LicenseCapability::ALL.len(), 5);
    }

    #[test]
    fn transition_delay_is_bounded_and_checks_just_after_the_boundary() {
        let now = Utc::now();
        assert_eq!(
            license_transition_delay(now, None),
            LICENSE_TRANSITION_MAXIMUM_CHECK_INTERVAL
        );
        assert_eq!(
            license_transition_delay(now, Some(now + chrono::Duration::hours(2))),
            LICENSE_TRANSITION_MAXIMUM_CHECK_INTERVAL
        );
        assert_eq!(
            license_transition_delay(now, Some(now + chrono::Duration::minutes(5))),
            StdDuration::from_secs(301)
        );
        assert_eq!(
            license_transition_delay(now, Some(now)),
            LICENSE_TRANSITION_BOUNDARY_MARGIN
        );
    }

    #[test]
    fn next_boundary_follows_not_before_expiry_and_grace_in_order() {
        let now = Utc::now();
        let mut payload = LicensePayload {
            schema: CURRENT_LICENSE_SCHEMA,
            product: LICENSE_PRODUCT.to_owned(),
            issuer: "citadel-p".to_owned(),
            audience: "citadel-core".to_owned(),
            license_id: "lic-boundary".to_owned(),
            replaced_license_id: None,
            customer: LicenseCustomer {
                id: "customer".to_owned(),
                name: "Customer".to_owned(),
            },
            edition: TEAM_EDITION.to_owned(),
            instance_id: Uuid::now_v7(),
            issued_at: now - chrono::Duration::days(2),
            not_before: now + chrono::Duration::minutes(1),
            expires_at: now + chrono::Duration::days(1),
            grace_until: Some(now + chrono::Duration::days(2)),
            limits: Default::default(),
            capabilities: Vec::new(),
        };
        assert_eq!(
            next_license_boundary(Some(&payload), now),
            Some(payload.not_before)
        );
        payload.not_before = now - chrono::Duration::minutes(1);
        assert_eq!(
            next_license_boundary(Some(&payload), now),
            Some(payload.expires_at)
        );
        payload.expires_at = now - chrono::Duration::minutes(1);
        assert_eq!(
            next_license_boundary(Some(&payload), now),
            payload.grace_until
        );
        payload.grace_until = Some(now - chrono::Duration::seconds(1));
        assert_eq!(next_license_boundary(Some(&payload), now), None);
    }
}
