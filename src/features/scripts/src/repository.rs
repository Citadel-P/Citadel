use crate::*;
use chrono::{DateTime, Utc};
use citadel_primitives::ActorId;
use citadel_primitives::WebhookConfig;
use futures_util::future::BoxFuture;
use serde_json::Value;
use uuid::Uuid;

pub trait ScriptRepository: Send + Sync {
    fn permissions<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<
        'a,
        Result<std::collections::BTreeMap<Uuid, citadel_primitives::PermissionLevel>, ScriptError>,
    >;
    fn create<'a>(
        &'a self,
        actor: ActorId,
        input: &'a ScriptConfiguration,
    ) -> BoxFuture<'a, Result<Script, ScriptError>>;
    fn list(
        &self,
        actor: ActorId,
        administrator: bool,
    ) -> BoxFuture<'_, Result<Vec<Script>, ScriptError>>;
    fn get<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<Script, ScriptError>>;
    fn update<'a>(
        &'a self,
        current: &'a Script,
        input: &'a ScriptConfiguration,
        actor: ActorId,
        metadata_only: bool,
    ) -> BoxFuture<'a, Result<Script, ScriptError>>;
    fn rename<'a>(
        &'a self,
        id: Uuid,
        name: &'a str,
        actor: ActorId,
    ) -> BoxFuture<'a, Result<Script, ScriptError>>;
    fn delete<'a>(&'a self, id: Uuid, actor: ActorId) -> BoxFuture<'a, Result<(), ScriptError>>;
    fn enqueue<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
        args: &'a Value,
        timeout_seconds: Option<i32>,
    ) -> BoxFuture<'a, Result<ScriptRun, ScriptError>>;
    fn enqueue_with_draft<'a>(
        &'a self,
        actor: ActorId,
        id: Uuid,
        trigger: &'a str,
        args: &'a Value,
        timeout_seconds: Option<i32>,
        draft: Option<&'a ScriptDraft>,
    ) -> BoxFuture<'a, Result<ScriptRun, ScriptError>>;
    fn enqueue_webhook<'a>(
        &'a self,
        id: Uuid,
        expected_webhook: &'a WebhookConfig,
        args: &'a Value,
    ) -> BoxFuture<'a, Result<ScriptRun, ScriptError>>;
    fn claim_next<'a>(
        &'a self,
        stale_before: DateTime<Utc>,
        remote: bool,
    ) -> BoxFuture<'a, Result<ScriptClaimResult, ScriptError>>;
    /// A cancellation request is durable across the queued-to-running transition.
    fn cancellation_requested(&self, run_id: Uuid) -> BoxFuture<'_, Result<bool, ScriptError>>;
    fn defer<'a>(
        &'a self,
        claim: &'a ScriptRunClaim,
    ) -> BoxFuture<'a, Result<ScriptRunStatus, ScriptError>>;
    fn finish<'a>(
        &'a self,
        claim: &'a ScriptRunClaim,
        result: &'a ScriptRunResult,
    ) -> BoxFuture<'a, Result<bool, ScriptError>>;
    fn list_runs<'a>(
        &'a self,
        script_id: Uuid,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<ScriptRun>, ScriptError>>;
    fn get_run<'a>(
        &'a self,
        script_id: Uuid,
        run_id: Uuid,
    ) -> BoxFuture<'a, Result<ScriptRun, ScriptError>>;
    fn list_scheduled<'a>(
        &'a self,
        after: Option<Uuid>,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<Script>, ScriptError>>;
    fn enqueue_scheduled<'a>(
        &'a self,
        script_id: Uuid,
        scheduled_minute: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<ScriptRun>, ScriptError>>;
    fn cancel<'a>(
        &'a self,
        script_id: Uuid,
        run_id: Uuid,
    ) -> BoxFuture<'a, Result<(), ScriptError>>;
}
