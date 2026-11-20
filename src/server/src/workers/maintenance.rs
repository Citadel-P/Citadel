use crate::realtime::RealtimeHub;
use citadel_runtime::maintenance::{MaintenanceError, MaintenanceStore};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

pub(crate) fn register(
    supervisor: &mut citadel_runtime::TaskSupervisor,
    token: CancellationToken,
    store: Arc<dyn MaintenanceStore>,
    realtime: Option<RealtimeHub>,
    build_retention_days: Option<i32>,
    retention_interval: Duration,
) {
    supervisor.spawn(
        "operation-recovery",
        citadel_primitives::JobCategory::Maintenance,
        recovery(token.clone(), store.clone(), realtime),
    );
    supervisor.spawn(
        "lease-expiry",
        citadel_primitives::JobCategory::Maintenance,
        expiry(token.clone(), store.clone()),
    );
    supervisor.spawn(
        "history-retention",
        citadel_primitives::JobCategory::Maintenance,
        retention(
            token.clone(),
            store.clone(),
            build_retention_days,
            retention_interval,
        ),
    );
    supervisor.spawn(
        "statistics-compaction",
        citadel_primitives::JobCategory::Maintenance,
        compaction(token, store),
    );
}
#[cfg(test)]
pub(crate) async fn run(
    token: CancellationToken,
    store: Arc<dyn MaintenanceStore>,
    realtime: Option<RealtimeHub>,
    build_retention_days: Option<i32>,
    retention_interval: Duration,
) -> Result<(), std::convert::Infallible> {
    let _ = tokio::join!(
        recovery(token.clone(), store.clone(), realtime),
        expiry(token.clone(), store.clone()),
        retention(
            token.clone(),
            store.clone(),
            build_retention_days,
            retention_interval
        ),
        compaction(token, store)
    );
    Ok(())
}
async fn recovery(
    token: CancellationToken,
    store: Arc<dyn MaintenanceStore>,
    realtime: Option<RealtimeHub>,
) -> Result<(), std::convert::Infallible> {
    let mut tick = super::schedule::interval("recovery", super::recovery::FALLBACK);
    loop {
        tokio::select! { ()=token.cancelled()=>break, _=tick.tick()=>{} }
        match citadel_runtime::jobs::observe_work(store.reconcile(), |v| {
            Some((v.len() as u64, "resource kinds"))
        })
        .await
        {
            Ok(changed) => {
                if let Some(hub) = &realtime {
                    for resource in changed {
                        hub.publish_resource_change(resource, uuid::Uuid::nil(), "reconciled");
                    }
                }
            }
            Err(error) => tracing::warn!(%error, "Resource reconciliation failed"),
        }
    }
    Ok(())
}
async fn expiry(
    token: CancellationToken,
    store: Arc<dyn MaintenanceStore>,
) -> Result<(), std::convert::Infallible> {
    let mut tick = super::schedule::interval("lease-expiry", Duration::from_secs(30));
    loop {
        tokio::select! { ()=token.cancelled()=>break, _=tick.tick()=>{} }
        if let Err(error) = citadel_runtime::jobs::observe(store.expire_leases()).await {
            tracing::warn!(%error, "Lease expiry failed");
        }
    }
    Ok(())
}
async fn retention(
    token: CancellationToken,
    store: Arc<dyn MaintenanceStore>,
    build_retention_days: Option<i32>,
    retention_interval: Duration,
) -> Result<(), std::convert::Infallible> {
    let mut tick = super::schedule::interval("retention", retention_interval);
    loop {
        tokio::select! { ()=token.cancelled()=>break, _=tick.tick()=>{} }
        // Storage owns invocation-local family progress and short bounded batches.
        let iteration = citadel_runtime::jobs::JobHandle::iteration();
        if let Some(job) = citadel_runtime::jobs::JobHandle::current() {
            job.budget(Duration::from_secs(10));
        }
        let drain = store.cleanup(build_retention_days);
        tokio::select! {
            () = token.cancelled() => break,
            result = tokio::time::timeout(Duration::from_secs(10), drain) => match result {
                Ok(Ok(count)) => { if let Some(i) = iteration { i.finish(if count == 0 { citadel_runtime::jobs::JobOutcome::NoWork } else { citadel_runtime::jobs::JobOutcome::Productive }, Some((count, "rows"))); } },
                Ok(Err(error)) => { if let Some(i) = iteration { i.finish(citadel_runtime::jobs::JobOutcome::Failed, None); } tracing::warn!(%error, "Scheduled retention failed"); },
                Err(_) => { if let Some(i) = iteration { i.finish(citadel_runtime::jobs::JobOutcome::BudgetExhausted, None); } tracing::info!("Retention budget exhausted; remaining rows wait for the next maintenance pass"); },
            }
        }
    }
    Ok(())
}
async fn compaction(
    token: CancellationToken,
    store: Arc<dyn MaintenanceStore>,
) -> Result<(), std::convert::Infallible> {
    let mut tick = super::schedule::interval("statistics-compaction", COMPACTION_IDLE_INTERVAL);
    if let Some(job) = citadel_runtime::jobs::JobHandle::current() {
        job.policy(
            format!(
                "After completion: {}s when caught up / {}s with work remaining",
                COMPACTION_IDLE_INTERVAL.as_secs(),
                COMPACTION_BACKLOG_INTERVAL.as_secs()
            ),
            Some(COMPACTION_BUDGET),
        );
    }
    loop {
        tokio::select! { ()=token.cancelled()=>break, _=tick.tick()=>{} }
        let iteration = citadel_runtime::jobs::JobHandle::iteration();
        let mut productive_passes = 0;
        let delay = tokio::select! {
            ()=token.cancelled()=>break,
            result=tokio::time::timeout(COMPACTION_BUDGET,compact_batch_observed(store.as_ref(), &mut productive_passes)) => match result {
                Ok(Ok(false)) => { if let Some(i) = iteration { i.finish(if productive_passes > 0 { citadel_runtime::jobs::JobOutcome::Productive } else { citadel_runtime::jobs::JobOutcome::NoWork }, None); } COMPACTION_IDLE_INTERVAL },
                Ok(Ok(true)) => { if let Some(i) = iteration { i.finish(citadel_runtime::jobs::JobOutcome::BudgetExhausted, None); } COMPACTION_BACKLOG_INTERVAL },
                Ok(Err(error)) => {
                    if let Some(i) = iteration { i.finish(citadel_runtime::jobs::JobOutcome::Failed, None); }
                    tracing::warn!(%error,"Statistics compaction failed");
                    COMPACTION_IDLE_INTERVAL
                },
                Err(_) => {
                    if let Some(i) = iteration { i.finish(citadel_runtime::jobs::JobOutcome::BudgetExhausted, None); }
                    tracing::debug!("Compaction budget exhausted; remaining history waits for the next pass");
                    COMPACTION_BACKLOG_INTERVAL
                },
            }
        };
        // Pause after work completes, including after a timeout. Never catch up
        // missed ticks with another immediate batch.
        tick.reset_after(delay);
    }
    Ok(())
}
// Idle installations check infrequently; backlog gets bounded batches separated
// by a full minute of rest. Actual throughput depends on database cost.
const COMPACTION_IDLE_INTERVAL: Duration = Duration::from_secs(300);
const COMPACTION_BACKLOG_INTERVAL: Duration = Duration::from_secs(60);
const COMPACTION_BUDGET: Duration = Duration::from_secs(5);
const COMPACTION_PASSES: usize = 128;

