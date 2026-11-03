use super::AppliedMigration;
use crate::MigrationRunner;

fn applied() -> Vec<AppliedMigration> {
    crate::runner::expected_checksums()
        .unwrap()
        .into_iter()
        .map(|(id, checksum)| AppliedMigration {
            id,
            checksum,
            completed: true,
            failed: false,
        })
        .collect()
}

#[test]
fn exact_complete_journal_returns_embedded_catalog_identity() {
    let mut entries = applied();
    entries.reverse();
    assert_eq!(
        MigrationRunner::validate_applied_catalog(&entries).unwrap(),
        MigrationRunner::catalog_sha256().unwrap()
    );
}

#[test]
fn missing_unknown_duplicate_failed_and_altered_entries_are_rejected() {
    for case in 0..7 {
        let mut entries = applied();
        match case {
            0 => entries.clear(),
            1 => {
                entries.pop();
            }
            2 => entries[0].id = "unknown".into(),
            3 => entries.push(entries[0].clone()),
            4 => entries[0].checksum = "0".repeat(64),
            5 => entries[0].completed = false,
            _ => entries[0].failed = true,
        }
        assert!(
            MigrationRunner::validate_applied_catalog(&entries).is_err(),
            "{case}"
        );
    }
}
