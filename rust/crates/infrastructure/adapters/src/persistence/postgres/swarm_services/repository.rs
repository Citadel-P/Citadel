use super::*;
use citadel_primitives::AuthorizedResource;

#[derive(Clone)]
pub struct PostgresSwarmServiceRepository {
    pub(super) pool: PgPool,
}

impl PostgresSwarmServiceRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
impl SwarmServiceRepository for PostgresSwarmServiceRepository {
    fn duplicate_draft(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'_, Result<citadel_swarm_services::SwarmServiceDuplicateDraft, SwarmServiceError>>
    {
        self.duplicate_draft_impl(actor, administrator, id)
    }

    fn update_check_candidates(
        &self,
        after: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, SwarmServiceError>> {
        self.update_check_candidates_impl(after, limit)
    }

    fn begin_update_check<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        expected: &'a citadel_swarm_services::SwarmService,
    ) -> BoxFuture<'a, Result<citadel_swarm_services::ServiceUpdateCheck, SwarmServiceError>> {
        self.begin_update_check_impl(actor, administrator, expected)
    }

    fn complete_update_check<'a>(
        &'a self,
        claim: &'a citadel_swarm_services::ServiceUpdateCheck,
        state: Option<&'a AutoUpdateState>,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        self.complete_update_check_impl(claim, state)
    }

    fn recover_update_checks(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, SwarmServiceError>> {
        self.recover_update_checks_impl(started_before, limit)
    }

    fn list_authorized<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        filter: &'a SwarmServiceFilter,
    ) -> BoxFuture<
        'a,
        Result<Vec<AuthorizedResource<citadel_swarm_services::SwarmService>>, SwarmServiceError>,
    > {
        self.list_authorized_impl(actor_id, administrator, filter)
    }

    fn get_authorized(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<
        '_,
        Result<AuthorizedResource<citadel_swarm_services::SwarmService>, SwarmServiceError>,
    > {
        self.get_authorized_impl(actor_id, administrator, id)
    }

    fn create<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        input: &'a CreateSwarmService,
    ) -> BoxFuture<
        'a,
        Result<AuthorizedResource<citadel_swarm_services::SwarmService>, SwarmServiceError>,
    > {
        self.create_impl(actor_id, administrator, input)
    }

    fn update<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        input: &'a UpdateSwarmService,
    ) -> BoxFuture<
        'a,
        Result<AuthorizedResource<citadel_swarm_services::SwarmService>, SwarmServiceError>,
    > {
        self.update_impl(actor_id, administrator, id, input)
    }

    fn update_description<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        description: Option<&'a str>,
    ) -> BoxFuture<
        'a,
        Result<AuthorizedResource<citadel_swarm_services::SwarmService>, SwarmServiceError>,
    > {
        self.update_description_impl(actor_id, administrator, id, description)
    }

    fn rename<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        input: &'a RenameSwarmService,
    ) -> BoxFuture<
        'a,
        Result<AuthorizedResource<citadel_swarm_services::SwarmService>, SwarmServiceError>,
    > {
        self.rename_impl(actor_id, administrator, input)
    }

    fn delete<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<ServiceDeletionClaim>, SwarmServiceError>> {
        self.delete_impl(actor_id, administrator, ids)
    }

    fn mark_delete_attempted<'a>(
        &'a self,
        claim: &'a ServiceDeletionClaim,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        self.mark_delete_attempted_impl(claim)
    }

    fn complete_delete<'a>(
        &'a self,
        actor_id: ActorId,
        deleted: &'a [ServiceDeletionClaim],
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        self.complete_delete_impl(actor_id, deleted)
    }

    fn release_delete<'a>(
        &'a self,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        self.release_delete_impl(ids)
    }

    fn stale_deletion_claims(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, ServiceDeletionClaim)>, SwarmServiceError>> {
        self.stale_deletion_claims_impl(started_before, limit)
    }

    fn claim_operation(
        &self,
        actor_id: ActorId,
        administrator: bool,
        request: citadel_swarm_services::ServiceOperationRequest,
    ) -> BoxFuture<'_, Result<ServiceOperationClaim, SwarmServiceError>> {
        self.claim_operation_impl(actor_id, administrator, request)
    }

    fn mark_attempted<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        self.mark_attempted_impl(claim)
    }

    fn mark_accepted<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        result: &'a RuntimeServiceResult,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        self.mark_accepted_impl(claim, result)
    }

    fn complete_operation<'a>(
        &'a self,
        actor_id: ActorId,
        claim: &'a ServiceOperationClaim,
        result: &'a RuntimeServiceResult,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        self.complete_operation_impl(actor_id, claim, result)
    }

    fn fail_operation<'a>(
        &'a self,
        actor_id: ActorId,
        claim: &'a ServiceOperationClaim,
        message: &'a str,
        outcome_unknown: bool,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        self.fail_operation_impl(actor_id, claim, message, outcome_unknown)
    }

    fn active_operation_claims(
        &self,
        after: Option<Uuid>,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, ServiceOperationClaim)>, SwarmServiceError>> {
        self.active_operation_claims_impl(after, limit)
    }

    fn stale_operation_claims(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, ServiceOperationClaim)>, SwarmServiceError>> {
        self.stale_operation_claims_impl(started_before, limit)
    }
}
