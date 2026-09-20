use crate::*;
use citadel_activities::ActivityEventInfo;
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use uuid::Uuid;

pub trait StackRepository: Send + Sync {
    fn update_check_candidates(
        &self,
        after: Uuid,
        images: bool,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<Uuid>, StackError>> {
        let _ = (after, images, limit);
        Box::pin(async { Ok(Vec::new()) })
    }
    fn save_update_check<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        expected: &'a StackDetails,
        state: &'a crate::StackUpdateState,
    ) -> BoxFuture<'a, Result<(), StackError>> {
        let _ = (actor, administrator, expected, state);
        Box::pin(async {
            Err(StackError::Runtime(
                "Stack update persistence is unavailable.".into(),
            ))
        })
    }
    fn claim_apply_versioned(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        rollback: Option<Uuid>,
        webhook: Option<Uuid>,
        options: crate::StackApplyOptions,
    ) -> BoxFuture<'_, Result<StackOperationClaim, StackError>> {
        if options.expected_version.is_some() || !options.service_names.is_empty() {
            return Box::pin(async {
                Err(StackError::Conflict(
                    "Versioned Stack Apply is unavailable.".into(),
                ))
            });
        }
        self.claim_apply(actor, administrator, id, rollback, webhook)
    }
    fn enqueue_webhook<'a>(
        &'a self,
        expected: &'a StackDetails,
        commit: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), StackError>>;
    fn ready_webhooks(
        &self,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<crate::StackWebhookJob>, StackError>>;
    fn discard_webhook(&self, id: Uuid) -> BoxFuture<'_, Result<(), StackError>>;
    fn record_drift<'a>(
        &'a self,
        expected: &'a StackDetails,
        status: StackReleaseStatus,
        info: ActivityEventInfo,
    ) -> BoxFuture<'a, Result<bool, StackError>>;
    fn drift_monitor_candidates(
        &self,
        after: Option<Uuid>,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<StackDetails>, StackError>>;
    fn list_authorized<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        filter: &'a StackFilter,
    ) -> BoxFuture<'a, Result<Vec<StackDetails>, StackError>>;
    fn get_authorized(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'_, Result<StackDetails, StackError>>;
    fn create<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        input: &'a CreateStack,
    ) -> BoxFuture<'a, Result<StackDetails, StackError>>;
    fn update<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        expected_row_version: i64,
        input: &'a UpdateStack,
        spec: &'a StackSpec,
    ) -> BoxFuture<'a, Result<StackDetails, StackError>>;
    fn update_metadata<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        description: Option<&'a str>,
    ) -> BoxFuture<'a, Result<StackDetails, StackError>>;
    fn rename<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        name: &'a str,
    ) -> BoxFuture<'a, Result<StackDetails, StackError>>;
    fn releases(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<StackReleaseDetails>, StackError>>;
    fn claim_apply(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        rollback_release_id: Option<Uuid>,
        webhook_job_id: Option<Uuid>,
    ) -> BoxFuture<'_, Result<StackOperationClaim, StackError>>;
    fn record_apply_source<'a>(
        &'a self,
        claim: &'a StackOperationClaim,
        source: &'a crate::StackReleaseSource,
    ) -> BoxFuture<'a, Result<(), StackError>>;
    fn complete_apply<'a>(
        &'a self,
        actor: ActorId,
        claim: &'a StackOperationClaim,
        result: &'a StackRuntimeResult,
        bindings: &'a [crate::ResourceBindingSnapshot],
        source: Option<&'a crate::StackReleaseSource>,
    ) -> BoxFuture<'a, Result<(), StackError>>;
    fn fail_apply<'a>(
        &'a self,
        actor: ActorId,
        claim: &'a StackOperationClaim,
        message: &'a str,
        unknown: bool,
    ) -> BoxFuture<'a, Result<(), StackError>>;
    fn stale_apply_claims(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, StackOperationClaim)>, StackError>>;
    fn stale_delete_claims(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, StackDeletionClaim)>, StackError>>;
    fn claim_delete<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<StackDeletionClaim>, StackError>>;
    fn complete_delete<'a>(
        &'a self,
        actor: ActorId,
        claims: &'a [StackDeletionClaim],
    ) -> BoxFuture<'a, Result<(), StackError>>;
    fn release_delete<'a>(&'a self, ids: &'a [Uuid]) -> BoxFuture<'a, Result<(), StackError>>;
    fn claim_state<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<StackStateClaim>, StackError>>;
    fn complete_state<'a>(
        &'a self,
        actor: ActorId,
        claim: &'a StackStateClaim,
        status: StackReleaseStatus,
        container_ids: &'a [String],
    ) -> BoxFuture<'a, Result<(), StackError>>;
    fn release_state<'a>(
        &'a self,
        claims: &'a [StackStateClaim],
    ) -> BoxFuture<'a, Result<(), StackError>>;
    fn stale_state_claims(
        &self,
        started_before: i64,
        limit: i64,
    ) -> BoxFuture<'_, Result<Vec<(ActorId, StackStateClaim)>, StackError>>;
    fn import<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        input: &'a ImportComposeProject,
        claim: &'a StackImportClaim,
    ) -> BoxFuture<'a, Result<StackDetails, StackError>>;
    fn find_import_owner(
        &self,
        platform_id: Uuid,
        project_name: &str,
    ) -> BoxFuture<'_, Result<Option<Uuid>, StackError>>;
}
