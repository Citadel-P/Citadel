use super::*;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test]
async fn closing_rejects_new_work_and_waits_for_admitted_completion() {
    let gate = AdmissionGate::default();
    let work = gate.try_enter("Restore").unwrap();
    let fence = Uuid::now_v7();
    gate.close(fence).unwrap();
    assert!(gate.try_enter("Build").is_none());
    assert_eq!(gate.blockers().get("Restore"), Some(&1));
    let cancel = CancellationToken::new();
    assert!(
        matches!(gate.drain(fence, Duration::from_millis(1), &cancel).await, Err(AdmissionError::Busy(blockers)) if blockers.get("Restore") == Some(&1))
    );
    assert!(!cancel.is_cancelled());
    drop(work);
    gate.drain(fence, Duration::from_secs(1), &cancel)
        .await
        .unwrap();
    assert!(gate.is_closed());
    gate.reopen(fence).unwrap();
    assert!(gate.try_enter("Build").is_some());
}

#[tokio::test]
async fn paused_queue_does_not_dispatch_until_owner_resumes() {
    let gate = AdmissionGate::default();
    let fence = Uuid::now_v7();
    gate.close(fence).unwrap();
    let token = CancellationToken::new();
    let waiting = gate.enter("Queue", &token);
    tokio::pin!(waiting);
    assert!(
        tokio::time::timeout(Duration::from_millis(1), &mut waiting)
            .await
            .is_err()
    );
    assert!(gate.blockers().is_empty());
    gate.reopen(fence).unwrap();
    let permit = waiting.await.unwrap();
    assert_eq!(gate.blockers().get("Queue"), Some(&1));
    drop(permit);
    assert!(gate.blockers().is_empty());
}

#[tokio::test]
async fn cancellation_or_dropped_waiter_never_reopens_maintenance() {
    let gate = AdmissionGate::default();
    let permit = gate.try_enter("Copy").unwrap();
    let fence = Uuid::now_v7();
    gate.close(fence).unwrap();
    let token = CancellationToken::new();
    assert!(
        tokio::time::timeout(
            Duration::from_millis(1),
            gate.drain(fence, Duration::from_secs(30), &token)
        )
        .await
        .is_err()
    );
    assert!(gate.is_closed());
    token.cancel();
    assert_eq!(
        gate.drain(fence, Duration::from_secs(1), &token).await,
        Err(AdmissionError::Cancelled)
    );
    assert!(gate.enter("Queue", &token).await.is_none());
    assert_eq!(gate.blockers().get("Copy"), Some(&1));
    drop(permit);
    assert!(gate.is_closed());
}

#[tokio::test]
async fn foreign_or_stale_owners_cannot_drain_or_resume() {
    let gate = AdmissionGate::default();
    let fence = Uuid::now_v7();
    let other = Uuid::now_v7();
    assert_eq!(gate.close(Uuid::nil()), Err(AdmissionError::Ownership));
    gate.close(fence).unwrap();
    gate.close(fence).unwrap();
    assert_eq!(gate.close(other), Err(AdmissionError::Ownership));
    assert_eq!(gate.reopen(other), Err(AdmissionError::Ownership));
    assert_eq!(
        gate.drain(other, Duration::from_secs(1), &CancellationToken::new())
            .await,
        Err(AdmissionError::Ownership)
    );
    gate.reopen(fence).unwrap();
    gate.close(other).unwrap();
    assert_eq!(gate.reopen(fence), Err(AdmissionError::Ownership));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn admission_racing_close_is_either_counted_or_rejected() {
    for _ in 0..100 {
        let gate = AdmissionGate::default();
        let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(2));
        let start = barrier.clone();
        let producer = gate.clone();
        let task = tokio::spawn(async move {
            start.wait().await;
            producer.try_enter("Race")
        });
        barrier.wait().await;
        let fence = Uuid::now_v7();
        gate.close(fence).unwrap();
        let permit = task.await.unwrap();
        assert_eq!(
            gate.blockers().get("Race").copied().unwrap_or(0),
            usize::from(permit.is_some())
        );
        assert!(gate.try_enter("Late").is_none());
        drop(permit);
        gate.drain(fence, Duration::from_secs(1), &CancellationToken::new())
            .await
            .unwrap();
    }
}
