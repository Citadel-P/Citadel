use std::sync::Arc;
use std::time::Duration;

use citadel_identity::{IdentityError, IdentityService};
use citadel_primitives::ActorId;
use citadel_scripts::{ScriptError, ScriptRunTokenIssuer};
use futures_util::future::BoxFuture;
use uuid::Uuid;

pub struct IdentityScriptRunTokenIssuer {
    identity: Arc<IdentityService>,
}

impl IdentityScriptRunTokenIssuer {
    #[must_use]
    pub fn new(identity: Arc<IdentityService>) -> Self {
        Self { identity }
    }
}

impl ScriptRunTokenIssuer for IdentityScriptRunTokenIssuer {
    fn issue<'a>(
        &'a self,
        run_as_actor_id: ActorId,
        run_id: Uuid,
        attempt_id: Uuid,
        lifetime: Duration,
    ) -> BoxFuture<'a, Result<String, ScriptError>> {
        Box::pin(async move {
            self.identity
                .issue_script_access_token(
                    run_as_actor_id,
                    run_id,
                    attempt_id,
                    chrono::Duration::from_std(lifetime)
                        .map_err(|error| ScriptError::External(error.to_string()))?,
                )
                .await
                .map_err(map_identity_error)
        })
    }
}

fn map_identity_error(error: IdentityError) -> ScriptError {
    match error {
        IdentityError::Validation(message) | IdentityError::Conflict(message) => {
            ScriptError::Validation(message)
        }
        IdentityError::LicenseRequired(capability) => ScriptError::Validation(format!(
            "License capability '{capability}' is required by the run-as identity."
        )),
        IdentityError::Forbidden => {
            ScriptError::Validation("The run-as identity is not authorized.".to_owned())
        }
        other => ScriptError::External(other.to_string()),
    }
}
