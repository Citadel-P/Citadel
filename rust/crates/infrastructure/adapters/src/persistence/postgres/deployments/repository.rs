use super::*;
use citadel_primitives::AuthorizedResource;
#[derive(Clone)]
pub struct PostgresDeploymentRepository {
    pub(super) pool: PgPool,
}

impl PostgresDeploymentRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl DeploymentRepository for PostgresDeploymentRepository {
    fn begin_update_check<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        expected: &'a citadel_deployments::Deployment,
    ) -> BoxFuture<'a, Result<citadel_deployments::DeploymentUpdateCheck, DeploymentError>> {
        self.begin_update_check_impl(actor, administrator, expected)
    }
    fn complete_update_check<'a>(
        &'a self,
        claim: &'a citadel_deployments::DeploymentUpdateCheck,
        state: Option<&'a AutoUpdateState>,
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        self.complete_update_check_impl(claim, state)
    }
    fn recover_update_checks(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, DeploymentError>> {
        self.recover_update_checks_impl(started_before, limit)
    }
    fn scheduled_update_candidates(
        &self,
        after: Uuid,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, DeploymentError>> {
        self.scheduled_update_candidates_impl(after, limit)
    }
    fn list_authorized<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        filter: &'a DeploymentFilter,
    ) -> BoxFuture<
        'a,
        Result<Vec<AuthorizedResource<citadel_deployments::Deployment>>, DeploymentError>,
    > {
        self.list_authorized_impl(actor_id, administrator, filter)
    }
    fn get_authorized<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'a, Result<AuthorizedResource<citadel_deployments::Deployment>, DeploymentError>>
    {
        self.get_authorized_impl(actor_id, administrator, id)
    }
    fn create<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        input: &'a CreateDeployment,
    ) -> BoxFuture<'a, Result<AuthorizedResource<citadel_deployments::Deployment>, DeploymentError>>
    {
        self.create_impl(actor_id, administrator, input)
    }
    fn update_config<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        expected_row_version: i64,
        spec: &'a DeploymentSpec,
    ) -> BoxFuture<'a, Result<AuthorizedResource<citadel_deployments::Deployment>, DeploymentError>>
    {
        self.update_config_impl(actor_id, administrator, id, expected_row_version, spec)
    }
    fn update_metadata<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        input: &'a UpdateDeploymentMetadata,
    ) -> BoxFuture<'a, Result<AuthorizedResource<citadel_deployments::Deployment>, DeploymentError>>
    {
        self.update_metadata_impl(actor_id, administrator, id, input)
    }
    fn rename<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        name: &'a str,
    ) -> BoxFuture<'a, Result<AuthorizedResource<citadel_deployments::Deployment>, DeploymentError>>
    {
        self.rename_impl(actor_id, administrator, id, name)
    }
    fn duplicate_draft<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'a, Result<DeploymentDuplicateDraft, DeploymentError>> {
        self.duplicate_draft_impl(actor_id, administrator, id)
    }
    fn claim_delete<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<DeletionClaim>, DeploymentError>> {
        self.claim_delete_impl(actor_id, administrator, ids)
    }
    fn complete_delete<'a>(
        &'a self,
        actor_id: ActorId,
        claims: &'a [DeletionClaim],
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        self.complete_delete_impl(actor_id, claims)
    }
    fn release_delete<'a>(
        &'a self,
        claims: &'a [DeletionClaim],
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        self.release_delete_impl(claims)
    }
    fn claim_apply<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'a, Result<ApplyClaim, DeploymentError>> {
        self.claim_apply_impl(actor_id, administrator, id)
    }
    fn claim_apply_versioned(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        expected_version: Option<i64>,
    ) -> BoxFuture<'_, Result<ApplyClaim, DeploymentError>> {
        self.claim_apply_versioned_impl(actor_id, administrator, id, expected_version)
    }
    fn complete_apply<'a>(
        &'a self,
        actor_id: ActorId,
        claim: &'a ApplyClaim,
        result: &'a RuntimeDeploymentResult,
        digest: Option<&'a str>,
        bindings: &'a [DeploymentBindingSnapshot],
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        self.complete_apply_impl(actor_id, claim, result, digest, bindings)
    }
    fn fail_apply<'a>(
        &'a self,
        actor_id: ActorId,
        claim: &'a ApplyClaim,
        message: &'a str,
        result: Option<&'a RuntimeDeploymentResult>,
        bindings: &'a [DeploymentBindingSnapshot],
    ) -> BoxFuture<'a, Result<(), DeploymentError>> {
        self.fail_apply_impl(actor_id, claim, message, result, bindings)
    }
    fn stale_apply_claims<'a>(
        &'a self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'a, Result<Vec<(ActorId, ApplyClaim)>, DeploymentError>> {
        self.stale_apply_claims_impl(started_before, limit)
    }
}
