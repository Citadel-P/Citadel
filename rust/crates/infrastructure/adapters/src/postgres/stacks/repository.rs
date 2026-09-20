use super::*;

#[derive(Clone)]
pub struct PostgresStackRepository {
    pub(super) pool: PgPool,
}

impl PostgresStackRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
impl StackRepository for PostgresStackRepository {
    fn update_check_candidates(
        &self,
        after: Uuid,
        images: bool,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, StackError>> {
        self.update_check_candidates_impl(after, images, limit)
    }

    fn save_update_check<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        expected: &'a StackDetails,
        state: &'a StackUpdateState,
    ) -> BoxFuture<'a, Result<(), StackError>> {
        self.save_update_check_impl(actor, administrator, expected, state)
    }

    fn enqueue_webhook<'a>(
        &'a self,
        expected: &'a StackDetails,
        commit: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), StackError>> {
        self.enqueue_webhook_impl(expected, commit)
    }

    fn ready_webhooks(
        &self,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<citadel_stacks::StackWebhookJob>, StackError>> {
        self.ready_webhooks_impl(limit)
    }

    fn discard_webhook(&self, id: Uuid) -> BoxFuture<'_, Result<(), StackError>> {
        self.discard_webhook_impl(id)
    }

    fn record_drift<'a>(
        &'a self,
        expected: &'a StackDetails,
        status: StackReleaseStatus,
        info: ActivityEventInfo,
    ) -> BoxFuture<'a, Result<bool, StackError>> {
        self.record_drift_impl(expected, status, info)
    }

    fn drift_monitor_candidates(
        &self,
        after: Option<Uuid>,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<StackDetails>, StackError>> {
        self.drift_monitor_candidates_impl(after, limit)
    }

    fn list_authorized<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        filter: &'a StackFilter,
    ) -> BoxFuture<'a, Result<Vec<StackDetails>, StackError>> {
        self.list_authorized_impl(actor, administrator, filter)
    }

    fn get_authorized(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'_, Result<StackDetails, StackError>> {
        self.get_authorized_impl(actor, administrator, id)
    }

    fn create<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        input: &'a CreateStack,
    ) -> BoxFuture<'a, Result<StackDetails, StackError>> {
        self.create_impl(actor, administrator, input)
    }

    fn update<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        expected_row_version: i64,
        input: &'a UpdateStack,
        spec: &'a StackSpec,
    ) -> BoxFuture<'a, Result<StackDetails, StackError>> {
        self.update_impl(actor, administrator, id, expected_row_version, input, spec)
    }

    fn update_metadata<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        description: Option<&'a str>,
    ) -> BoxFuture<'a, Result<StackDetails, StackError>> {
        self.update_metadata_impl(actor, administrator, id, description)
    }

    fn rename<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        name: &'a str,
    ) -> BoxFuture<'a, Result<StackDetails, StackError>> {
        self.rename_impl(actor, administrator, id, name)
    }

    fn releases(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<StackReleaseDetails>, StackError>> {
        self.releases_impl(actor, administrator, id)
    }

    fn claim_apply(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        rollback_release_id: Option<Uuid>,
        webhook_job_id: Option<Uuid>,
    ) -> BoxFuture<'_, Result<StackOperationClaim, StackError>> {
        self.claim_apply_impl(
            actor,
            administrator,
            id,
            rollback_release_id,
            webhook_job_id,
        )
    }

    fn claim_apply_versioned(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        rollback_release_id: Option<Uuid>,
        webhook_job_id: Option<Uuid>,
        options: citadel_stacks::StackApplyOptions,
    ) -> BoxFuture<'_, Result<StackOperationClaim, StackError>> {
        self.claim_apply_versioned_impl(
            actor,
            administrator,
            id,
            rollback_release_id,
            webhook_job_id,
            options,
        )
    }

    fn record_apply_source<'a>(
        &'a self,
        claim: &'a StackOperationClaim,
        source: &'a StackReleaseSource,
    ) -> BoxFuture<'a, Result<(), StackError>> {
        self.record_apply_source_impl(claim, source)
    }

    fn complete_apply<'a>(
        &'a self,
        actor: ActorId,
        claim: &'a StackOperationClaim,
        result: &'a StackRuntimeResult,
        bindings: &'a [ResourceBindingSnapshot],
        source: Option<&'a StackReleaseSource>,
    ) -> BoxFuture<'a, Result<(), StackError>> {
        self.complete_apply_impl(actor, claim, result, bindings, source)
    }

    fn fail_apply<'a>(
        &'a self,
        actor: ActorId,
        claim: &'a StackOperationClaim,
        message: &'a str,
        unknown: bool,
    ) -> BoxFuture<'a, Result<(), StackError>> {
        self.fail_apply_impl(actor, claim, message, unknown)
    }

    fn stale_apply_claims(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, StackOperationClaim)>, StackError>> {
        self.stale_apply_claims_impl(started_before, limit)
    }

    fn stale_delete_claims(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, StackDeletionClaim)>, StackError>> {
        self.stale_delete_claims_impl(started_before, limit)
    }

    fn claim_delete<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<StackDeletionClaim>, StackError>> {
        self.claim_delete_impl(actor, administrator, ids)
    }

    fn complete_delete<'a>(
        &'a self,
        actor: ActorId,
        claims: &'a [StackDeletionClaim],
    ) -> BoxFuture<'a, Result<(), StackError>> {
        self.complete_delete_impl(actor, claims)
    }

    fn release_delete<'a>(&'a self, ids: &'a [Uuid]) -> BoxFuture<'a, Result<(), StackError>> {
        self.release_delete_impl(ids)
    }

    fn claim_state<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<StackStateClaim>, StackError>> {
        self.claim_state_impl(actor, administrator, ids)
    }

    fn complete_state<'a>(
        &'a self,
        actor: ActorId,
        claim: &'a StackStateClaim,
        status: StackReleaseStatus,
        container_ids: &'a [String],
    ) -> BoxFuture<'a, Result<(), StackError>> {
        self.complete_state_impl(actor, claim, status, container_ids)
    }

    fn release_state<'a>(
        &'a self,
        claims: &'a [StackStateClaim],
    ) -> BoxFuture<'a, Result<(), StackError>> {
        self.release_state_impl(claims)
    }

    fn stale_state_claims(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, StackStateClaim)>, StackError>> {
        self.stale_state_claims_impl(started_before, limit)
    }

    fn import<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        input: &'a ImportComposeProject,
        claim: &'a StackImportClaim,
    ) -> BoxFuture<'a, Result<StackDetails, StackError>> {
        self.import_impl(actor, administrator, input, claim)
    }

    fn find_import_owner(
        &self,
        platform_id: Uuid,
        project_name: &str,
    ) -> BoxFuture<'_, Result<Option<Uuid>, StackError>> {
        self.find_import_owner_impl(platform_id, project_name)
    }
}
