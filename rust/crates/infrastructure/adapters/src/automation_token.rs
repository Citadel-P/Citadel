use std::sync::Arc;
use std::time::Duration;

use citadel_automation::{AutomationError, AutomationRunTokenIssuer};
use citadel_identity::{IdentityError, IdentityService};
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use uuid::Uuid;

pub struct IdentityAutomationRunTokenIssuer {
    identity: Arc<IdentityService>,
}

impl IdentityAutomationRunTokenIssuer {
    #[must_use]
    pub fn new(identity: Arc<IdentityService>) -> Self {
        Self { identity }
    }
}

impl AutomationRunTokenIssuer for IdentityAutomationRunTokenIssuer {
    fn issue<'a>(
        &'a self,
        run_as_actor_id: ActorId,
        run_id: Uuid,
        lifetime: Duration,
    ) -> BoxFuture<'a, Result<String, AutomationError>> {
        Box::pin(async move {
            self.identity
                .issue_automation_access_token(
                    run_as_actor_id,
                    run_id,
                    chrono::Duration::from_std(lifetime)
                        .map_err(|error| AutomationError::External(error.to_string()))?,
                )
                .await
                .map_err(map_identity_error)
        })
    }
}

fn map_identity_error(error: IdentityError) -> AutomationError {
    match error {
        IdentityError::Validation(message) | IdentityError::Conflict(message) => {
            AutomationError::Validation(message)
        }
        IdentityError::LicenseRequired(capability) => AutomationError::Validation(format!(
            "License capability '{capability}' is required by the run-as identity."
        )),
        IdentityError::Forbidden => {
            AutomationError::Validation("The run-as identity is not authorized.".to_owned())
        }
        other => AutomationError::External(other.to_string()),
    }
}
