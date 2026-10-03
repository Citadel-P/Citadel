use crate::*;
use chrono::{DateTime, Utc};
use citadel_primitives::ActorId;
use citadel_primitives::WebhookConfig;
use futures_util::future::BoxFuture;
use serde_json::Value;
use uuid::Uuid;

pub trait AutomationRepository: Send + Sync {
    fn permissions<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<
        'a,
        Result<
            std::collections::BTreeMap<Uuid, citadel_primitives::PermissionLevel>,
            AutomationError,
        >,
    >;
    fn create<'a>(
        &'a self,
        actor: ActorId,
        input: &'a AutomationActionConfiguration,
    ) -> BoxFuture<'a, Result<AutomationAction, AutomationError>>;
    fn list(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<AutomationAction>, AutomationError>>;
    fn get<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<AutomationAction, AutomationError>>;
    fn update<'a>(
        &'a self,
        current: &'a AutomationAction,
        input: &'a AutomationActionConfiguration,
        actor: ActorId,
        metadata_only: bool,
    ) -> BoxFuture<'a, Result<AutomationAction, AutomationError>>;
    fn rename<'a>(
        &'a self,
        id: Uuid,
        name: &'a str,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<AutomationAction, AutomationError>>;
    fn delete<'a>(&'a self, id: Uuid, actor: ActorId)
    -> BoxFuture<'a, Result<(), AutomationError>>;
    fn enqueue<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
        args: &'a Value,
        timeout_seconds: Option<i32>,
    ) -> BoxFuture<'a, Result<AutomationRun, AutomationError>>;
    fn enqueue_for_execution<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
        args: &'a Value,
        timeout_seconds: Option<i32>,
        code: Option<&'a str>,
    ) -> BoxFuture<'a, Result<AutomationRunClaim, AutomationError>>;
    fn enqueue_webhook<'a>(
        &'a self,
        id: Uuid,
        expected_webhook: &'a WebhookConfig,
        args: &'a Value,
    ) -> BoxFuture<'a, Result<AutomationRun, AutomationError>>;
    fn claim_next<'a>(
        &'a self,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<AutomationRunClaim>, AutomationError>>;
    fn finish<'a>(
        &'a self,
        claim: &'a AutomationRunClaim,
        result: &'a AutomationRunResult,
    ) -> BoxFuture<'a, Result<bool, AutomationError>>;
    fn list_runs<'a>(
        &'a self,
        action_id: Uuid,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<AutomationRun>, AutomationError>>;
    fn get_run<'a>(
        &'a self,
        action_id: Uuid,
        run_id: Uuid,
    ) -> BoxFuture<'a, Result<AutomationRun, AutomationError>>;
    fn list_scheduled<'a>(
        &'a self,
        after: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<AutomationAction>, AutomationError>>;
    fn enqueue_scheduled<'a>(
        &'a self,
        action_id: Uuid,
        scheduled_minute: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<AutomationRun>, AutomationError>>;
    fn cancel<'a>(
        &'a self,
        action_id: Uuid,
        run_id: Uuid,
    ) -> BoxFuture<'a, Result<(), AutomationError>>;
}
