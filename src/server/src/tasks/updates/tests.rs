use super::*;
use citadel_runtime::{DynamicTasks, admission::AdmissionGate};
use citadel_updates::UpdateDrainRuntime;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test]
async fn detached_cleanup_remains_visible_after_request_permit_is_released() {
    let gate = AdmissionGate::default();
    let tasks = DynamicTasks::new(CancellationToken::new());
    let runtime = TrackedUpdateDrain::new(gate.clone(), tasks.clone());
    let request = gate.try_enter("HTTP requests").unwrap();
    let reservation = tasks.reserve().unwrap();
    let fence = Uuid::now_v7();
    runtime.close(fence).unwrap();
    drop(request);
    let blockers = runtime.blockers(fence).unwrap();
    assert_eq!(blockers.len(), 1);
    assert_eq!(blockers[0].kind, "Detached work and cleanup");
    assert_eq!(blockers[0].count, 1);
    assert!(!tasks.cancellation().is_cancelled());
    // Required child cleanup can still register while admitted work finishes.
    let child = tasks.reserve().unwrap();
    drop(reservation);
    assert_eq!(runtime.blockers(fence).unwrap()[0].count, 1);
    drop(child);
    assert!(runtime.blockers(fence).unwrap().is_empty());
    assert!(gate.is_closed());
    assert!(runtime.blockers(Uuid::now_v7()).is_err());
}
