use super::*;

pub(super) struct InventoryReconciliationWorker {
    pub(super) targets: Arc<PlatformRuntimeRegistry>,
    pub(super) budget: IoBudget,
    pub(super) node_agent_policy:
        citadel_adapters::node_agent_reconciliation::NodeAgentReconciliationPolicy,
    pub(super) docker: DockerClient,
    pub(super) pool: PgPool,
    pub(super) local_triggers: BoundedReceiver<()>,
    pub(super) agent_triggers: BoundedReceiver<Option<uuid::Uuid>>,
    pub(super) agent_overflow: Arc<std::sync::atomic::AtomicBool>,
    pub(super) realtime: Option<RealtimeHub>,
    pub(super) interval: Duration,
    pub(super) retry_delay: Duration,
}

pub(super) async fn inventory_reconciliation(
    cancellation: CancellationToken,
    mut worker: InventoryReconciliationWorker,
) -> Result<(), std::convert::Infallible> {
    let store = PostgresInventoryProjectionStore::new(worker.pool.clone())
        .with_health_owner()
        .with_node_policy(worker.node_agent_policy.clone());
    let mut ticker = tokio::time::interval_at(
        tokio::time::Instant::now() + worker.interval,
        worker.interval,
    );
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut target_changes = worker.targets.changes();
    let mut scope = None;
    let mut agent_ids = std::collections::BTreeSet::new();
    let mut all_agents = false;

    loop {
        if target_changes.has_changed().unwrap_or(false) {
            target_changes.borrow_and_update();
            scope = None;
            // The authoritative full pass covers signals queued before it starts.
            while worker.local_triggers.try_recv().is_some() {}
            while worker.agent_triggers.try_recv().is_some() {}
            agent_ids.clear();
            all_agents = true;
        }
        if scope.is_some() {
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Ok(()),
                () = tokio::time::sleep(Duration::from_millis(500)) => {}
            }
            if matches!(scope, Some(ReconciliationTrigger::LocalEvent)) {
                while worker.local_triggers.try_recv().is_some() {}
            } else {
                while let Some(id) = worker.agent_triggers.try_recv() {
                    if let Some(id) = id {
                        agent_ids.insert(id);
                    } else {
                        all_agents = true;
                    }
                }
            }
        }

        if matches!(scope, Some(ReconciliationTrigger::AgentEvent)) {
            all_agents |= worker
                .agent_overflow
                .swap(false, std::sync::atomic::Ordering::AcqRel);
        } else if scope.is_none() {
            worker
                .agent_overflow
                .store(false, std::sync::atomic::Ordering::Release);
        }

        if let Err(error) = reconcile_inventory(
            &cancellation,
            &worker,
            &store,
            scope,
            if all_agents { None } else { Some(&agent_ids) },
        )
        .await
        {
            tracing::warn!(%error, "platform inventory reconciliation failed");
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Ok(()),
                () = tokio::time::sleep(worker.retry_delay) => {}
            }
            continue;
        }

        tokio::select! {
            biased;
            () = cancellation.cancelled() => return Ok(()),
            trigger = worker.local_triggers.recv(&cancellation) => {
                if trigger.is_none() {
                    return Ok(());
                }
                scope = Some(ReconciliationTrigger::LocalEvent);
            }
            trigger = worker.agent_triggers.recv(&cancellation) => {
                if trigger.is_none() {
                    return Ok(());
                }
                agent_ids.clear();
                all_agents = trigger.flatten().is_none();
                if let Some(id)=trigger.flatten() {agent_ids.insert(id);}
                scope = Some(ReconciliationTrigger::AgentEvent);
            }
            _ = ticker.tick() => scope = None,
            _ = target_changes.changed() => scope = None,
        }
    }
}

