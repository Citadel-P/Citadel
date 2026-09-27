use super::*;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

#[tokio::test(start_paused = true)]
async fn duplicate_checks_execute_one_scan_and_distinct_keys_remain_concurrent() {
    let gate = Arc::new(UpdateCheckGate::default());
    let scans = Arc::new(AtomicUsize::new(0));
    let id = Uuid::now_v7();
    let mut tasks = Vec::new();
    for _ in 0..16 {
        let gate = gate.clone();
        let scans = scans.clone();
        tasks.push(tokio::spawn(async move {
            let Some(_lease) = gate.try_enter(id) else {
                return false;
            };
            scans.fetch_add(1, Ordering::SeqCst);
            tokio::time::sleep(Duration::from_secs(5)).await;
            true
        }));
    }
    tokio::task::yield_now().await;
    assert_eq!(scans.load(Ordering::SeqCst), 1);
    let independent = gate.try_enter(Uuid::now_v7()).unwrap();
    let mut accepted = 0;
    for task in tasks {
        accepted += usize::from(task.await.unwrap());
    }
    assert_eq!(accepted, 1);
    assert!(gate.try_enter(id).is_some());
    drop(independent);
    assert!(gate.0.lock().unwrap().is_empty());
}

#[tokio::test]
async fn cancellation_failure_and_success_release_all_keys() {
    let gate = Arc::new(UpdateCheckGate::default());
    let id = Uuid::now_v7();
    for fail in [false, true] {
        let result: Result<(), ()> = async {
            let _lease = gate.try_enter(id).unwrap();
            if fail { Err(()) } else { Ok(()) }
        }
        .await;
        assert_eq!(result.is_err(), fail);
        assert!(gate.0.lock().unwrap().is_empty());
    }
    let held = gate.clone();
    let task = tokio::spawn(async move {
        let _lease = held.try_enter(id).unwrap();
        std::future::pending::<()>().await;
    });
    tokio::task::yield_now().await;
    assert!(gate.try_enter(id).is_none());
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(gate.try_enter(id).is_some());
    assert!(gate.0.lock().unwrap().is_empty());
}
