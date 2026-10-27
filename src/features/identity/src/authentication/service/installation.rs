use super::*;
use citadel_primitives::PatchField;

impl IdentityService {
    #[must_use]
    pub fn with_installation_policy(mut self, policy: InstallationPolicy) -> Self {
        self.installation_policy = policy;
        self
    }

    pub const fn installation_policy(&self) -> InstallationPolicy {
        self.installation_policy
    }

    pub fn installation_changes(&self) -> tokio::sync::watch::Receiver<()> {
        self.installation_changes.subscribe()
    }

    pub async fn installation_settings(&self) -> Result<InstallationSettings, IdentityError> {
        let mut settings = self.store.installation_settings().await?;
        settings.update_checks_enabled &= self.installation_policy.update_checks_allowed;
        settings.mfa_policy = settings
            .mfa_policy
            .at_least(self.installation_policy.minimum_mfa_policy);
        Ok(settings)
    }

    pub async fn get_installation_settings(
        &self,
        principal: &ActorPrincipal,
    ) -> Result<InstallationSettings, IdentityError> {
        if !principal.is_administrator() {
            return Err(IdentityError::Forbidden);
        }
        self.installation_settings().await
    }

    pub async fn patch_installation_settings(
        &self,
        principal: &ActorPrincipal,
        patch: PatchInstallationSettings,
    ) -> Result<InstallationSettings, IdentityError> {
        if !principal.is_administrator() {
            return Err(IdentityError::Forbidden);
        }
        let enabled = match patch.update_checks_enabled {
            PatchField::Value(value) => Some(value),
            PatchField::Missing => None,
            PatchField::Null => {
                return Err(IdentityError::Validation(
                    "updateChecksEnabled cannot be null.".into(),
                ));
            }
        };
        let policy = match patch.mfa_policy {
            PatchField::Value(value) => Some(value),
            PatchField::Missing => None,
            PatchField::Null => {
                return Err(IdentityError::Validation(
                    "mfaPolicy cannot be null.".into(),
                ));
            }
        };
        if enabled.is_none() && policy.is_none() {
            return Err(IdentityError::Validation(
                "At least one installation setting is required.".into(),
            ));
        }
        self.validate_installation_choices(enabled, policy)?;
        self.store
            .patch_installation_settings(principal, &patch, self.clock.now())
            .await?;
        self.installation_changes.send_replace(());
        self.installation_settings().await
    }

    pub(super) fn validate_installation_choices(
        &self,
        enabled: Option<bool>,
        policy: Option<crate::MfaPolicy>,
    ) -> Result<(), IdentityError> {
        if enabled == Some(true) && !self.installation_policy.update_checks_allowed {
            return Err(IdentityError::Validation(
                "Update checks are disabled by server configuration.".into(),
            ));
        }
        if policy.is_some_and(|policy| {
            policy.at_least(self.installation_policy.minimum_mfa_policy) != policy
        }) {
            return Err(IdentityError::Validation(
                "The MFA policy cannot be weaker than server configuration.".into(),
            ));
        }
        Ok(())
    }
}
