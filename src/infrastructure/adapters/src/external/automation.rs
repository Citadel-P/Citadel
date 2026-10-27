//! Authenticated Agent dispatch; persistence records ownership before transmission.
use crate::{
    connectors::{
        agent::{client::AgentClient, execution::AgentExecutionClient},
        edge::{EdgeRegistry, EdgeTarget},
    },
    persistence::postgres::automation::executors::{self, Host, Reservation, Target},
};
use citadel_automation::{
    AutomationError, AutomationProgress, AutomationRunClaim, AutomationRunResult,
    AutomationRunStatus, RemoteAutomationExecutor, RemoteAutomationOutcome,
    sandbox::AutomationScript,
};
use citadel_contracts::citadel::automation::v1::{AutomationAttempt, AutomationRequest};
use citadel_identity::IdentityService;
use citadel_primitives::ResourceType;
use futures_util::{StreamExt, future::BoxFuture};
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

pub struct AgentAutomationExecutor {
    pool: sqlx::PgPool,
    direct: Option<AgentClient>,
    edge: EdgeRegistry,
    identity: Arc<IdentityService>,
    secure_edge: bool,
}
impl AgentAutomationExecutor {
    pub fn new(
        pool: sqlx::PgPool,
        direct: Option<AgentClient>,
        edge: EdgeRegistry,
        identity: Arc<IdentityService>,
        secure_edge: bool,
    ) -> Self {
        Self {
            pool,
            direct,
            edge,
            identity,
            secure_edge,
        }
    }
    async fn connect(
        &self,
        target: &Target,
        cancel: &CancellationToken,
    ) -> Result<Option<AgentExecutionClient>, AutomationError> {
        if let Some(endpoint) = &target.endpoint {
            // Signed requests authenticate but do not encrypt script credentials.
            if !url::Url::parse(endpoint).is_ok_and(|url| url.scheme() == "https") {
                return Err(AutomationError::Validation(
                    "Remote automation requires HTTPS for Direct Agents.".into(),
                ));
            }
            let Some(client) = &self.direct else {
                return Ok(None);
            };
            Ok(
                match tokio::time::timeout(
                    Duration::from_secs(10),
                    client.for_address(endpoint, target.id, cancel),
                )
                .await
                {
                    Ok(Ok(client)) => Some(AgentExecutionClient::Direct(Arc::new(client))),
                    _ => None,
                },
            )
        } else {
            if !self.secure_edge {
                return Err(AutomationError::Validation(
                    "Remote automation requires an HTTPS Edge Agent public URL.".into(),
                ));
            }
            let key = match target.kind {
                ResourceType::Platform => EdgeTarget::platform(target.id),
                ResourceType::BuildAgentPool => EdgeTarget::build_pool(target.id),
                _ => {
                    return Err(AutomationError::Validation(
                        "Invalid Agent resource type.".into(),
                    ));
                }
            };
            Ok(self.edge.get(&key).ok().map(AgentExecutionClient::Edge))
        }
    }
    async fn cleanup(
        &self,
        client: &AgentExecutionClient,
        reservation: &Reservation,
    ) -> Result<bool, AutomationError> {
        let cancel = CancellationToken::new();
        let result = tokio::time::timeout(
            Duration::from_secs(10),
            client.automation_cleanup(attempt(reservation), &cancel),
        )
        .await;
        if matches!(result, Ok(Ok(ref result)) if result.confirmed) {
            executors::confirm(&self.pool, reservation).await?;
            return Ok(true);
        }
        Ok(false)
    }
}
fn attempt(value: &Reservation) -> AutomationAttempt {
    AutomationAttempt {
        run_id: value.run.to_string(),
        attempt_id: value.attempt.to_string(),
        authority_id: value.authority.to_string(),
        generation: value.generation,
    }
}
fn result(status: AutomationRunStatus, error: &str) -> RemoteAutomationOutcome {
    RemoteAutomationOutcome::Completed(AutomationRunResult {
        status,
        exit_code: None,
        logs: String::new(),
        error: Some(error.into()),
    })
}
impl RemoteAutomationExecutor for AgentAutomationExecutor {
    fn execute<'a>(
        &'a self,
        claim: &'a AutomationRunClaim,
        script: AutomationScript,
        progress: &'a tokio::sync::broadcast::Sender<AutomationProgress>,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RemoteAutomationOutcome, AutomationError>> {
        Box::pin(async move {
            if !url::Url::parse(&script.base_url).is_ok_and(|url| url.scheme() == "https") {
                return Err(AutomationError::Validation(
                    "Configure an HTTPS Citadel public URL before running actions on Agents."
                        .into(),
                ));
            }
            let principal = self
                .identity
                .authenticate_bearer(&script.token)
                .await
                .map_err(|_| {
                    AutomationError::Validation("Run-as authorization is no longer valid.".into())
                })?;
            let ids = claim.run.executor.agent_ids();
            if ids.is_empty() {
                return Err(AutomationError::Validation(
                    "Select eligible automation Agents.".into(),
                ));
            }
            let offset = usize::from(claim.run.id.as_bytes()[15]) % ids.len();
            let mut authorized = false;
            let selection_started = std::time::Instant::now();
            for index in 0..ids.len() {
                if index > 0 && selection_started.elapsed() >= Duration::from_secs(20) {
                    break;
                }
                let id = ids[(offset + index) % ids.len()];
                let Some(target) = executors::target(&self.pool, id).await? else {
                    continue;
                };
                let allowed = match target.kind {
                    ResourceType::Platform => self.identity.require_resource::<citadel_automation::permissions::ExecuteAutomationAgent>(&principal, id).await,
                    ResourceType::BuildAgentPool => self.identity.require_resource::<citadel_builds::permissions::ExecuteBuildAgentPool>(&principal, id).await,
                    _ => continue,
                };
                if allowed.is_err() {
                    continue;
                }
                authorized = true;
                if !target.available {
                    continue;
                }
                let Some(client) = self.connect(&target, cancellation).await? else {
                    continue;
                };
                let host = match tokio::time::timeout(
                    Duration::from_secs(10),
                    client.automation_host(cancellation),
                )
                .await
                {
                    Ok(Ok(host)) => host,
                    _ => continue,
                };
                if host.occupied >= host.slots {
                    continue;
                }
                let host = Host {
                    daemon: host.daemon_id,
                    authority: uuid::Uuid::parse_str(&host.authority_id).map_err(|_| {
                        AutomationError::External("Agent returned an invalid authority.".into())
                    })?,
                    generation: host.generation,
                    slots: host.slots,
                    heavy: host.heavy_slots,
                };
                if !executors::reserve(&self.pool, claim, &target, &host).await? {
                    continue;
                }
                let reservation = Reservation {
                    run: claim.run.id,
                    attempt: claim
                        .run
                        .attempt_id
                        .ok_or_else(|| AutomationError::Conflict("Missing attempt.".into()))?,
                    agent: id,
                    endpoint: target.endpoint.clone(),
                    kind: target.kind,
                    daemon: host.daemon,
                    authority: host.authority,
                    generation: host.generation,
                };
                let maximum = script
                    .maximum_log_bytes
                    .saturating_mul(2)
                    .saturating_add(1024);
                let request = AutomationRequest {
                    attempt: Some(attempt(&reservation)),
                    source: script.source,
                    token: script.token.clone(),
                    args_json: script.args_json,
                    core_url: script.base_url,
                    timeout_seconds: script.timeout_seconds as u32,
                    maximum_log_bytes: script.maximum_log_bytes as u32,
                };
                let mut outcome = result(
                    AutomationRunStatus::Failed,
                    "Agent execution ended without a confirmed result. Cleanup must be verified; this attempt will not be retried.",
                );
                let deadline =
                    tokio::time::sleep(Duration::from_secs(script.timeout_seconds as u64 + 30));
                tokio::pin!(deadline);
                let mut saw_output = false;
                if let Ok(mut stream) = client.automation_run(request, cancellation).await {
                    loop {
                        let next = tokio::select! {
                            () = cancellation.cancelled() => { outcome = result(AutomationRunStatus::Cancelled, "Automation cancellation requested."); break; },
                            () = &mut deadline => break,
                            item = stream.next() => item,
                        };
                        match next {
                            Some(Ok(event))
                                if event.not_started
                                    && !event.completed
                                    && !saw_output
                                    && event.stream.is_empty()
                                    && event.logs.is_empty() =>
                            {
                                outcome = RemoteAutomationOutcome::Deferred;
                                break;
                            }
                            Some(Ok(event))
                                if event.completed
                                    && !event.not_started
                                    && event.logs.len() <= maximum
                                    && event
                                        .error
                                        .as_ref()
                                        .is_none_or(|error| error.len() <= maximum) =>
                            {
                                let status = match event.status.as_str() {
                                    "Succeeded" => AutomationRunStatus::Succeeded,
                                    "Failed" => AutomationRunStatus::Failed,
                                    "Cancelled" => AutomationRunStatus::Cancelled,
                                    "TimedOut" => AutomationRunStatus::TimedOut,
                                    _ => break,
                                };
                                outcome = RemoteAutomationOutcome::Completed(AutomationRunResult {
                                    status,
                                    exit_code: event.exit_code,
                                    logs: event.logs.replace(&script.token, "[redacted]"),
                                    error: event
                                        .error
                                        .map(|value| value.replace(&script.token, "[redacted]")),
                                });
                                break;
                            }
                            Some(Ok(event))
                                if !event.not_started
                                    && !event.completed
                                    && event.stream.len() <= maximum =>
                            {
                                saw_output = true;
                                let _ = progress.send(AutomationProgress {
                                    run_id: Some(claim.run.id),
                                    stream: Some(event.stream.replace(&script.token, "[redacted]")),
                                    ..Default::default()
                                });
                            }
                            _ => break,
                        }
                    }
                }
                if cancellation.is_cancelled()
                    && matches!(&outcome, RemoteAutomationOutcome::Completed(r) if r.status == AutomationRunStatus::Failed)
                {
                    outcome = result(
                        AutomationRunStatus::Cancelled,
                        "Automation cancellation requested.",
                    );
                }
                let cleaned = self.cleanup(&client, &reservation).await?;
                if matches!(outcome, RemoteAutomationOutcome::Deferred) && !cleaned {
                    return Ok(result(
                        AutomationRunStatus::Failed,
                        "Agent refused execution, but cleanup could not be confirmed. This attempt will not be retried.",
                    ));
                }
                return Ok(outcome);
            }
            if !authorized {
                return Err(AutomationError::Validation(
                    "The run-as identity cannot execute on any selected Agent.".into(),
                ));
            }
            Ok(RemoteAutomationOutcome::Deferred)
        })
    }
    fn resolve<'a>(
        &'a self,
        run: &'a citadel_automation::AutomationRun,
        actor: citadel_primitives::ActorId,
        reason: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), AutomationError>> {
        Box::pin(async move {
            let r = executors::retained(&self.pool, run.id)
                .await?
                .filter(|r| Some(r.attempt) == run.attempt_id)
                .ok_or_else(|| {
                    AutomationError::Conflict(
                        "The retained attempt changed. Refresh the run.".into(),
                    )
                })?;
            let target = Target {
                id: r.agent,
                kind: r.kind,
                endpoint: r.endpoint.clone(),
                available: true,
            };
            let client = self.connect(&target, cancel).await?.ok_or_else(|| {
                AutomationError::Conflict("The original Agent is offline.".into())
            })?;
            let host = tokio::select! {
                biased;
                () = cancel.cancelled() => return Err(AutomationError::Conflict("Recovery was interrupted.".into())),
                result = tokio::time::timeout(Duration::from_secs(10), client.automation_host(cancel)) => result.map_err(|_| AutomationError::Conflict("Agent check timed out.".into()))?.map_err(|_| AutomationError::Conflict("Could not verify the original Agent.".into()))?,
            };
            let authority = uuid::Uuid::parse_str(&host.authority_id)
                .ok()
                .filter(|v| !v.is_nil());
            if host.daemon_id != r.daemon || authority.is_none() || authority == Some(r.authority) {
                return Err(AutomationError::Conflict("Stop the old script and restart the original Agent on the same Docker host before resolving cleanup.".into()));
            }
            executors::resolve(&self.pool, &r, actor, reason).await
        })
    }
    fn reconcile<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<bool, AutomationError>> {
        Box::pin(async move {
            let mut changed = false;
            for reservation in executors::pending(&self.pool).await? {
                if cancellation.is_cancelled() {
                    break;
                }
                let target = Target {
                    id: reservation.agent,
                    kind: reservation.kind,
                    endpoint: reservation.endpoint.clone(),
                    available: true,
                };
                let Ok(Some(client)) = self.connect(&target, cancellation).await else {
                    continue;
                };
                changed |= self.cleanup(&client, &reservation).await?;
            }
            Ok(changed)
        })
    }
}
