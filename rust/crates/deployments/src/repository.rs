//! Transactional durable Deployment persistence.
use crate::*;
use citadel_domain::ActorId;
use futures_util::future::BoxFuture;
use uuid::Uuid;
pub trait DeploymentRepository: Send + Sync {
    fn begin_update_check<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        expected: &'a DeploymentDetails,
    ) -> BoxFuture<'a, Result<DeploymentUpdateCheck, DeploymentError>> {
        let _ = (actor, administrator, expected);
        Box::pin(async {
            Err(DeploymentError::Runtime(
                "Image update checks are unavailable.".into(),
            ))
        })
    }
    fn complete_update_check<'a>(
        &'a self,
        claim: &'a DeploymentUpdateCheck,
        state: Option<&'a crate::AutoUpdateState>,
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        let _ = (claim, state);
        Box::pin(async {
            Err(DeploymentError::Runtime(
                "Image update checks are unavailable.".into(),
            ))
        })
    }
    fn recover_update_checks(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, DeploymentError>> {
        let _ = (started_before, limit);
        Box::pin(async { Ok(Vec::new()) })
    }
    fn scheduled_update_candidates(
        &self,
        after: Uuid,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, DeploymentError>> {
        let _ = (after, limit);
        Box::pin(async { Ok(Vec::new()) })
    }
    fn claim_apply_versioned(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        expected_version: Option<i64>,
    ) -> BoxFuture<'_, Result<ApplyClaim, DeploymentError>> {
        if expected_version.is_some() {
            return Box::pin(async {
                Err(DeploymentError::Runtime(
                    "Versioned Apply is unavailable.".into(),
                ))
            });
        }
        self.claim_apply(actor, administrator, id)
    }
    fn list_authorized<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        filter: &'a DeploymentFilter,
    ) -> BoxFuture<'a, Result<Vec<DeploymentDetails>, DeploymentError>>;

    fn get_authorized<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'a, Result<DeploymentDetails, DeploymentError>>;

    fn create<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        input: &'a CreateDeployment,
    ) -> BoxFuture<'a, Result<DeploymentDetails, DeploymentError>>;

    fn update_config<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        expected_row_version: i64,
        spec: &'a DeploymentSpec,
    ) -> BoxFuture<'a, Result<DeploymentDetails, DeploymentError>>;

    fn update_metadata<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        input: &'a UpdateDeploymentMetadata,
    ) -> BoxFuture<'a, Result<DeploymentDetails, DeploymentError>>;

    fn rename<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        name: &'a str,
    ) -> BoxFuture<'a, Result<DeploymentDetails, DeploymentError>>;

    fn duplicate_draft<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'a, Result<DeploymentDuplicateDraft, DeploymentError>>;

    fn claim_delete<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<DeletionClaim>, DeploymentError>>;

    fn complete_delete<'a>(
        &'a self,
        actor_id: ActorId,
        claims: &'a [DeletionClaim],
    ) -> BoxFuture<'a, Result<(), DeploymentError>>;

    fn release_delete<'a>(
        &'a self,
        claims: &'a [DeletionClaim],
    ) -> BoxFuture<'a, Result<(), DeploymentError>>;

    fn claim_apply<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'a, Result<ApplyClaim, DeploymentError>> {
        let _ = (actor_id, administrator, id);
        Box::pin(async {
            Err(DeploymentError::Runtime(
                "Deployment Apply persistence is unavailable.".to_owned(),
            ))
        })
    }

    fn complete_apply<'a>(
        &'a self,
        actor_id: ActorId,
        claim: &'a ApplyClaim,
        result: &'a RuntimeDeploymentResult,
        digest: Option<&'a str>,
        bindings: &'a [DeploymentBindingSnapshot],
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        let _ = (actor_id, claim, result, digest, bindings);
        Box::pin(async {
            Err(DeploymentError::Runtime(
                "Deployment Apply persistence is unavailable.".to_owned(),
            ))
        })
    }

    fn fail_apply<'a>(
        &'a self,
        actor_id: ActorId,
        claim: &'a ApplyClaim,
        message: &'a str,
        result: Option<&'a RuntimeDeploymentResult>,
        bindings: &'a [DeploymentBindingSnapshot],
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        let _ = (actor_id, claim, message, result, bindings);
        Box::pin(async {
            Err(DeploymentError::Runtime(
                "Deployment Apply persistence is unavailable.".to_owned(),
            ))
        })
    }

    fn stale_apply_claims<'a>(
        &'a self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'a, Result<Vec<(ActorId, ApplyClaim)>, DeploymentError>> {
        let _ = (started_before, limit);
        Box::pin(async { Ok(Vec::new()) })
    }
}
