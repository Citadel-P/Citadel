use crate::*;
use citadel_domain::ActorId;
use futures_util::future::BoxFuture;
use uuid::Uuid;

/// The optional version fences an automated action to the configuration that
/// was authenticated and checked, without changing ordinary interactive Apply.
#[derive(Clone, Copy)]
pub struct ServiceOperationRequest {
    pub id: Uuid,
    pub kind: ServiceOperationKind,
    pub replicas: Option<i32>,
    pub expected_version: Option<i64>,
}

pub trait SwarmServiceRepository: Send + Sync {
    fn duplicate_draft(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'_, Result<crate::SwarmServiceDuplicateDraft, SwarmServiceError>>;
    fn update_check_candidates(
        &self,
        after: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, SwarmServiceError>>;
    fn begin_update_check<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        expected: &'a SwarmServiceDetails,
    ) -> BoxFuture<'a, Result<ServiceUpdateCheck, SwarmServiceError>>;
    fn complete_update_check<'a>(
        &'a self,
        claim: &'a ServiceUpdateCheck,
        state: Option<&'a crate::AutoUpdateState>,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>>;
    fn recover_update_checks(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, SwarmServiceError>>;
    fn list_authorized<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        filter: &'a SwarmServiceFilter,
    ) -> BoxFuture<'a, Result<Vec<SwarmServiceDetails>, SwarmServiceError>>;
    fn get_authorized(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'_, Result<SwarmServiceDetails, SwarmServiceError>>;
    fn create<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        input: &'a CreateSwarmService,
    ) -> BoxFuture<'a, Result<SwarmServiceDetails, SwarmServiceError>>;
    fn update<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        input: &'a UpdateSwarmService,
    ) -> BoxFuture<'a, Result<SwarmServiceDetails, SwarmServiceError>>;
    fn update_description<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
        description: Option<&'a str>,
    ) -> BoxFuture<'a, Result<SwarmServiceDetails, SwarmServiceError>>;

    fn rename<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        input: &'a RenameSwarmService,
    ) -> BoxFuture<'a, Result<SwarmServiceDetails, SwarmServiceError>>;
    fn delete<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<ServiceDeletionClaim>, SwarmServiceError>>;
    fn mark_delete_attempted<'a>(
        &'a self,
        claim: &'a ServiceDeletionClaim,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>>;
    fn complete_delete<'a>(
        &'a self,
        actor_id: ActorId,
        deleted: &'a [ServiceDeletionClaim],
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>>;
    fn release_delete<'a>(
        &'a self,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>>;
    fn stale_deletion_claims(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, ServiceDeletionClaim)>, SwarmServiceError>>;
    fn claim_operation(
        &self,
        actor_id: ActorId,
        administrator: bool,
        request: ServiceOperationRequest,
    ) -> BoxFuture<'_, Result<ServiceOperationClaim, SwarmServiceError>>;
    fn mark_attempted<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>>;
    fn mark_accepted<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        result: &'a RuntimeServiceResult,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>>;
    fn complete_operation<'a>(
        &'a self,
        actor_id: ActorId,
        claim: &'a ServiceOperationClaim,
        result: &'a RuntimeServiceResult,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>>;
    fn fail_operation<'a>(
        &'a self,
        actor_id: ActorId,
        claim: &'a ServiceOperationClaim,
        message: &'a str,
        outcome_unknown: bool,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>>;
    fn stale_operation_claims(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, ServiceOperationClaim)>, SwarmServiceError>>;
}
