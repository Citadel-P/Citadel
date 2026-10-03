use crate::*;
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

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use std::collections::BTreeSet;
    use std::time::Duration as StdDuration;
    use uuid::Uuid;

    use crate::{CURRENT_LICENSE_SCHEMA, LicenseCustomer, LicensePayload, VerifiedLicense};

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
        assert_eq!(crate::LEGACY_BUSINESS_EDITION, "Business");
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
