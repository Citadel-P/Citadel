use super::*;
use citadel_platforms::jobs::{RecoveryReason, RuntimeRecoveryCoordinator};

pub(super) struct InventoryReconciliationWorker {
    pub(super) targets: Arc<PlatformRuntimeRegistry>,
    pub(super) refreshes: EventRefreshSender,
    pub(super) local_triggers: BoundedReceiver<()>,
    pub(super) agent_triggers: BoundedReceiver<Option<uuid::Uuid>>,
    pub(super) agent_overflow: Arc<std::sync::atomic::AtomicBool>,
}

pub(super) async fn inventory_reconciliation(
    cancellation: CancellationToken,
    mut worker: InventoryReconciliationWorker,
) -> Result<(), std::convert::Infallible> {
    let safety = Duration::from_secs(6 * 3600);
    let mut ticker = tokio::time::interval_at(tokio::time::Instant::now() + safety, safety);
    ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    let mut changes = worker.targets.changes();
    let mut reason = RecoveryReason::Bootstrap;
    let mut scope = None;
    let mut agent_ids = std::collections::BTreeSet::new();
    let mut all_agents = true;
    loop {
        // Scoped ingress already debounces. Forward reconnects immediately so
        // their dirty generation invalidates an in-flight metadata read.
        if matches!(scope, Some(ReconciliationTrigger::LocalEvent)) {
            while worker.local_triggers.try_recv().is_some() {}
        }
        if matches!(scope, Some(ReconciliationTrigger::AgentEvent)) {
            while let Some(id) = worker.agent_triggers.try_recv() {
                if let Some(id) = id {
                    agent_ids.insert(id);
                } else {
                    all_agents = true;
                }
            }
            all_agents |= worker
                .agent_overflow
                .swap(false, std::sync::atomic::Ordering::AcqRel);
        }
        for target in worker.targets.snapshot().await.iter() {
            use citadel_platforms::ConnectorKind;
            if !matches!(
                target.connector_type,
                ConnectorKind::Local | ConnectorKind::Agent
            ) {
                continue;
            }
            match scope {
                Some(ReconciliationTrigger::LocalEvent)
                    if target.connector_type != ConnectorKind::Local =>
                {
                    continue;
                }
                Some(ReconciliationTrigger::AgentEvent)
                    if target.connector_type != ConnectorKind::Agent
                        || (!all_agents && !agent_ids.contains(&target.id)) =>
                {
                    continue;
                }
                _ => {}
            }
            if let Some(refresh) = RuntimeRecoveryCoordinator::request(reason, target.platform_type)
            {
                tracing::debug!(platform_id=%target.id, ?reason, "Requesting scoped recovery");
                if worker
                    .refreshes
                    .send(
                        ScopedEventRequest {
                            platform_id: target.id,
                            refresh,
                        },
                        &cancellation,
                    )
                    .await
                    .is_err()
                {
                    return Ok(());
                }
            }
        }
        tokio::select! {
            biased;
            () = cancellation.cancelled() => return Ok(()),
            trigger = worker.local_triggers.recv(&cancellation) => {
                if trigger.is_none() { return Ok(()); }
                scope = Some(ReconciliationTrigger::LocalEvent);
                reason = RecoveryReason::Reconnect;
            }
            trigger = worker.agent_triggers.recv(&cancellation) => {
                let Some(id) = trigger else { return Ok(()); };
                agent_ids.clear();
                all_agents = id.is_none();
                agent_ids.extend(id);
                scope = Some(ReconciliationTrigger::AgentEvent);
                reason = RecoveryReason::Online;
            }
            _ = ticker.tick() => { scope = None; reason = RecoveryReason::Safety; }
            _ = changes.changed() => { scope = None; reason = RecoveryReason::Bootstrap; }
        }
    }
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
