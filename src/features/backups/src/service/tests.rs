use super::*;

#[test]
fn failures_before_execution_retain_diagnostics_for_log_history() {
    let source = failed_before_execution("SourceUnavailable", "No physical storage found.".into());
    let permission = rejected_backup("Source access revoked.".into());
    for result in [source, permission] {
        assert_eq!(result.status, BackupRunStatus::Failed);
        assert!(result.items.is_empty());
        assert_eq!(
            result.snapshot_availability,
            BackupSnapshotAvailability::NotCreated
        );
        assert_eq!(result.logs.len(), 1);
        assert_eq!(result.logs[0].stream, "stderr");
        assert_eq!(
            Some(result.logs[0].message.as_str()),
            result.error_message.as_deref()
        );
    }
    let restore = rejected_restore("Target access revoked.".into());
    assert_eq!(restore.status, BackupRestoreStatus::Failed);
    assert_eq!(restore.logs.len(), 1);
    assert_eq!(restore.logs[0].message, "Target access revoked.");
}
