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
                        source: ReconciliationTrigger::LocalEvent,
                        platform_id: None,
                        container_id: None,
                        container_state: None,
                        container_name: None,
                        container: None,
                        resource_type: "daemon".into(),
                        action: "reconnect".into(),
                        event_time_millis: None,
                    };
                    if sender.send(event, &cancellation).await.is_err() {
                        return Ok(());
                    }
                    metrics.event_enqueued();
                }
                connected_once = true;
                retry = Duration::from_millis(250);
                stream
            }
            Err(error) => {
                metrics.stream_reconnected();
                tracing::warn!(%error, "Docker event stream connection failed");
                tokio::select! {
                    () = cancellation.cancelled() => return Ok(()),
                    () = tokio::time::sleep(retry) => {}
                }
                retry = std::cmp::min(retry.saturating_mul(2), Duration::from_secs(5));
                continue;
            }
        };

        loop {
            let event = tokio::select! {
                () = cancellation.cancelled() => return Ok(()),
                event = stream.next() => event,
            };
            match event {
                Some(Ok(event)) => {
                    if event.time > 0 {
                        // Docker's `since` bound is inclusive. Replaying the final second
                        // after reconnect is preferable to dropping sibling events; the
                        // projection trigger is coalesced and persistence is idempotent.
                        since = Some(event.time);
                    }
                    let event_time_millis = if event.time_nano > 0 {
                        Some(u128::try_from(event.time_nano).unwrap_or_default() / 1_000_000)
                    } else if event.time > 0 {
                        Some(u128::try_from(event.time).unwrap_or_default() * 1_000)
                    } else {
                        None
                    };
                    let event = InventoryEvent {
                        source: ReconciliationTrigger::LocalEvent,
                        platform_id: None,
                        container_id: Some(event.actor.id),
                        container_state: None,
                        container_name: None,
                        container: None,
                        resource_type: event.resource_type,
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
            source: ReconciliationTrigger::AgentEvent,
            platform_id: Some(platform_id),
            container_id: None,
            container_state: None,
            container_name: None,
            container: None,
            resource_type: "daemon".into(),
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
                    if sender
                        .send(
                            InventoryEvent {
                                source: ReconciliationTrigger::AgentEvent,
                                platform_id: Some(platform_id),
                                container_id: event.container_id,
                                container_state: event.container_state,
                                container_name: event.container_name,
                                container: event.container,
                                resource_type: event.resource_type.to_owned(),
                                action: event.action,
                                event_time_millis: None,
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
    mut receiver: BoundedReceiver<InventoryEvent>,
    worker: EventWorker,
) -> Result<(), std::convert::Infallible> {
    let EventWorker {
        metrics,
        docker,
        pool,
        realtime,
        targets,
        local_reconciliation,
        agent_reconciliation,
    } = worker;
    let _task = metrics.task_guard();
    loop {
        let event = match receiver.recv(&cancellation).await {
            Some(event) => event,
            None => return Ok(()),
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let lag = event
            .event_time_millis
            .map_or(0, |event_time| now.saturating_sub(event_time))
            .min(u128::from(u64::MAX)) as u64;
        metrics.event_consumed(lag);
        match apply_container_event(
            &event,
            &docker,
            &pool,
            realtime.as_ref(),
            &cancellation,
            &targets,
        )
        .await
        {
            Ok(true) => {
                continue;
            }
            Ok(false) => {}
            Err(error) => {
                tracing::warn!(%error, "Container event update failed; scheduling reconciliation")
            }
        }
        queue_inventory_reconciliation(&event, &local_reconciliation, &agent_reconciliation);
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
    if !event.resource_type.eq_ignore_ascii_case("container") {
        return Ok(false);
    }
    let Some(id) = event.container_id.as_deref().filter(|id| !id.is_empty()) else {
        return Ok(false);
    };
    if event.action.starts_with("exec_") || event.action == "attach" || event.action == "top" {
        return Ok(true);
    }
    let destroyed = event.action.eq_ignore_ascii_case("destroy");
    let (state, name, container) = if destroyed {
        (None, None, None)
    } else if matches!(event.source, ReconciliationTrigger::LocalEvent) {
        let inspected = tokio::select! {
            () = cancellation.cancelled() => return Ok(true),
            result = docker.inspect_container_document(id) => result?,
        };
        let container = citadel_adapters::docker::container_observation(inspected)?;
        (
            Some(container.state.clone()),
            Some(container.name.clone()),
            Some(container),
        )
    } else {
        if event.container_state.is_none() {
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
            || target.connector_type.eq_ignore_ascii_case("Local"),
            |id| id == target.id && target.connector_type.eq_ignore_ascii_case("Agent"),
        );
        if !matches {
            continue;
        }
        let updated = if let Some(container) = &container {
            citadel_adapters::resource_status_store::container_observation(
                pool,
                target.id,
                None,
                container,
                chrono::Utc::now().timestamp(),
            )
            .await?
        } else {
            citadel_adapters::resource_status_store::container_event(
                pool,
                target.id,
                None,
                id,
                state.as_deref(),
                name.as_deref(),
                chrono::Utc::now().timestamp(),
            )
            .await?
        };
        // Swarm task/service relationships require a complete desired set before
        // replica counts can fall. Keep the full fallback for Swarm container events.
        handled |= updated && !target.platform_type.eq_ignore_ascii_case("DockerSwarm");
        if updated && let Some(hub) = realtime {
            hub.publish_runtime_change(target.id, "container", &event.action, id.to_owned());
        }
    }
    Ok(handled)
}

pub(super) fn queue_inventory_reconciliation(
    event: &InventoryEvent,
    local: &BoundedSender<()>,
    agent: &AgentReconciliationSignal,
) {
    if event.action != "reconnect"
        && !triggers_inventory_reconciliation(&event.resource_type, &event.action)
    {
        return;
    }
    tracing::debug!(source=?event.source, resource=%event.resource_type, action=%event.action, "Full inventory refresh requested");
    match event.source {
        ReconciliationTrigger::LocalEvent => {
            let _ = local.try_send(());
        }
        ReconciliationTrigger::AgentEvent => {
            agent.request(event.platform_id);
        }
    }
}
