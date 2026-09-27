//! Confirmed health transitions own lifecycle effects, never individual probes.
use super::*;
use std::{collections::BTreeMap, future::Future};
use tokio::time::Instant;

const RETRY: Duration = Duration::from_secs(5);
const DEPLOYMENT_SAFETY: Duration = Duration::from_secs(5 * 60);

pub(super) struct LifecycleWorker {
    pub targets: Arc<PlatformRuntimeRegistry>,
    pub pool: PgPool,
    pub realtime: Option<RealtimeHub>,
    pub refreshes: EventRefreshSender,
    pub alerts: Arc<dyn AlertEventSink>,
}

pub(super) struct Progress {
    transition: PlatformHealthTransition,
    persisted: bool,
    recovery_requested: bool,
}
impl Progress {
    fn new(transition: PlatformHealthTransition) -> Self {
        Self {
            transition,
            persisted: false,
            recovery_requested: false,
        }
    }
}
struct Pending {
    progress: Progress,
    due: Instant,
}

pub(super) async fn platform_lifecycle(
    cancellation: CancellationToken,
    receiver: BoundedReceiver<PlatformHealthTransition>,
    worker: LifecycleWorker,
) -> Result<(), std::convert::Infallible> {
    run_transitions(&cancellation, receiver, |mut progress| {
        let worker = &worker;
        let cancellation = &cancellation;
        async move {
            let result = worker.apply(&mut progress, cancellation).await;
            (progress, result)
        }
    })
    .await;
    Ok(())
}

impl LifecycleWorker {
    async fn apply(
        &self,
        progress: &mut Progress,
        cancel: &CancellationToken,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let target = &progress.transition.target;
        let online = progress.transition.online;
        // Discard delayed observations after removal or connector reconfiguration.
        if !self.targets.snapshot().await.iter().any(|current| {
            current.id == target.id
                && current.address == target.address
                && current.connector_type == target.connector_type
        }) {
            return Ok(());
        }
        let _work = RuntimeWork::PlatformLifecycle.start();
        if !progress.persisted {
            if online {
                citadel_adapters::persistence::postgres::platforms::status::platform_online(
                    &self.pool, target.id,
                )
                .await?;
            } else {
                citadel_adapters::persistence::postgres::platforms::status::platform_offline(
                    &self.pool, target.id,
                )
                .await?;
            }
            progress.persisted = true;
        }
        if online && !progress.recovery_requested {
            if target.connector_type == citadel_platforms::ConnectorKind::Agent {
                // Existing per-Platform subscriptions are retained; missing ones
                // are ensured by their owner without waiting for its safety tick.
                self.targets.ensure_subscriptions();
            }
            if target.connector_type != citadel_platforms::ConnectorKind::EdgeAgent {
                let refresh = citadel_platforms::jobs::RuntimeRecoveryCoordinator::request(
                    citadel_platforms::jobs::RecoveryReason::Online,
                    target.platform_type,
                )
                .expect("Online recovery starts with Platform metadata");
                self.refreshes
                    .send(
                        ScopedEventRequest {
                            platform_id: target.id,
                            refresh,
                        },
                        cancel,
                    )
                    .await
                    .map_err(|_| std::io::Error::other("recovery queue closed or cancelled"))?;
            }
            // Edge sessions bootstrap on connection; probes must not restart the
            // session or compete with its own scoped recovery owner.
            progress.recovery_requested = true;
        }
        let observation = if online {
            platform_reachable_observation(target)
        } else {
            platform_unreachable_observation(
                target,
                &RuntimeCapabilityError::new(
                    citadel_platforms::RuntimeErrorKind::Unavailable,
                    "Platform health checks failed",
                    true,
                ),
            )
        };
        // A failed alert retries this step without repeating a committed status
        // or scheduling another resource recovery plan.
        self.alerts.observe(&observation).await?;
        if let Some(hub) = &self.realtime {
            hub.publish_runtime_change(
                target.id,
                "platformInventory",
                if online { "reachable" } else { "offline" },
                target.id.to_string(),
            );
        }
        Ok(())
    }
}

fn same_transition(left: &PlatformHealthTransition, right: &PlatformHealthTransition) -> bool {
    left.target.id == right.target.id
        && left.online == right.online
        && left.target.address == right.target.address
        && left.target.connector_type == right.target.connector_type
}