// True means useful work may remain; stop when a stalled-source scan wraps.
#[cfg(test)]
async fn compact_batch(store: &dyn MaintenanceStore) -> Result<bool, MaintenanceError> {
    compact_batch_observed(store, &mut 0).await
}
async fn compact_batch_observed(
    store: &dyn MaintenanceStore,
    productive_passes: &mut u64,
) -> Result<bool, MaintenanceError> {
    let mut sweep = citadel_runtime::maintenance::CompactionSweep::default();
    let mut stalled = std::collections::BTreeSet::new();
    let mut progressed = false;
    let mut physical_complete = false;
    for _ in 0..COMPACTION_PASSES {
        let pass = store.compact_next(&mut sweep).await?;
        if let Some(job) = citadel_runtime::jobs::JobHandle::current() {
            job.progress(
                u64::from(pass.processed > 0),
                "productive compaction passes",
            );
        }
        *productive_passes += u64::from(pass.processed > 0);
        progressed |= pass.processed > 0;
        physical_complete |= !pass.scan_pending;
        let repeated_stall = pass
            .stalled_source
            .is_some_and(|source| !stalled.insert(source));
        if physical_complete {
            if pass.stalled_source.is_some() {
                if repeated_stall && pass.processed == 0 {
                    return Ok(progressed);
                }
            } else if pass.processed == 0 && !pass.scan_pending {
                // Discovery keeps productive scans pending until an idle sweep.
                // Earlier progress is completed work, not a remaining backlog.
                return Ok(false);
            }
        }
        tokio::task::yield_now().await;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::future::BoxFuture;
    use std::sync::{
        Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    };

    #[derive(Default)]
    struct Store {
        hours: Mutex<Vec<usize>>,
        processed: Mutex<Vec<usize>>,
        compactions: AtomicUsize,
        sweeps: AtomicUsize,
        active: AtomicBool,
        stall: bool,
        fail: bool,
        outcomes:
            Mutex<std::collections::VecDeque<citadel_runtime::maintenance::CompactionProgress>>,
    }
    struct Active<'a>(&'a AtomicBool);
    impl Drop for Active<'_> {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }
    impl MaintenanceStore for Store {
        fn reconcile(&self) -> BoxFuture<'_, Result<Vec<&'static str>, MaintenanceError>> {
            Box::pin(async { Ok(vec![]) })
        }
        fn expire_leases(&self) -> BoxFuture<'_, Result<(), MaintenanceError>> {
            Box::pin(async { Ok(()) })
        }
        fn cleanup(&self, _: Option<i32>) -> BoxFuture<'_, Result<u64, MaintenanceError>> {
            Box::pin(async {
                self.sweeps.fetch_add(1, Ordering::SeqCst);
                Ok(0)
            })
        }
        fn compact_next<'a>(
            &'a self,
            _: &'a mut citadel_runtime::maintenance::CompactionSweep,
        ) -> BoxFuture<'a, Result<citadel_runtime::maintenance::CompactionProgress, MaintenanceError>>
        {
            Box::pin(async {
                self.compactions.fetch_add(1, Ordering::SeqCst);
                self.active.store(true, Ordering::SeqCst);
                let _active = Active(&self.active);
                if self.fail {
                    return Err(MaintenanceError("test failure".into()));
                }
                if self.stall {
                    std::future::pending::<()>().await;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
                if let Some(outcome) = self.outcomes.lock().unwrap().pop_front() {
                    return Ok(outcome);
                }
                let mut hours = self.hours.lock().unwrap();
                let candidate = hours
                    .iter()
                    .enumerate()
                    .filter(|(_, n)| **n > 0)
                    .max_by_key(|(_, n)| **n)
                    .map(|(i, _)| i);
                if let Some(i) = candidate {
                    hours[i] -= 1;
                    self.processed.lock().unwrap()[i] += 1;
                    Ok(citadel_runtime::maintenance::CompactionProgress {
                        processed: 1,
                        stalled_source: None,
                        scan_pending: false,
                    })
                } else {
                    Ok(citadel_runtime::maintenance::CompactionProgress::default())
                }
            })
        }
    }

    #[tokio::test(start_paused = true)]
    async fn registered_maintenance_jobs_report_real_policy_and_budget_without_failure() {
        for stall in [false, true] {
            let catalog = citadel_runtime::jobs::JobCatalog::new(uuid::Uuid::now_v7());
            let token = CancellationToken::new();
            let mut owner =
                citadel_runtime::TaskSupervisor::new(token.clone()).with_jobs(catalog.clone());
            register(
                &mut owner,
                token,
                Arc::new(Store {
                    stall,
                    ..Default::default()
                }),
                None,
                None,
                Duration::from_secs(900),
            );
            tokio::time::sleep(Duration::from_secs(307)).await;
            let snapshot = catalog.snapshot();
            assert_eq!(snapshot.jobs.len(), 4);
            let job = snapshot
                .jobs
                .iter()
                .find(|job| job.id == "statistics-compaction")
                .unwrap();
            assert_eq!(job.budget_ms, Some(COMPACTION_BUDGET.as_millis() as u64));
            assert!(
                job.trigger
                    .as_deref()
                    .unwrap()
                    .contains(&COMPACTION_IDLE_INTERVAL.as_secs().to_string())
            );
            assert_eq!(job.failures, 0);
            assert_eq!(job.budget_exhaustions, u64::from(stall));
            assert_eq!(
                job.last_outcome,
                Some(if stall {
                    citadel_runtime::jobs::JobOutcome::BudgetExhausted
                } else {
                    citadel_runtime::jobs::JobOutcome::NoWork
                })
            );
            assert!(job.next_check.is_some());
            owner.shutdown(Duration::from_secs(1)).await.unwrap();
            assert!(
                catalog
                    .snapshot()
                    .jobs
                    .iter()
                    .all(|job| job.state == citadel_runtime::jobs::JobState::Stopped)
            );
        }
    }

    #[tokio::test(start_paused = true)]
    async fn drained_paged_work_reports_success_and_returns_to_idle_cadence() {
        use citadel_runtime::maintenance::CompactionProgress as Progress;
        let catalog = citadel_runtime::jobs::JobCatalog::new(uuid::Uuid::now_v7());
        let token = CancellationToken::new();
        let store = Arc::new(Store {
            outcomes: Mutex::new(
                [
                    Progress {
                        processed: 1,
                        scan_pending: true,
                        ..Default::default()
                    },
                    Progress {
                        scan_pending: true,
                        ..Default::default()
                    },
                    Progress::default(),
                ]
                .into(),
            ),
            ..Default::default()
        });
        let mut owner =
            citadel_runtime::TaskSupervisor::new(token.clone()).with_jobs(catalog.clone());
        register(
            &mut owner,
            token,
            store.clone(),
            None,
            None,
            Duration::from_secs(900),
        );
        tokio::time::sleep(Duration::from_secs(302)).await;
        let snapshot = catalog.snapshot();
        let job = snapshot
            .jobs
            .iter()
            .find(|job| job.id == "statistics-compaction")
            .unwrap();
        assert_eq!(job.budget_exhaustions, 0);
        assert_eq!(
            job.last_outcome,
            Some(citadel_runtime::jobs::JobOutcome::Productive)
        );
        assert_eq!(store.compactions.load(Ordering::SeqCst), 3);
        tokio::time::sleep(Duration::from_secs(295)).await;
        assert_eq!(store.compactions.load(Ordering::SeqCst), 3);
        tokio::time::sleep(Duration::from_secs(5)).await;
        assert_eq!(store.compactions.load(Ordering::SeqCst), 4);
        owner.shutdown(Duration::from_secs(1)).await.unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn compaction_drains_1024_sources_and_backfill_without_repeating_retention() {
        // 2,048 source-hours of migration backlog, plus another 1,024 during the hour.
        // SQL latency is modeled separately from pass count, without wall-clock assertions.
        let store = Arc::new(Store {
            hours: Mutex::new(vec![2; 1024]),
            processed: Mutex::new(vec![0; 1024]),
            ..Default::default()
        });
        let token = CancellationToken::new();
        let worker = tokio::spawn(run(
            token.clone(),
            store.clone(),
            None,
            None,
            Duration::from_secs(900),
        ));
        tokio::time::sleep(Duration::from_secs(1800)).await;
        for hours in store.hours.lock().unwrap().iter_mut() {
            *hours += 1;
        }
        tokio::time::sleep(Duration::from_secs(1800)).await;
        token.cancel();
        worker.await.unwrap().unwrap();
        assert_eq!(store.hours.lock().unwrap().iter().sum::<usize>(), 0);
        assert!(store.processed.lock().unwrap().iter().all(|n| *n == 3));
        assert!((3..=4).contains(&store.sweeps.load(Ordering::SeqCst)));
        assert!(store.compactions.load(Ordering::SeqCst) <= 60 * COMPACTION_PASSES);
        assert!(!store.active.load(Ordering::SeqCst));
    }

    #[tokio::test(start_paused = true)]
    async fn compaction_waits_five_minutes_at_startup_when_idle_and_after_errors() {
        for fail in [false, true] {
            let store = Arc::new(Store {
                fail,
                ..Default::default()
            });
            let token = CancellationToken::new();
            let worker = tokio::spawn(run(
                token.clone(),
                store.clone(),
                None,
                None,
                Duration::from_secs(900),
            ));
            tokio::time::sleep(Duration::from_secs(299)).await;
            assert_eq!(store.compactions.load(Ordering::SeqCst), 0);
            tokio::time::sleep(Duration::from_secs(3)).await;
            assert_eq!(store.compactions.load(Ordering::SeqCst), 1);
            tokio::time::sleep(Duration::from_secs(297)).await;
            assert_eq!(store.compactions.load(Ordering::SeqCst), 1);
            tokio::time::sleep(Duration::from_secs(3)).await;
            assert_eq!(store.compactions.load(Ordering::SeqCst), 2);
            token.cancel();
            worker.await.unwrap().unwrap();
            assert!(!store.active.load(Ordering::SeqCst));
        }
    }

    #[tokio::test(start_paused = true)]
    async fn compaction_pauses_between_backlog_batches_then_returns_to_idle() {
        let store = Arc::new(Store {
            hours: Mutex::new(vec![129]),
            processed: Mutex::new(vec![0]),
            ..Default::default()
        });
        let token = CancellationToken::new();
        let worker = tokio::spawn(run(
            token.clone(),
            store.clone(),
            None,
            None,
            Duration::from_secs(900),
        ));
        tokio::time::sleep(Duration::from_secs(304)).await;
        assert_eq!(store.compactions.load(Ordering::SeqCst), 128);
        assert_eq!(store.hours.lock().unwrap()[0], 1);
        tokio::time::sleep(Duration::from_secs(58)).await;
        assert_eq!(
            store.compactions.load(Ordering::SeqCst),
            128,
            "the pause starts after the batch completes"
        );
        tokio::time::sleep(Duration::from_secs(4)).await;
        assert_eq!(store.compactions.load(Ordering::SeqCst), 130);
        assert_eq!(store.hours.lock().unwrap()[0], 0);
        tokio::time::sleep(Duration::from_secs(290)).await;
        assert_eq!(store.compactions.load(Ordering::SeqCst), 130);
        tokio::time::sleep(Duration::from_secs(10)).await;
        assert_eq!(store.compactions.load(Ordering::SeqCst), 131);
        token.cancel();
        worker.await.unwrap().unwrap();
    }

    #[tokio::test(start_paused = true)]
    async fn compaction_budget_and_shutdown_drop_inflight_work() {
        let store = Arc::new(Store {
            stall: true,
            ..Default::default()
        });
        let token = CancellationToken::new();
        let worker = tokio::spawn(run(
            token.clone(),
            store.clone(),
            None,
            None,
            Duration::from_secs(900),
        ));
        tokio::time::sleep(Duration::from_secs(302)).await;
        assert!(store.active.load(Ordering::SeqCst));
        tokio::time::sleep(Duration::from_secs(5)).await;
        assert!(
            !store.active.load(Ordering::SeqCst),
            "the five-second budget drops stalled work"
        );
        tokio::time::sleep(Duration::from_secs(57)).await;
        assert_eq!(store.compactions.load(Ordering::SeqCst), 1);
        tokio::time::sleep(Duration::from_secs(3)).await;
        assert!(store.active.load(Ordering::SeqCst));
        token.cancel();
        worker.await.unwrap().unwrap();
        assert!(
            !store.active.load(Ordering::SeqCst),
            "shutdown cancels and drops in-flight SQL"
        );
        assert_eq!(store.compactions.load(Ordering::SeqCst), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn empty_physical_pages_do_not_end_the_scan_or_spin_after_completion() {
        use citadel_runtime::maintenance::CompactionProgress as Progress;
        let store = Store {
            outcomes: Mutex::new(
                [
                    Progress {
                        scan_pending: true,
                        ..Default::default()
                    },
                    Progress {
                        scan_pending: true,
                        stalled_source: Some(1),
                        ..Default::default()
                    },
                    Progress {
                        scan_pending: true,
                        stalled_source: Some(1),
                        ..Default::default()
                    },
                    Progress {
                        processed: 1,
                        ..Default::default()
                    },
                    Progress::default(),
                ]
                .into(),
            ),
            ..Default::default()
        };
        assert!(!compact_batch(&store).await.unwrap());
        assert_eq!(store.compactions.load(Ordering::SeqCst), 5);
        assert!(!compact_batch(&store).await.unwrap());
        assert_eq!(store.compactions.load(Ordering::SeqCst), 6);
        // A completed physical scan must stay complete for this batch while the
        // host scan wraps, even if compact() starts another physical page.
        let store = Store {
            outcomes: Mutex::new(
                [
                    Progress {
                        stalled_source: Some(1),
                        ..Default::default()
                    },
                    Progress {
                        scan_pending: true,
                        stalled_source: Some(1),
                        ..Default::default()
                    },
                ]
                .into(),
            ),
            ..Default::default()
        };
        assert!(!compact_batch(&store).await.unwrap());
        assert_eq!(store.compactions.load(Ordering::SeqCst), 2);
    }

    #[tokio::test(start_paused = true)]
    async fn productive_scans_continue_past_empty_pages_until_backlog_drains() {
        use citadel_runtime::maintenance::CompactionProgress as Progress;
        let empty = Progress {
            scan_pending: true,
            ..Default::default()
        };
        let work = Progress {
            processed: 1,
            scan_pending: true,
            ..Default::default()
        };
        // Discovery keeps productive scans pending across their empty final
        // page. Each source has another hour behind the same empty first pages.
        let store = Store {
            outcomes: Mutex::new(
                [
                    empty,
                    empty,
                    work,
                    empty,
                    empty,
                    empty,
                    work,
                    empty,
                    empty,
                    empty,
                    empty,
                    Progress::default(),
                ]
                .into(),
            ),
            ..Default::default()
        };
        assert!(!compact_batch(&store).await.unwrap());
        assert_eq!(store.compactions.load(Ordering::SeqCst), 12);
        assert!(store.outcomes.lock().unwrap().is_empty());
        assert!(!compact_batch(&store).await.unwrap());
    }

    #[tokio::test(start_paused = true)]
    async fn a_new_pending_scan_is_not_skipped_after_an_earlier_scan_completed() {
        use citadel_runtime::maintenance::CompactionProgress as Progress;
        let store = Store {
            outcomes: Mutex::new(
                [
                    Progress {
                        processed: 1,
                        ..Default::default()
                    },
                    Progress {
                        scan_pending: true,
                        ..Default::default()
                    },
                    Progress {
                        processed: 1,
                        scan_pending: true,
                        ..Default::default()
                    },
                    Progress::default(),
                ]
                .into(),
            ),
            ..Default::default()
        };
        assert!(!compact_batch(&store).await.unwrap());
        assert_eq!(store.compactions.load(Ordering::SeqCst), 4);
        assert!(store.outcomes.lock().unwrap().is_empty());
    }

    #[tokio::test(start_paused = true)]
    async fn compaction_pass_limit_bounds_a_fast_backlog() {
        let store = Store {
            hours: Mutex::new(vec![1000]),
            processed: Mutex::new(vec![0]),
            ..Default::default()
        };
        assert!(compact_batch(&store).await.unwrap());
        assert_eq!(store.compactions.load(Ordering::SeqCst), COMPACTION_PASSES);
        assert_eq!(store.hours.lock().unwrap()[0], 1000 - COMPACTION_PASSES);
    }
    #[tokio::test(start_paused = true)]
    async fn source_stalls_do_not_delay_healthy_work_and_all_stalled_batches_rest() {
        use citadel_runtime::maintenance::CompactionProgress as Progress;
        for physical_progress in [0, 1] {
            let stalled = |source| Progress {
                processed: 0,
                stalled_source: Some(source),
                scan_pending: false,
            };
            let store = Arc::new(Store {
                outcomes: Mutex::new(
                    [
                        stalled(1),
                        Progress {
                            processed: physical_progress,
                            stalled_source: Some(2),
                            scan_pending: false,
                        },
                        stalled(1),
                        // Next batch reaches a corrected source, then a healthy source.
                        Progress {
                            processed: 1,
                            stalled_source: None,
                            scan_pending: false,
                        },
                        Progress {
                            processed: 1,
                            stalled_source: None,
                            scan_pending: false,
                        },
                    ]
                    .into(),
                ),
                ..Default::default()
            });
            let token = CancellationToken::new();
            let worker = tokio::spawn(run(
                token.clone(),
                store.clone(),
                None,
                None,
                Duration::from_secs(900),
            ));
            tokio::time::sleep(Duration::from_secs(301)).await;
            assert_eq!(
                store.compactions.load(Ordering::SeqCst),
                3,
                "all-stalled scan must stop"
            );
            let delay = if physical_progress == 0 { 300 } else { 60 };
            tokio::time::sleep(Duration::from_secs(delay - 2)).await;
            assert_eq!(store.compactions.load(Ordering::SeqCst), 3);
            tokio::time::sleep(Duration::from_secs(3)).await;
            assert_eq!(store.compactions.load(Ordering::SeqCst), 6);
            token.cancel();
            worker.await.unwrap().unwrap();
        }
        let store = Arc::new(Store {
            outcomes: Mutex::new(
                [
                    Progress {
                        processed: 0,
                        stalled_source: Some(1),
                        scan_pending: false,
                    },
                    Progress {
                        processed: 1,
                        stalled_source: None,
                        scan_pending: false,
                    },
                    Progress::default(),
                ]
                .into(),
            ),
            ..Default::default()
        });
        let token = CancellationToken::new();
        let worker = tokio::spawn(run(
            token.clone(),
            store.clone(),
            None,
            None,
            Duration::from_secs(900),
        ));
        tokio::time::sleep(Duration::from_secs(301)).await;
        assert_eq!(
            store.compactions.load(Ordering::SeqCst),
            3,
            "healthy work follows a stall in the same batch"
        );
        token.cancel();
        worker.await.unwrap().unwrap();
    }
}
