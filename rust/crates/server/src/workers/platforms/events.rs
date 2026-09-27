use super::*;

pub(super) async fn event_source(
    cancellation: CancellationToken,
    docker: DockerClient,
    sender: BoundedSender<InventoryEvent>,
    metrics: Arc<Metrics>,
) -> Result<(), DockerError> {
    let _task = metrics.task_guard();
    let mut retry = Duration::from_millis(250);
    let mut since = None;
    let filters = std::collections::HashMap::from([(
        "type".to_owned(),
        [
            "container",
            "image",
            "network",
            "volume",
            "node",
            "service",
            "task",
            "secret",
            "config",
            "builder",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect(),
    )]);
    let mut connected_once = false;
    loop {
        if connected_once {
            docker.invalidate_daemon().await;
        }
        let stream = tokio::select! {
            () = cancellation.cancelled() => return Ok(()),
            result = docker.events(since, Some(&filters)) => result,
        };
        let mut stream = match stream {
            Ok(stream) => {
                if connected_once {
                    let event = InventoryEvent {
                        resource: None,
                        stream_recovered: true,
                        swarm_scope: false,
                        kind: RuntimeEventKind::Unknown,
                        source: ReconciliationTrigger::LocalEvent,
                        platform_id: None,
                        container_id: None,
                        container_state: None,
                        container_name: None,
                        container: None,
                        action: "reconnect".into(),
                        event_time_millis: None,
                    };
                    if sender.send(event, &cancellation).await.is_err() {
                        return Ok(());
                    }
                    metrics.event_enqueued();
                }
                connected_once = true;
                stream
            }
            Err(error) => {
                metrics.stream_reconnected();
                tracing::warn!(%error, "Docker event stream connection failed");
                tokio::select! {
                    () = cancellation.cancelled() => return Ok(()),
                    () = tokio::time::sleep(retry) => {}
                }
                retry = std::cmp::min(retry.saturating_mul(2), Duration::from_secs(10));
                continue;
            }
        };

        let connected_at = tokio::time::Instant::now();
        loop {
            let event = tokio::select! {
                () = cancellation.cancelled() => return Ok(()),
                event = stream.next() => event,
            };
            match event {
                Some(Ok(event)) => {
                    RuntimeWork::DockerRawEvent.units(1);
                    if event.time > 0 {
                        // Docker's `since` bound is inclusive. Replaying the final second
                        // after reconnect is preferable to dropping sibling events; the
                        // projection trigger is coalesced and persistence is idempotent.
                        since = Some(event.time);
                    }
                    let Some(kind) =
                        citadel_adapters::connectors::docker::events::normalize(&event)
                    else {
                        if event.resource_type == "container" {
                            RuntimeWork::ContainerEventIgnored.units(1);
                        }
                        continue;
                    };
                    let event_time_millis = if event.time_nano > 0 {
                        Some(u128::try_from(event.time_nano).unwrap_or_default() / 1_000_000)
                    } else if event.time > 0 {
                        Some(u128::try_from(event.time).unwrap_or_default() * 1_000)
                    } else {
                        Some(chrono::Utc::now().timestamp_millis().max(0) as u128)
                    };
                    let event = InventoryEvent {
                        resource: None,
                        stream_recovered: false,
                        swarm_scope: event.scope == "swarm",
                        kind,
                        source: ReconciliationTrigger::LocalEvent,
                        platform_id: None,
                        container_id: Some(event.actor.id),
                        container_state: None,
                        container_name: None,
                        container: None,
                        action: event.action,
                        event_time_millis,
                    };
                    if sender.send(event, &cancellation).await.is_err() {
                        return Ok(());
                    }
                    metrics.event_enqueued();
                }
                Some(Err(error)) => {
                    metrics.stream_reconnected();
                    tracing::warn!(%error, "Docker event stream interrupted");
                    break;
                }
                None => {
                    metrics.stream_reconnected();
                    break;
                }
            }
        }
        // Opening an HTTP stream is not evidence of a healthy connection.
        // EOF and decode failures must back off too, even after HTTP 200.
        if connected_at.elapsed() >= Duration::from_secs(30) {
            retry = Duration::from_millis(250);
        }
        if wait_to_reconnect(&cancellation, retry).await {
            return Ok(());
        }
        retry = retry.saturating_mul(2).min(Duration::from_secs(10));
    }
}

pub(super) async fn agent_event_source(
    cancellation: CancellationToken,
    platform: uuid::Uuid,
    _agent: AgentClient,
    targets: Arc<PlatformRuntimeRegistry>,
    sender: BoundedSender<InventoryEvent>,
    metrics: Arc<Metrics>,
    reconnect_delay: Duration,
) -> Result<(), std::convert::Infallible> {
    let _task = metrics.task_guard();
    loop {
        let (platform_id, agent) = match agent_subscription(&targets, platform).await {
            Ok(Some(target)) => target,
            result => {
                if let Err(error) = result {
                    tracing::warn!(%error, "Agent Platform lookup for events failed");
                }
                if wait_to_reconnect(&cancellation, reconnect_delay).await {
                    return Ok(());
                }
                continue;
            }
        };
        let stream = agent.stream_daemon_events(&cancellation).await;
        let mut stream = match stream {
            Ok(stream) => stream,
            Err(error) => {
                if cancellation.is_cancelled() {
                    return Ok(());
                }
                metrics.agent_stream_reconnected();
                tracing::warn!(%error, "Agent daemon event stream connection failed");
                if wait_to_reconnect(&cancellation, reconnect_delay).await {
                    return Ok(());
                }
                continue;
            }
        };
        let recovered = InventoryEvent {
            resource: None,
            stream_recovered: true,
            swarm_scope: false,
            kind: RuntimeEventKind::Unknown,
            source: ReconciliationTrigger::AgentEvent,
            platform_id: Some(platform_id),
            container_id: None,
            container_state: None,
            container_name: None,
            container: None,
            action: "reconnect".into(),
            event_time_millis: None,
        };
        if sender.send(recovered, &cancellation).await.is_err() {
            return Ok(());
        }
        metrics.event_enqueued();
        let changed = targets.reconfigured(platform_id, agent.address());
        tokio::pin!(changed);
        loop {
            let event = tokio::select! {
                biased;
                () = cancellation.cancelled() => return Ok(()),
                () = &mut changed => break,
                event = stream.next() => event,
            };
            match event {
                Some(Ok(event)) => {
                    RuntimeWork::AgentDaemonEvent.units(1);
                    if sender
                        .send(
                            InventoryEvent {
                                resource: event.resource,
                                stream_recovered: false,
                                swarm_scope: event.swarm_scope,
                                kind: event.kind,
                                source: ReconciliationTrigger::AgentEvent,
                                platform_id: Some(platform_id),
                                container_id: event.container_id,
                                container_state: event.container_state,
                                container_name: event.container_name,
                                container: event.container,
                                action: event.action,
                                // The current Agent wire contract has no daemon timestamp.
                                // Stamp receipt before queueing, not after backlog/lock waits.
                                event_time_millis: Some(
                                    chrono::Utc::now().timestamp_millis().max(0) as u128,
                                ),
                            },
                            &cancellation,
                        )
                        .await
                        .is_err()
                    {
                        return Ok(());
                    }
                    metrics.event_enqueued();
                }
                Some(Err(error)) => {
                    metrics.agent_stream_reconnected();
                    tracing::warn!(%error, "Agent daemon event stream interrupted");
                    break;
                }
                None => {
                    metrics.agent_stream_reconnected();
                    break;
                }
            }
        }
        if wait_to_reconnect(&cancellation, reconnect_delay).await {
            return Ok(());
        }
    }
}

pub(super) struct EventWorker {
    pub(super) event_refresh: EventRefreshSender,
    pub(super) metrics: Arc<Metrics>,
    pub(super) docker: DockerClient,
    pub(super) pool: PgPool,
    pub(super) realtime: Option<RealtimeHub>,
    pub(super) targets: Arc<PlatformRuntimeRegistry>,
    pub(super) local_reconciliation: BoundedSender<()>,
    pub(super) agent_reconciliation: AgentReconciliationSignal,
}

pub(super) async fn event_consumer(
    cancellation: CancellationToken,
    receiver: BoundedReceiver<InventoryEvent>,
    worker: EventWorker,
) -> Result<(), std::convert::Infallible> {
    let EventWorker {
        event_refresh,
        metrics,
        docker,
        pool,
        realtime,
        targets,
        local_reconciliation,
        agent_reconciliation,
    } = worker;
    let _task = metrics.task_guard();
    let stream = futures_util::stream::unfold(receiver, |mut receiver| {
        let cancellation = &cancellation;
        async move {
            receiver
                .recv(cancellation)
                .await
                .map(|event| (event, receiver))
        }
    })
    .fuse();
    tokio::pin!(stream);
    let mut pending = None;
    loop {
        let event = match if pending.is_some() {
            pending.take()
        } else {
            stream.next().await
        } {
            Some(event) => event,
            None => return Ok(()),
        };
        if state_event(&event) {
            let Some((batch, carry)) = super::super::container_batch::gather(
                event,
                stream.as_mut(),
                same_state_scope,
                &cancellation,
            )
            .await
            else {
                return Ok(());
            };
            pending = carry;
            for event in &batch {
                record_consumed(&metrics, event);
            }
            apply_state_batch(
                batch,
                &pool,
                realtime.as_ref(),
                &targets,
                &event_refresh,
                &cancellation,
            )
            .await;
            continue;
        }
        record_consumed(&metrics, &event);
        if event.stream_recovered {
            queue_stream_recovery(&event, &local_reconciliation, &agent_reconciliation);
            continue;
        }
        let outcome = match apply_container_event(
            &event,
            &docker,
            &pool,
            realtime.as_ref(),
            &cancellation,
            &targets,
        )
        .await
        {
            Ok(true) => DeltaOutcome::Applied,
            Ok(false) => DeltaOutcome::Unavailable,
            Err(error) => {
                RuntimeWork::ContainerEventApply.failures(1);
                tracing::warn!(%error, "Container event update failed; requesting only its resource scope");
                DeltaOutcome::Unavailable
            }
        };
        request_event_refresh(&event, outcome, &targets, &event_refresh, &cancellation).await;
    }
}

fn state_event(event: &InventoryEvent) -> bool {
    !event.stream_recovered
        && matches!(event.kind, RuntimeEventKind::Container(change) if change.state_delta().is_some())
        && event.container_id.as_ref().is_some_and(|id| !id.is_empty())
}

fn same_state_scope(first: &InventoryEvent, next: &InventoryEvent) -> bool {
    state_event(next) && first.source == next.source && first.platform_id == next.platform_id
}

fn record_consumed(metrics: &Metrics, event: &InventoryEvent) {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let lag = event
        .event_time_millis
        .map_or(0, |time| now.saturating_sub(time))
        .min(u128::from(u64::MAX)) as u64;
    metrics.event_consumed(lag);
}

async fn apply_state_batch(
    batch: Vec<InventoryEvent>,
    pool: &PgPool,
    realtime: Option<&RealtimeHub>,
    targets: &PlatformRuntimeRegistry,
    sender: &EventRefreshSender,
    cancellation: &CancellationToken,
) {
    use super::super::container_batch;
    use citadel_platforms::jobs::ContainerStateDelta;
    let source = batch[0].source;
    let platform = batch[0].platform_id;
    let batch = container_batch::coalesce(
        batch,
        |event| event.container_id.clone().unwrap(),
        |event| event.event_time_millis,
    );
    let deltas: Vec<_> = batch
        .iter()
        .map(|event| {
            let RuntimeEventKind::Container(change) = event.kind else {
                unreachable!()
            };
            ContainerStateDelta {
                docker_id: event.container_id.clone().unwrap(),
                state: change.state_delta().unwrap(),
                observed_at: event
                    .event_time_millis
                    .and_then(|time| i64::try_from(time / 1000).ok())
                    .unwrap_or_else(|| chrono::Utc::now().timestamp()),
                observed_at_millis: event
                    .event_time_millis
                    .and_then(|time| i64::try_from(time).ok())
                    .unwrap_or_else(|| chrono::Utc::now().timestamp_millis()),
            }
        })
        .collect();
    let snapshot = targets.snapshot().await;
    for target in snapshot.iter().filter(|target| match source {
        ReconciliationTrigger::LocalEvent => {
            target.connector_type == citadel_platforms::ConnectorKind::Local
        }
        ReconciliationTrigger::AgentEvent => {
            target.connector_type == citadel_platforms::ConnectorKind::Agent
                && platform == Some(target.id)
        }
    }) {
        let _event = RuntimeWork::ContainerEventApply.start();
        let _projection = RuntimeWork::ContainerEventProjection.start();
        let unavailable = match citadel_adapters::persistence::postgres::platforms::status::container_state_deltas_committed(
            pool, target.id, None, &deltas,
        ).await {
            Ok(results) => {
                RuntimeWork::ContainerEventProjection.units(results.iter().filter(|r| r.changed).count() as u64);
                container_batch::publish(realtime, target.id, &results);
                results.iter().any(|r| !r.accepted)
            }
            Err(error) => {
                RuntimeWork::ContainerEventApply.failures(1);
                tracing::warn!(%error, platform_id=%target.id, "Container state batch failed; requesting container scope");
                true
            }
        };
        container_batch::refresh(
            target.id,
            target.platform_type == citadel_platforms::PlatformKind::DockerSwarm,
            unavailable,
            sender,
            cancellation,
        )
        .await;
    }
}

pub(super) async fn apply_container_event(
    event: &InventoryEvent,
    docker: &DockerClient,
    pool: &PgPool,
    realtime: Option<&RealtimeHub>,
    cancellation: &CancellationToken,
    targets: &PlatformRuntimeRegistry,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let RuntimeEventKind::Container(change) = event.kind else {
        return apply_resource_event(event, docker, pool, realtime, cancellation, targets).await;
    };
    let Some(id) = event.container_id.as_deref().filter(|id| !id.is_empty()) else {
        return Ok(false);
    };
    let _event = RuntimeWork::ContainerEventApply.start();
    let destroyed = change == ContainerChange::Tombstone;
    let state_delta = change.state_delta();
    let observed_at = event
        .event_time_millis
        .and_then(|millis| i64::try_from(millis / 1_000).ok())
        .unwrap_or_else(|| chrono::Utc::now().timestamp());
    let (state, name, container) = if destroyed || state_delta.is_some() {
        (None, None, None)
    } else if matches!(event.source, ReconciliationTrigger::LocalEvent) {
        let observed = tokio::select! {
            () = cancellation.cancelled() => return Ok(true),
            result = docker.container_event_observation(id) => result?,
        };
        let Some(mut container) = observed else {
            return Ok(false);
        };
        if let Some(state) = change.state() {
            container.state = state.to_owned();
        }
        (
            Some(container.state.clone()),
            Some(container.name.clone()),
            Some(container),
        )
    } else {
        if event.container_state.is_none()
            || event
                .container
                .as_ref()
                .is_none_or(|container| container.id != id || container.image_id.is_empty())
        {
            return Ok(false);
        }
        (
            event.container_state.clone(),
            event.container_name.clone(),
            event.container.clone(),
        )
    };
    let mut handled = false;
    let snapshot = targets.snapshot().await;
    for target in snapshot.iter() {
        let matches = event.platform_id.map_or_else(
            || target.connector_type == citadel_platforms::ConnectorKind::Local,
            |id| {
                id == target.id
                    && (target.connector_type == citadel_platforms::ConnectorKind::Agent)
            },
        );
        if !matches {
            continue;
        }
        let _projection = RuntimeWork::ContainerEventProjection.start();
        let updated = if let Some(state) = state_delta {
            use citadel_platforms::jobs::{ContainerStateDelta, ProjectionChange};
            let result = citadel_adapters::persistence::postgres::platforms::status::container_state_deltas_committed(
                pool, target.id, None, &[ContainerStateDelta {
                    docker_id: id.to_owned(), state, observed_at,
                    observed_at_millis: event.event_time_millis
                        .and_then(|time| i64::try_from(time).ok())
                        .unwrap_or_else(|| chrono::Utc::now().timestamp_millis()),
                }],
            ).await?.remove(0);
            if result.accepted {
                ProjectionChange::committed(result.changed)
            } else {
                ProjectionChange::Unavailable
            }
        } else if let Some(container) = &container {
            citadel_adapters::persistence::postgres::platforms::status::container_observation_committed(
                pool,
                target.id,
                None,
                container,
                observed_at,
            )
            .await?
        } else {
            citadel_adapters::persistence::postgres::platforms::status::container_event_committed_at(
                pool,
                target.id,
                None,
                id,
                state.as_deref(),
                name.as_deref(),
                observed_at,
                event.event_time_millis.and_then(|time| i64::try_from(time).ok())
                    .unwrap_or_else(|| chrono::Utc::now().timestamp_millis()),
            )
            .await?
        };
        RuntimeWork::ContainerEventProjection.units(u64::from(updated.changed()));
        // A tombstone for an already absent row is handled without enumeration.
        handled |= updated.accepted() || destroyed;
        if updated.changed()
            && let Some(hub) = realtime
        {
            hub.publish_container_observation(target.id, &event.action, id.to_owned());
        }
    }
    Ok(handled)
}

pub(super) fn queue_stream_recovery(
    event: &InventoryEvent,
    local: &BoundedSender<()>,
    agent: &AgentReconciliationSignal,
) {
    // Only the subscription can report stream recovery. A daemon action string
    // (even "reconnect") cannot upgrade normal event work to full inventory.
    if !event.stream_recovered {
        return;
    }
    RuntimeWork::InventoryRequestReconnect.units(1);
    match event.source {
        ReconciliationTrigger::LocalEvent => match local.try_send(()) {
            Ok(()) => {}
            Err(citadel_runtime::QueueSendError::Full(_)) => {
                RuntimeWork::InventoryRequestCoalesced.units(1);
            }
            Err(_) => RuntimeWork::InventoryRequestCoalesced.failures(1),
        },
        ReconciliationTrigger::AgentEvent => {
            agent.request(event.platform_id);
        }
    }
}

async fn request_event_refresh(
    event: &InventoryEvent,
    outcome: DeltaOutcome,
    targets: &PlatformRuntimeRegistry,
    sender: &EventRefreshSender,
    cancellation: &CancellationToken,
) {
    let snapshot = targets.snapshot().await;
    for target in snapshot.iter() {
        let matches = match event.source {
            ReconciliationTrigger::LocalEvent => {
                target.connector_type == citadel_platforms::ConnectorKind::Local
            }
            ReconciliationTrigger::AgentEvent => {
                target.connector_type == citadel_platforms::ConnectorKind::Agent
                    && event.platform_id == Some(target.id)
            }
        };
        if !matches {
            continue;
        }
        let decision = event_decision(
            event.kind,
            target.platform_type == citadel_platforms::PlatformKind::DockerSwarm,
            event.swarm_scope,
            outcome,
        );
        let refresh = match decision {
            ReconciliationDecision::Reconcile(scope) => EventRefresh::Resource(scope),
            ReconciliationDecision::SwarmDirty => EventRefresh::Swarm,
            ReconciliationDecision::None | ReconciliationDecision::ApplyDelta => continue,
        };
        if sender
            .send(
                ScopedEventRequest {
                    platform_id: target.id,
                    refresh,
                },
                cancellation,
            )
            .await
            .is_err()
        {
            return;
        }
        // A missed task-container delta also invalidates manager relationships.
        // Keep both finite scopes; neither can request a platform scan.
        if target.platform_type == citadel_platforms::PlatformKind::DockerSwarm
            && matches!(event.kind, RuntimeEventKind::Container(_))
            && refresh != EventRefresh::Swarm
            && sender
                .send(
                    ScopedEventRequest {
                        platform_id: target.id,
                        refresh: EventRefresh::Swarm,
                    },
                    cancellation,
                )
                .await
                .is_err()
        {
            return;
        }
    }
}

async fn apply_resource_event(
    event: &InventoryEvent,
    docker: &DockerClient,
    pool: &PgPool,
    realtime: Option<&RealtimeHub>,
    cancellation: &CancellationToken,
    targets: &PlatformRuntimeRegistry,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    use citadel_platforms::jobs::{ProjectionKind, SnapshotGeneration};
    let kind = match event.kind {
        RuntimeEventKind::Image(_) => ProjectionKind::Images,
        RuntimeEventKind::Network(_) => ProjectionKind::Networks,
        RuntimeEventKind::Volume(_) => ProjectionKind::Volumes,
        _ => return Ok(false),
    };
    let snapshot = targets.snapshot().await;
    let store = PostgresInventoryProjectionStore::new(pool.clone());
    let mut observed = event.resource.clone();
    let mut handled = false;
    for target in snapshot.iter().filter(|target| {
        event.platform_id.map_or_else(
            || target.connector_type == citadel_platforms::ConnectorKind::Local,
            |id| {
                target.id == id && target.connector_type == citadel_platforms::ConnectorKind::Agent
            },
        )
    }) {
        let generation = SnapshotGeneration::capture(target.id, None, kind).await;
        if observed.is_none() && matches!(event.source, ReconciliationTrigger::LocalEvent) {
            let Some(id) = event.container_id.as_deref().filter(|id| !id.is_empty()) else {
                return Ok(false);
            };
            observed = docker
                .resource_event_observation(event.kind, id, cancellation)
                .await?;
        }
        let Some(delta) = observed.as_ref().filter(|d| d.valid_for(event.kind)) else {
            return Ok(false);
        };
        let change = store.persist_delta(target.id, delta, &generation).await?;
        handled |= change.accepted();
        if change.changed()
            && let Some(hub) = realtime
        {
            hub.publish_resource_observation(target.id, None, &event.action, delta);
        }
    }
    Ok(handled)
}

#[cfg(test)]
mod batching_tests {
    use super::*;

    fn event(change: ContainerChange) -> InventoryEvent {
        InventoryEvent {
            resource: None,
            stream_recovered: false,
            swarm_scope: false,
            kind: RuntimeEventKind::Container(change),
            source: ReconciliationTrigger::LocalEvent,
            platform_id: None,
            container_id: Some("fixture".into()),
            container_state: None,
            container_name: None,
            container: None,
            action: String::new(),
            event_time_millis: Some(1000),
        }
    }

    #[tokio::test]
    async fn metadata_tombstone_recovery_and_scope_changes_are_ordering_barriers() {
        for variant in 0..7 {
            let first = event(ContainerChange::Exited);
            let mut barrier = event(ContainerChange::Running);
            match variant {
                0 => barrier.kind = RuntimeEventKind::Container(ContainerChange::Created),
                1 => barrier.kind = RuntimeEventKind::Container(ContainerChange::Observe),
                2 => barrier.kind = RuntimeEventKind::Container(ContainerChange::Tombstone),
                3 => barrier.stream_recovered = true,
                4 => barrier.platform_id = Some(uuid::Uuid::now_v7()),
                5 => barrier.source = ReconciliationTrigger::AgentEvent,
                _ => barrier.container_id = None,
            }
            let mut stream = Box::pin(futures_util::stream::iter([
                barrier,
                event(ContainerChange::Running),
            ]));
            let (batch, carry) = super::super::super::container_batch::gather(
                first,
                stream.as_mut(),
                same_state_scope,
                &CancellationToken::new(),
            )
            .await
            .unwrap();
            assert_eq!(batch.len(), 1);
            assert!(carry.is_some());
            assert!(state_event(&stream.next().await.unwrap()));
        }
    }
}