fn enqueue(pending: &mut BTreeMap<uuid::Uuid, Pending>, transition: PlatformHealthTransition) {
    let id = transition.target.id;
    if pending.get(&id).is_some_and(|entry| {
        let old = &entry.progress.transition;
        old.online == transition.online
            && old.target.address == transition.target.address
            && old.target.connector_type == transition.target.connector_type
    }) {
        return;
    }
    pending.insert(
        id,
        Pending {
            progress: Progress::new(transition),
            due: Instant::now(),
        },
    );
}

async fn run_transitions<E, A, AF>(
    cancel: &CancellationToken,
    mut receiver: BoundedReceiver<PlatformHealthTransition>,
    apply: A,
) where
    E: std::fmt::Display,
    A: Fn(Progress) -> AF,
    AF: Future<Output = (Progress, Result<(), E>)>,
{
    let mut pending = BTreeMap::<uuid::Uuid, Pending>::new();
    loop {
        if pending.is_empty() {
            let Some(transition) = receiver.recv(cancel).await else {
                return;
            };
            enqueue(&mut pending, transition);
        }
        // Drain a burst before selecting work, preserving only each latest state.
        // One entry per configured Platform, like the monitor's hysteresis map.
        // Always consume replacements, including when all current effects fail.
        // Capping this map independently would block a newer recovery transition
        // behind the old failed transitions occupying every slot.
        while let Some(transition) = receiver.try_recv() {
            enqueue(&mut pending, transition);
        }
        let (&id, next) = pending.iter().min_by_key(|(_, entry)| entry.due).unwrap();
        tokio::select! {
            biased;
            () = cancel.cancelled() => return,
            () = tokio::time::sleep_until(next.due) => {},
            incoming = receiver.recv(cancel) => {
                let Some(transition) = incoming else { return; };
                enqueue(&mut pending, transition); continue;
            }
        }
        let entry = pending.remove(&id).unwrap();
        let work = apply(entry.progress);
        tokio::pin!(work);
        let (progress, result) = loop {
            tokio::select! {
                biased;
                () = cancel.cancelled() => return,
                result = &mut work => break result,
                incoming = receiver.recv(cancel) => {
                    let Some(transition) = incoming else { return; };
                    enqueue(&mut pending, transition);
                }
            }
        };
        // Repeated delivery of the same confirmed state must not reset step
        // progress or schedule a second plan after an in-flight effect finishes.
        if pending
            .get(&id)
            .is_some_and(|entry| same_transition(&entry.progress.transition, &progress.transition))
        {
            pending.remove(&id);
        }
        if let Err(error) = result {
            RuntimeWork::PlatformLifecycle.failures(1);
            tracing::warn!(%error, platform_id=%id, "Platform lifecycle effect failed; retaining transition");
            // A newer confirmed transition supersedes the failed older one.
            pending.entry(id).or_insert(Pending {
                progress,
                due: Instant::now() + RETRY,
            });
        }
    }
}

/// Repair a crash between committed inventory and Deployment observation without
/// attaching database reconciliation to every five-second health sample.
pub(super) async fn deployment_catch_up(
    cancel: CancellationToken,
    targets: Arc<PlatformRuntimeRegistry>,
    pool: PgPool,
    realtime: Option<RealtimeHub>,
) -> Result<(), std::convert::Infallible> {
    let mut ticker = crate::workers::schedule::interval("lifecycle-deployments", DEPLOYMENT_SAFETY);
    loop {
        tokio::select! { biased; () = cancel.cancelled() => return Ok(()), _ = ticker.tick() => {} }
        for target in targets.snapshot().await.iter() {
            let _work = RuntimeWork::LifecycleDeploymentRecovery.start();
            let result = tokio::select! {
                () = cancel.cancelled() => return Ok(()),
                result = citadel_adapters::persistence::postgres::platforms::status::reconcile_deployments(&pool, target.id, None, false) => result,
            };
            match result {
                Ok(changed) if changed > 0 => {
                    if let Some(hub) = &realtime {
                        hub.publish_runtime_change(
                            target.id,
                            "platformInventory",
                            "updated",
                            target.id.to_string(),
                        );
                    }
                }
                Err(error) => {
                    tracing::warn!(%error, platform_id=%target.id, "Deployment lifecycle catch-up failed")
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests;
