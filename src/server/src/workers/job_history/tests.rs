use super::run;
use citadel_instance::{InstanceError, JobDiagnosticCapture, JobHistory};
use citadel_instance::{
    JobDiagnosticSource, JobHistoryBatch, JobHistoryEvent, JobHistoryRepository,
};
use futures_util::future::BoxFuture;
use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

struct Source {
    catalog: citadel_runtime::jobs::JobCatalog,
    commits: AtomicU64,
}
impl JobDiagnosticSource for Source {
    fn capture(&self, after: u64) -> JobDiagnosticCapture {
        let c = self.catalog.diagnostic_capture(after);
        JobDiagnosticCapture {
            snapshot: serde_json::from_value(serde_json::to_value(c.snapshot).unwrap()).unwrap(),
            events: c
                .events
                .into_iter()
                .map(|e| JobHistoryEvent {
                    sequence: e.sequence,
                    job_id: e.job_id,
                    at: e.at,
                    kind: e.kind,
                    problem: e
                        .problem
                        .map(citadel_adapters::external::instance_jobs::map_problem),
                })
                .collect(),
            through: c.through,
            lost_events: c.lost_events,
        }
    }
    fn committed(&self) {
        self.commits.fetch_add(1, Ordering::SeqCst);
    }
}
struct Store {
    attempts: Mutex<Vec<(u64, Vec<u64>, bool)>>,
}
impl JobHistoryRepository for Store {
    fn persist<'a>(
        &'a self,
        batch: &'a JobHistoryBatch,
    ) -> BoxFuture<'a, Result<(), InstanceError>> {
        Box::pin(async move {
            let mut attempts = self.attempts.lock().unwrap();
            attempts.push((
                batch.session.heartbeat_at,
                batch.events.iter().map(|e| e.sequence).collect(),
                batch.session.stopped_at.is_some(),
            ));
            if attempts.len() == 1 {
                Err(InstanceError::Storage("fixture outage".into()))
            } else {
                Ok(())
            }
        })
    }
    fn read<'a>(
        &'a self,
        _: Option<&'a str>,
        _: u32,
    ) -> BoxFuture<'a, Result<JobHistory, InstanceError>> {
        panic!("reporting never reads history")
    }
}
#[tokio::test(start_paused = true)]
async fn failed_flush_retries_identical_batch_and_shutdown_is_bounded() {
    let source = Arc::new(Source {
        catalog: citadel_runtime::jobs::JobCatalog::new(Uuid::now_v7()),
        commits: AtomicU64::new(0),
    });
    let job = source.catalog.register(
        "worker",
        "Core",
        citadel_runtime::jobs::JobOwnership::Coordinator,
    );
    job.begin()
        .finish(citadel_runtime::jobs::JobOutcome::Failed, None);
    let store = Arc::new(Store {
        attempts: Mutex::new(Vec::new()),
    });
    let cancel = CancellationToken::new();
    let task = tokio::spawn(run(
        source.clone(),
        store.clone(),
        Uuid::now_v7(),
        "fixture".into(),
        cancel.clone(),
    ));
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(1)).await;
    tokio::task::yield_now().await;
    assert_eq!(source.commits.load(Ordering::SeqCst), 0);
    job.begin().finish(
        citadel_runtime::jobs::JobOutcome::Productive,
        Some((1, "items")),
    );
    tokio::time::advance(Duration::from_secs(30)).await;
    tokio::task::yield_now().await;
    cancel.cancel();
    tokio::time::timeout(Duration::from_secs(6), task)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let attempts = store.attempts.lock().unwrap();
    assert!(attempts.len() >= 3);
    assert_eq!(
        attempts[0], attempts[1],
        "retry must preserve exact batch identity"
    );
    assert!(attempts.last().unwrap().2);
    assert!(
        attempts.last().unwrap().1.contains(&2),
        "quick recovery survives failed persistence"
    );
}

struct Maintenance {
    calls: AtomicU64,
    stall: bool,
}
impl citadel_instance::JobHistoryMaintenance for Maintenance {
    fn maintain(&self) -> BoxFuture<'_, Result<u64, InstanceError>> {
        Box::pin(async {
            let call = self.calls.fetch_add(1, Ordering::SeqCst);
            if self.stall {
                return std::future::pending().await;
            }
            match call {
                0 => Err(InstanceError::Storage("maintenance fixture failure".into())),
                1 => Ok(12),
                _ => Ok(0),
            }
        })
    }
}

#[tokio::test(start_paused = true)]
async fn maintenance_is_delayed_bounded_cancellable_and_notifies_only_after_commit() {
    for stall in [false, true] {
        let catalog = citadel_runtime::jobs::JobCatalog::new(Uuid::now_v7());
        let source = Arc::new(Source {
            catalog: catalog.clone(),
            commits: AtomicU64::new(0),
        });
        let store = Arc::new(Maintenance {
            calls: AtomicU64::new(0),
            stall,
        });
        let cancel = CancellationToken::new();
        let mut supervisor =
            citadel_runtime::TaskSupervisor::new(cancel.clone()).with_jobs(catalog.clone());
        supervisor.spawn(
            "job-history-maintenance",
            "Maintenance",
            super::run_maintenance(store.clone(), source.clone(), cancel.clone()),
        );
        let first = crate::workers::schedule::first_phase(
            "job-history-maintenance",
            Duration::from_secs(300),
        );
        tokio::time::sleep(first - Duration::from_secs(1)).await;
        assert_eq!(store.calls.load(Ordering::SeqCst), 0);
        tokio::time::sleep(Duration::from_secs(309)).await;
        let job = catalog
            .snapshot()
            .jobs
            .into_iter()
            .find(|j| j.id == "job-history-maintenance")
            .unwrap();
        if stall {
            assert!(job.budget_exhaustions > 0);
            assert_eq!(source.commits.load(Ordering::SeqCst), 0);
        } else {
            assert_eq!(job.failures, 1);
            assert_eq!(job.productive, 1);
            assert_eq!(source.commits.load(Ordering::SeqCst), 1);
        }
        supervisor.shutdown(Duration::from_secs(1)).await.unwrap();
        let calls = store.calls.load(Ordering::SeqCst);
        tokio::time::sleep(Duration::from_secs(1000)).await;
        assert_eq!(store.calls.load(Ordering::SeqCst), calls);
    }
}
