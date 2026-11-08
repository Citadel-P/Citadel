use super::*;

impl IdentityService {
    #[must_use]
    pub fn with_setup_policy(mut self, policy: SetupPolicy) -> Self {
        self.setup_policy = policy;
        self
    }
    pub fn setup_policy(&self) -> SetupPolicy {
        self.setup_policy
    }
    pub fn setup_changes(&self) -> tokio::sync::watch::Receiver<()> {
        self.setup_changes.subscribe()
    }
    pub async fn mfa_policy(&self) -> Result<crate::MfaPolicy, IdentityError> {
        Ok(self
            .store
            .mfa_policy()
            .await?
            .at_least(self.setup_policy.minimum_mfa_policy))
    }
    pub(super) fn validate_setup_choices(
        &self,
        enabled: Option<bool>,
        policy: Option<crate::MfaPolicy>,
    ) -> Result<(), IdentityError> {
        if enabled == Some(true) && !self.setup_policy.update_checks_allowed {
            return Err(IdentityError::Validation(
                "Update checks are disabled by server configuration.".into(),
            ));
        }
        if policy.is_some_and(|value| value.at_least(self.setup_policy.minimum_mfa_policy) != value)
        {
            return Err(IdentityError::Validation(
                "The MFA policy cannot be weaker than server configuration.".into(),
            ));
        }
        Ok(())
    }
}
