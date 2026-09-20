use crate::runtime::NoopLicenseStateNotifier;
use crate::*;
use citadel_primitives::ActorId;
use std::sync::Arc;
#[derive(Clone)]
pub struct LicenseService {
    store: Arc<dyn LicenseStore>,
    verifier: Arc<dyn LicenseVerifier>,
    clock: Arc<dyn LicenseClock>,
    notifier: Arc<dyn LicenseStateNotifier>,
    core_version: String,
}

impl LicenseService {
    #[must_use]
    pub fn new(
        store: Arc<dyn LicenseStore>,
        verifier: Arc<dyn LicenseVerifier>,
        clock: Arc<dyn LicenseClock>,
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

    pub async fn get(&self) -> Result<LicenseState, LicenseError> {
        self.current_state().await
    }

    pub async fn entitlements(&self) -> Result<LicenseState, LicenseError> {
        self.current_state().await
    }

    pub async fn request(&self) -> Result<LicenseRequest, LicenseError> {
        let now = self.clock.now();
        let source = self.store.load(now).await?;
        Ok(LicenseRequest {
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
    ) -> Result<LicenseState, LicenseError> {
        for _ in 0..3 {
            let now = self.clock.now();
            let source = self.store.load(now).await?;
            let verification = self.verifier.verify(raw_license, &source.identity, now);
            if !verification.is_accepted() {
                return Err(LicenseError::Validation(
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
                    return Err(LicenseError::ReplacementMismatch);
                }
                if verification.status == LicenseStatus::NotYetValid
                    && existing_verification.as_ref().is_some_and(|result| {
                        matches!(
                            result.status,
                            LicenseStatus::Valid | LicenseStatus::GracePeriod
                        )
                    })
                {
                    return Err(LicenseError::ReplacementNotYetEffective);
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
            let new_snapshot = new_state.clone();
            let info = if let (Some(installed), Some(old_verification)) =
                (&source.installed, &existing_verification)
            {
                let old_state =
                    build_state(&source.identity, Some(installed), Some(old_verification));
                LicenseChange::Replaced(Box::new(old_state), new_snapshot)
            } else {
                LicenseChange::Installed(new_snapshot)
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
                return Ok(new_state);
            }
        }
        Err(LicenseError::Conflict(
            "The installed license changed concurrently. Retry the operation.".to_owned(),
        ))
    }

    pub async fn remove(&self, actor_id: ActorId) -> Result<LicenseState, LicenseError> {
        for _ in 0..3 {
            let now = self.clock.now();
            let source = self.store.load(now).await?;
            let Some(installed) = &source.installed else {
                return Ok(LicenseState::community(source.identity.instance_id));
            };
            let verification = self
                .verifier
                .verify(&installed.raw_license, &source.identity, now);
            let state = build_state(&source.identity, Some(installed), Some(&verification));
            let info = LicenseChange::Removed(state.clone());
            if self
                .store
                .remove(&installed.fingerprint, actor_id, &info, now)
                .await?
            {
                self.notifier
                    .license_state_changed(source.identity.instance_id);
                return Ok(LicenseState::community(source.identity.instance_id));
            }
        }
        Err(LicenseError::Conflict(
            "The installed license changed concurrently. Retry the operation.".to_owned(),
        ))
    }

    pub async fn current_state(&self) -> Result<LicenseState, LicenseError> {
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
