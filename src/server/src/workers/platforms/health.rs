use super::*;

#[derive(Default)]
pub(crate) struct HealthState {
    pub(crate) online: Option<bool>,
    pub(crate) successes: u8,
    pub(crate) failures: u8,
}
impl HealthState {
    pub(crate) fn observe(&mut self, healthy: bool) -> Option<bool> {
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

pub(crate) async fn probe_health(
    runtime: &dyn PlatformHealthPort,
    token: &CancellationToken,
    timeout: Duration,
) -> bool {
    matches!(
        tokio::time::timeout(timeout, runtime.probe(token)).await,
        Ok(Ok(_))
    )
}

#[derive(Clone)]
pub(crate) struct PlatformHealthTransition {
    pub target: ReconciliationTarget,
    pub online: bool,
    pub edge_binding: Option<uuid::Uuid>,
}

pub(crate) struct HealthWorker {
    pub(crate) docker: DockerClient,
    pub(crate) targets: Arc<PlatformRuntimeRegistry>,
    pub(crate) edge: Arc<dyn citadel_platforms::jobs::background::EdgeHealthReader>,
    pub(crate) transitions: BoundedSender<PlatformHealthTransition>,
}

pub(crate) async fn platform_health_monitor(
    cancellation: CancellationToken,
    worker: HealthWorker,
) -> Result<(), std::convert::Infallible> {
    let HealthWorker {
        docker,
        targets: registry,
        edge,
        transitions,
    } = worker;
    let budget = IoBudget::new(HEALTH_CONCURRENCY.try_into().unwrap(), RuntimeWork::Health);
    let mut ticker = tokio::time::interval(Duration::from_secs(5));
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut states = std::collections::HashMap::<
        (uuid::Uuid, String, Option<uuid::Uuid>),
        (citadel_platforms::ConnectorKind, HealthState),
    >::new();
    loop {
        if let Some(job) = citadel_runtime::jobs::JobHandle::current() {
            job.policy("Every 5 seconds; probes bounded to 2 seconds each", None);
            job.waiting("Next health cycle", None);
        }
        tokio::select! { ()=cancellation.cancelled()=>return Ok(()), _=ticker.tick()=>{} }
        let observation = citadel_runtime::jobs::JobHandle::iteration();
        let mut cycle_failed = false;
        let targets = registry.snapshot().await;
        let edge_ids: Vec<_> = targets
            .iter()
            .filter(|t| t.connector_type == citadel_platforms::ConnectorKind::EdgeAgent)
            .map(|t| t.id)
            .collect();
        let edge_states = if edge_ids.is_empty() {
            std::collections::HashMap::new()
        } else {
            let result = tokio::select! {
                ()=cancellation.cancelled()=>return Ok(()),
                result=edge.health(&edge_ids)=>result,
            };
            match result {
                Ok(rows) => rows.into_iter().map(|row| (row.platform, row)).collect(),
                Err(error) => {
                    cycle_failed = true;
                    tracing::warn!(%error,"Edge health lookup failed");
                    std::collections::HashMap::new()
                }
            }
        };
        states.retain(|key, (connector, _)| {
            targets
                .iter()
                .any(|t| t.id == key.0 && t.address == key.1 && t.connector_type == *connector)
                && (cycle_failed || key.2 == edge_states.get(&key.0).map(|row| row.binding))
        });
        let probes = futures_util::stream::iter(targets.iter().cloned().map(|target| {
            let docker = docker.clone();
            let budget = budget.clone();
            let cancellation = cancellation.clone();
            let edge_health = edge_states.get(&target.id).copied();
            async move {
                if target.connector_type == citadel_platforms::ConnectorKind::EdgeAgent {
                    return (
                        target,
                        edge_health.map(|row| (row.healthy, Some(row.binding))),
                    );
                }
                let Some(_permit) = budget.enter(&cancellation).await else {
                    return (target, None);
                };
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
                (target, Some((healthy, None)))
            }
        }))
        .buffer_unordered(HEALTH_CONCURRENCY);
        tokio::pin!(probes);
        loop {
            let next = tokio::select! { ()=cancellation.cancelled()=>return Ok(()), next=probes.next()=>next };
            let Some((target, Some((healthy, edge_binding)))) = next else {
                if next.is_none() {
                    break;
                }
                continue;
            };
            let (_, state) = states
                .entry((target.id, target.address.clone(), edge_binding))
                .or_insert_with(|| (target.connector_type, HealthState::default()));
            let Some(online) = state.observe(healthy) else {
                continue;
            };
            if transitions
                .send(
                    PlatformHealthTransition {
                        target,
                        online,
                        edge_binding,
                    },
                    &cancellation,
                )
                .await
                .is_err()
            {
                return Ok(());
            }
        }
        if let Some(i) = observation {
            i.finish(
                if cycle_failed {
                    citadel_runtime::jobs::JobOutcome::Failed
                } else if targets.is_empty() {
                    citadel_runtime::jobs::JobOutcome::NoWork
                } else {
                    citadel_runtime::jobs::JobOutcome::Productive
                },
                (!cycle_failed).then_some((targets.len() as u64, "target checks")),
            );
        }
    }
}