pub(super) async fn reconcile_inventory(
    cancellation: &CancellationToken,
    worker: &InventoryReconciliationWorker,
    store: &dyn InventoryProjectionStore,
    scope: Option<ReconciliationTrigger>,
    agent_ids: Option<&std::collections::BTreeSet<uuid::Uuid>>,
) -> Result<(), RuntimeCapabilityError> {
    let targets = worker.targets.snapshot().await;
    let mut refreshes = futures_util::stream::iter(targets.iter().cloned()).map(|target| async move {
        if cancellation.is_cancelled() {
            return Ok(());
        }
        match scope {
            Some(ReconciliationTrigger::LocalEvent)
                if !(target.connector_type == citadel_platforms::ConnectorKind::Local) =>
            {
                return Ok(());
            }
            Some(ReconciliationTrigger::AgentEvent)
                if !(target.connector_type == citadel_platforms::ConnectorKind::Agent) || agent_ids.is_some_and(|ids| !ids.contains(&target.id)) =>
            {
                return Ok(());
            }
            _ => {}
        }
        let selected_agent;
        let runtime: &dyn PlatformInventoryPort =
            if target.connector_type == citadel_platforms::ConnectorKind::Local  {
                &worker.docker
            } else if target.connector_type == citadel_platforms::ConnectorKind::Agent  {
                let Some(agent) = target.agent.as_ref() else {
                    return Ok(());
                };
                selected_agent = agent.as_ref().clone();
                &selected_agent
            } else {
                // Edge Agents use an inbound command session. That resolver is registered
                // when the Edge transport owns a live session; disconnected targets keep
                // their last bounded projection instead of being overwritten.
                return Ok(());
            };

        let Some(_permit) = worker.budget.enter(cancellation).await else { return Ok(()); };
        tracing::debug!(platform_id=%target.id, reason=?scope, "Full inventory reconciliation");
        let started_at = chrono::Utc::now();
        match collect_inventory(
            runtime,
            &InventoryCollectionTarget {
                platform_id: target.id,
                platform_type: target.platform_type,
            },
            cancellation,
        )
        .await
        {
            Ok(snapshot) => {
                let change = match store.persist(&snapshot).await {
                    Ok(change) => change,
                    Err(error) => {
                        tracing::warn!(%error, platform_id=%target.id, "Platform inventory rejected");
                        return Ok(());
                    }
                };
                if let Some(realtime) = worker.realtime.as_ref() {
                    realtime.publish_runtime_change(
                        target.id,
                        "platformInventory",
                        "reconciled",
                        change.platform_id.to_string(),
                    );
                }
            }
            Err(error) if error.kind == citadel_platforms::RuntimeErrorKind::Cancelled => {
                return Ok(());
            }
            Err(error) => {
                tracing::warn!(
                    platform_id = %target.id,
                    connector_type = %target.connector_type,
                    %error,
                    "platform inventory target failed"
                );
                if target.platform_type == citadel_platforms::PlatformKind::DockerSwarm  {
                    PostgresInventoryProjectionStore::new(worker.pool.clone())
                        .mark_swarm_stale(target.id, started_at).await?;
                }
                // Health monitoring owns availability transitions and alerts. An
                // inventory error must not bypass its failure/success thresholds.
            }
        }
        Ok::<(), RuntimeCapabilityError>(())
    }).buffer_unordered(INVENTORY_CONCURRENCY);
    while let Some(result) = refreshes.next().await {
        if let Err(error) = result {
            tracing::warn!(%error, "platform refresh failed; continuing remaining targets");
        }
    }
    Ok(())
}

pub(super) fn platform_reachable_observation(target: &ReconciliationTarget) -> AlertObservation {
    AlertObservation {
        alert_type: "PlatformUnreachable".to_owned(),
        info: serde_json::json!({
            "PlatformName": target.name,
            "Id": target.id,
            "Address": target.address,
            "HumanMessage": format!("Platform '{}' is reachable.", target.name),
        }),
        resource_id: target.id,
        resource_name: target.name.clone(),
        resource_type: "Platform".to_owned(),
        deduplication_component: "inventory".to_owned(),
        observed_at: chrono::Utc::now(),
        value: None,
        matched: false,
    }
}

pub(super) fn platform_unreachable_observation(
    target: &ReconciliationTarget,
    error: &RuntimeCapabilityError,
) -> AlertObservation {
    let message = format!(
        "Could not synchronize platform '{}': {}",
        target.name, error
    );
    AlertObservation {
        alert_type: "PlatformUnreachable".to_owned(),
        info: serde_json::json!({
            "PlatformName": target.name,
            "Id": target.id,
            "Address": target.address,
            "ErrorMessage": error.to_string(),
            "HumanMessage": message,
        }),
        resource_id: target.id,
        resource_name: target.name.clone(),
        resource_type: "Platform".to_owned(),
        deduplication_component: "inventory".to_owned(),
        observed_at: chrono::Utc::now(),
        value: None,
        matched: true,
    }
}
