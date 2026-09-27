use super::*;

#[derive(Default)]
pub(super) struct HealthState {
    pub(super) online: Option<bool>,
    pub(super) successes: u8,
    pub(super) failures: u8,
}
impl HealthState {
    pub(super) fn observe(&mut self, healthy: bool) -> Option<bool> {
        if healthy {
            self.failures = 0;
            self.successes = self.successes.saturating_add(1);
        } else {
            self.successes = 0;
            self.failures = self.failures.saturating_add(1);
        }
        let confirmed = if healthy {
            self.successes >= 2
        } else {
            self.failures >= 3
        };
        if confirmed && self.online != Some(healthy) {
            self.online = Some(healthy);
            Some(healthy)
        } else {
            None
        }
    }
}

pub(super) async fn probe_health(
    runtime: &dyn PlatformHealthPort,
    token: &CancellationToken,
    timeout: Duration,
) -> bool {
    matches!(
        tokio::time::timeout(timeout, runtime.probe(token)).await,
        Ok(Ok(_))
    )
}

pub(super) async fn edge_health(
    pool: &PgPool,
    platform: uuid::Uuid,
) -> Result<Option<bool>, sqlx::Error> {
    // No binding (pending enrollment) and revoked bindings must not raise an
    // unreachable alert. Node bindings do not determine the manager's health.
    sqlx::query_scalar("SELECT COALESCE(connectionstatus='Connected' AND lastheartbeatatutc>CURRENT_TIMESTAMP-INTERVAL '90 seconds',false) FROM edgeagentbindings WHERE platformid=$1 AND resourcetype='Platform' AND dockernodeid IS NULL AND revokedatutc IS NULL AND connectionstatus<>'Revoked'")
        .bind(platform).fetch_optional(pool).await
}

#[derive(Clone)]
pub(super) struct PlatformHealthTransition {
    pub target: ReconciliationTarget,
    pub online: bool,
}

pub(super) struct HealthWorker {
    pub(super) docker: DockerClient,
    pub(super) targets: Arc<PlatformRuntimeRegistry>,
    pub(super) pool: PgPool,
    pub(super) transitions: BoundedSender<PlatformHealthTransition>,
}

pub(super) async fn platform_health_monitor(
    cancellation: CancellationToken,
    worker: HealthWorker,
) -> Result<(), std::convert::Infallible> {
    let HealthWorker {
        docker,
        targets: registry,
        pool,
        transitions,
    } = worker;
    let budget = IoBudget::new(HEALTH_CONCURRENCY.try_into().unwrap(), RuntimeWork::Health);
    let mut ticker = tokio::time::interval(Duration::from_secs(5));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut states = std::collections::HashMap::<
        (uuid::Uuid, String),
        (citadel_platforms::ConnectorKind, HealthState),
    >::new();
    loop {
        tokio::select! { ()=cancellation.cancelled()=>return Ok(()), _=ticker.tick()=>{} }
        let targets = registry.snapshot().await;
        states.retain(|key, (connector, _)| {
            targets
                .iter()
                .any(|t| t.id == key.0 && t.address == key.1 && t.connector_type == *connector)
        });
        let probes = futures_util::stream::iter(targets.iter().cloned().map(|target| {
            let docker = docker.clone();
            let budget = budget.clone();
            let cancellation = cancellation.clone();
            let pool = pool.clone();
            async move {
                let Some(_permit) = budget.enter(&cancellation).await else {
                    return (target, None);
                };
                if target.connector_type == citadel_platforms::ConnectorKind::EdgeAgent {
                    let health = match edge_health(&pool, target.id).await {
                        Ok(health) => health,
                        Err(error) => {
                            tracing::warn!(%error,"Edge health lookup failed");
                            None
                        }
                    };
                    return (target, health);
                }
                let selected;
                let runtime: &dyn PlatformHealthPort =
                    if target.connector_type == citadel_platforms::ConnectorKind::Local {
                        &docker
                    } else {
                        let Some(agent) = target.agent.as_ref() else {
                            return (target, None);
                        };
                        selected = agent.as_ref().clone();
                        &selected
                    };
                let healthy = probe_health(runtime, &cancellation, Duration::from_secs(2)).await;
                (target, Some(healthy))
            }
        }))
        .buffer_unordered(HEALTH_CONCURRENCY);
        tokio::pin!(probes);
        loop {
            let next = tokio::select! { ()=cancellation.cancelled()=>return Ok(()), next=probes.next()=>next };
            let Some((target, Some(healthy))) = next else {
                if next.is_none() {
                    break;
                }
                continue;
            };
            let (_, state) = states
                .entry((target.id, target.address.clone()))
                .or_insert_with(|| (target.connector_type, HealthState::default()));
            let Some(online) = state.observe(healthy) else {
                continue;
            };
            if transitions
                .send(PlatformHealthTransition { target, online }, &cancellation)
                .await
                .is_err()
            {
                return Ok(());
            }
        }
    }
}

pub(super) async fn readiness_probe(
    cancellation: CancellationToken,
    docker: DockerClient,
    pool: PgPool,
    readiness: Arc<Readiness>,
    metrics: Arc<Metrics>,
    interval: Duration,
) -> Result<(), std::convert::Infallible> {
    let _task = metrics.task_guard();
    let mut ticker = tokio::time::interval(interval);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            () = cancellation.cancelled() => return Ok(()),
            _ = ticker.tick() => {
                let _iteration = RuntimeWork::Readiness.start();
                let database_ready = sqlx::query_scalar::<_, i32>("SELECT 1")
                    .fetch_one(&pool)
                    .await
                    .is_ok();
                let docker_ready = docker.ping().await.is_ok();
                let setup = if readiness.is_setup() {
                    true
                } else {
                    sqlx::query_scalar::<_, bool>(
                        "SELECT initializedat IS NOT NULL FROM instancesetupstates ORDER BY id LIMIT 1"
                    )
                    .fetch_optional(&pool)
                    .await
                    .ok()
                    .flatten()
                    .unwrap_or(false)
                };
                readiness.set(database_ready, docker_ready);
                readiness.set_setup(setup);
                if !database_ready || !docker_ready {
                    metrics.readiness_failed();
                }
            }
        }
    }
}
