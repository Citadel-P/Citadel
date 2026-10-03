use crate::*;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::Duration as StdDuration;
pub const LICENSE_TRANSITION_MAXIMUM_CHECK_INTERVAL: StdDuration = StdDuration::from_secs(60 * 60);

pub const LICENSE_TRANSITION_BOUNDARY_MARGIN: StdDuration = StdDuration::from_secs(1);

#[derive(Clone)]
pub struct LicenseTransitionMonitor {
    store: Arc<dyn LicenseStore>,
    verifier: Arc<dyn LicenseVerifier>,
    clock: Arc<dyn LicenseClock>,
}

impl LicenseTransitionMonitor {
    #[must_use]
    pub fn new(
        store: Arc<dyn LicenseStore>,
        verifier: Arc<dyn LicenseVerifier>,
        clock: Arc<dyn LicenseClock>,
    ) -> Self {
        Self {
            store,
            verifier,
            clock,
        }
    }

    pub async fn check_once(&self) -> Result<LicenseTransitionCheck, LicenseError> {
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
    payload: Option<&crate::LicensePayload>,
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

fn transition_activity(
    identity: &CitadelInstanceIdentity,
    installed: &InstalledLicense,
    verification: &LicenseVerificationResult,
) -> Option<LicenseChange> {
    match verification.status {
        LicenseStatus::GracePeriod => Some(LicenseChange::EnteredGracePeriod(build_state(
            identity,
            Some(installed),
            Some(verification),
        ))),
        LicenseStatus::Expired => Some(LicenseChange::Expired(build_state(
            identity,
            Some(installed),
            Some(verification),
        ))),
        LicenseStatus::Invalid
        | LicenseStatus::InstanceMismatch
        | LicenseStatus::UnsupportedSchema
        | LicenseStatus::UnknownSigningKey => Some(LicenseChange::ValidationFailed(
            Some(installed.fingerprint.clone()),
            verification.status,
            verification.error_code.map(str::to_owned),
        )),
        LicenseStatus::Community | LicenseStatus::NotYetValid | LicenseStatus::Valid => None,
    }
}
