use super::*;
use crate::BackupRunItem;
fn fixture() -> (CoverageSnapshot, ObservedStorage) {
    let platform = Uuid::now_v7();
    let policy = Uuid::now_v7();
    let run = Uuid::now_v7();
    let source = BackupSourceSpec::DockerVolume {
        platform_id: platform,
        volume_name: "data".into(),
        docker_node_id: Some("a".into()),
        consistency: Default::default(),
    };
    let target = ObservedStorage {
        target: StorageTarget {
            platform_id: platform,
            name: "data".into(),
            host_path: None,
            docker_node_id: Some("a".into()),
        },
        included: false,
    };
    let run = BackupRun {
        id: run,
        backup_policy_id: policy,
        backup_repository_id: Uuid::now_v7(),
        policy_name_snapshot: "Daily".into(),
        repository_type_snapshot: "Restic".into(),
        source_snapshot: source.clone(),
        trigger: "Manual".into(),
        status: BackupRunStatus::Succeeded,
        snapshot_availability: BackupSnapshotAvailability::Available,
        restic_snapshot_id: Some("snapshot".into()),
        parent_snapshot_id: None,
        files_processed: None,
        bytes_processed: None,
        bytes_added: None,
        warnings: vec![],
        queued_at: Utc::now(),
        started_at: None,
        completed_at: Some(Utc::now()),
        exit_code: None,
        error_code: None,
        error_message: None,
        triggered_by_actor_id: Uuid::now_v7(),
        items: vec![BackupRunItem {
            id: Uuid::now_v7(),
            backup_run_id: run,
            platform_id: platform,
            volume_name: "data".into(),
            host_path: None,
            docker_node_id: Some("a".into()),
            node_hostname: None,
            status: BackupRunItemStatus::Succeeded,
            restic_snapshot_id: Some("snapshot".into()),
            parent_snapshot_id: None,
            files_processed: None,
            bytes_processed: None,
            bytes_added: None,
            started_at: None,
            completed_at: Some(Utc::now()),
            exit_code: None,
            error_code: None,
            error_message: None,
        }],
    };
    (
        CoverageSnapshot {
            policies: vec![CoveragePolicy {
                id: policy,
                name: "Daily".into(),
                enabled: true,
                cron: Some("0 0 * * *".into()),
                source,
                repository_ready: true,
                latest_run: Some(run),
            }],
            ..Default::default()
        },
        target,
    )
}
#[test]
fn green_requires_runnable_schedule_and_matching_available_snapshot() {
    let (mut snapshot, target) = fixture();
    assert_eq!(
        policy_status(&snapshot.policies[0], &target.target, true),
        Status::Protected
    );
    assert_eq!(
        policy_status(&snapshot.policies[0], &target.target, false),
        Status::Warning
    );
    let summary = snapshot.evaluate(
        target.target.platform_id,
        &[target.clone()],
        None,
        true,
        false,
    );
    assert_eq!(summary.reasons, vec![CoverageReason::SchedulingUnavailable]);
    snapshot.policies[0].cron = None;
    assert_eq!(
        policy_status(&snapshot.policies[0], &target.target, true),
        Status::Warning
    );
    snapshot.policies[0].cron = Some("0 0 * * *".into());
    snapshot.policies[0].latest_run.as_mut().unwrap().items[0].docker_node_id = Some("b".into());
    assert_eq!(
        policy_status(&snapshot.policies[0], &target.target, true),
        Status::Warning
    );
}
#[test]
fn failures_dominate_manual_configuration_and_history_survives() {
    let (mut snapshot, target) = fixture();
    let at = Utc::now();
    snapshot.successes.push((target.target.clone(), at));
    for failed in [
        BackupRunStatus::Failed,
        BackupRunStatus::TimedOut,
        BackupRunStatus::Interrupted,
        BackupRunStatus::Rejected,
    ] {
        snapshot.policies[0].latest_run.as_mut().unwrap().status = failed;
        let summary = snapshot.evaluate(
            target.target.platform_id,
            &[target.clone()],
            None,
            true,
            false,
        );
        assert_eq!(summary.status, Some(Status::Failed));
        assert_eq!(summary.volumes[0].last_successful_run_at, Some(at));
    }
}
#[test]
fn empty_and_incomplete_inventory_are_distinct_without_message_matching() {
    assert_eq!(aggregate([], false), Some(Status::NotApplicable));
    assert_eq!(aggregate([], true), Some(Status::Warning));
    let (snapshot, mut target) = fixture();
    target.target.docker_node_id = Some("b".into());
    assert_eq!(
        snapshot
            .evaluate(target.target.platform_id, &[target], None, true, true)
            .status,
        Some(Status::Unprotected)
    );
}
#[test]
fn workload_eligibility_preserves_node_and_bind_boundaries() {
    let (_, mut target) = fixture();
    assert!(!workload_includes(
        BackupPreviewKind::Deployment,
        &target.target,
        true
    ));
    target.target.docker_node_id = None;
    assert!(workload_includes(
        BackupPreviewKind::Deployment,
        &target.target,
        true
    ));
    assert!(!workload_includes(
        BackupPreviewKind::Deployment,
        &target.target,
        false
    ));
    target.target.host_path = Some("/srv/app".into());
    assert!(workload_includes(
        BackupPreviewKind::Deployment,
        &target.target,
        false
    ));
    target.target.host_path = Some("/var/run/docker.sock".into());
    assert!(!workload_includes(
        BackupPreviewKind::Stack,
        &target.target,
        false
    ));
}
