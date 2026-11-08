use crate::{
    InstallationPolicy, InstallationSettings, InstanceError, InstanceRepository,
    PatchInstallationSettings,
};
use citadel_identity::{ActorPrincipal, Clock};
use citadel_primitives::PatchField;
use std::sync::Arc;

pub struct InstanceService {
    repository: Arc<dyn InstanceRepository>,
    clock: Arc<dyn Clock>,
    installation_policy: InstallationPolicy,
    installation_changes: tokio::sync::watch::Sender<()>,
}
impl InstanceService {
    #[must_use]
    pub fn new(repository: Arc<dyn InstanceRepository>, clock: Arc<dyn Clock>) -> Self {
        Self {
            repository,
            clock,
            installation_policy: Default::default(),
            installation_changes: tokio::sync::watch::channel(()).0,
        }
    }

    #[must_use]
    pub fn with_installation_policy(mut self, policy: InstallationPolicy) -> Self {
        self.installation_policy = policy;
        self
    }

    pub fn installation_policy(&self) -> InstallationPolicy {
        self.installation_policy.clone()
    }

    pub fn installation_changes(&self) -> tokio::sync::watch::Receiver<()> {
        self.installation_changes.subscribe()
    }

    pub async fn installation_settings(&self) -> Result<InstallationSettings, InstanceError> {
        let mut settings = self.repository.installation_settings().await?;
        settings.update_checks_enabled &= self.installation_policy.update_checks_allowed;
        settings.mfa_policy = settings
            .mfa_policy
            .at_least(self.installation_policy.minimum_mfa_policy);
        Ok(settings)
    }

    pub async fn get_installation_settings(
        &self,
        principal: &ActorPrincipal,
    ) -> Result<InstallationSettings, InstanceError> {
        if !principal.is_administrator() {
            return Err(InstanceError::Forbidden);
        }
        self.installation_settings().await
    }

    pub async fn patch_installation_settings(
        &self,
        principal: &ActorPrincipal,
        mut patch: PatchInstallationSettings,
    ) -> Result<InstallationSettings, InstanceError> {
        if !principal.is_administrator() {
            return Err(InstanceError::Forbidden);
        }
        let enabled = match patch.update_checks_enabled {
            PatchField::Value(value) => Some(value),
            PatchField::Missing => None,
            PatchField::Null => {
                return Err(InstanceError::Validation(
                    "updateChecksEnabled cannot be null.".into(),
                ));
            }
        };
        let policy = match patch.mfa_policy {
            PatchField::Value(value) => Some(value),
            PatchField::Missing => None,
            PatchField::Null => {
                return Err(InstanceError::Validation(
                    "mfaPolicy cannot be null.".into(),
                ));
            }
        };
        let interval = match patch.update_check_interval_hours {
            PatchField::Value(value @ (6 | 12 | 24)) => Some(value),
            PatchField::Missing => None,
            _ => {
                return Err(InstanceError::Validation(
                    "Update check frequency must be 6, 12 or 24 hours.".into(),
                ));
            }
        };
        if matches!(&patch.configuration, PatchField::Value(value) if value.is_empty()) {
            return Err(InstanceError::Validation(
                "At least one configuration setting is required.".into(),
            ));
        }
        if matches!(patch.configuration, PatchField::Null) {
            return Err(InstanceError::Validation(
                "configuration cannot be null.".into(),
            ));
        }
        if enabled.is_none()
            && policy.is_none()
            && interval.is_none()
            && matches!(patch.configuration, PatchField::Missing)
        {
            return Err(InstanceError::Validation(
                "At least one installation setting is required.".into(),
            ));
        }
        self.validate_installation_choices(enabled, policy)?;
        patch.deployment_overrides = self.installation_policy.overrides.clone();
        self.repository
            .patch_installation_settings(principal, &patch, self.clock.now())
            .await?;
        self.installation_changes.send_replace(());
        self.installation_settings().await
    }

    fn validate_installation_choices(
        &self,
        enabled: Option<bool>,
        policy: Option<citadel_identity::MfaPolicy>,
    ) -> Result<(), InstanceError> {
        if enabled == Some(true) && !self.installation_policy.update_checks_allowed {
            return Err(InstanceError::Validation(
                "Update checks are disabled by server configuration.".into(),
            ));
        }
        if policy.is_some_and(|policy| {
            policy.at_least(self.installation_policy.minimum_mfa_policy) != policy
        }) {
            return Err(InstanceError::Validation(
                "The MFA policy cannot be weaker than server configuration.".into(),
            ));
        }
        Ok(())
    }
}
