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

#[test]
fn upgrade_requires_complete_approved_source_and_unchanged_target_prefix() {
    let manifest = crate::MIGRATION_MANIFEST_JSON;
    let checksum = MigrationRunner::catalog_sha256().unwrap();
    MigrationRunner::validate_upgrade_source(&applied(), manifest, &checksum).unwrap();
    for case in 0..8 {
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
            6 => entries[0].failed = true,
            _ => entries.reverse(),
        }
        assert!(
            MigrationRunner::validate_upgrade_source(&entries, manifest, &checksum).is_err(),
            "{case}"
        );
    }
    assert!(
        MigrationRunner::validate_upgrade_source(&applied(), manifest, &"0".repeat(64)).is_err()
    );
    assert!(
        MigrationRunner::validate_upgrade_source(&applied(), &format!("{manifest} "), &checksum)
            .is_err()
    );
    // A real append-only source can contain fewer migrations than its target.
    let mut value: serde_json::Value = serde_json::from_str(manifest).unwrap();
    value["migrations"].as_array_mut().unwrap().pop();
    let source = serde_json::to_string(&value).unwrap();
    use sha2::{Digest, Sha256};
    let hash: String = Sha256::digest(source.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let mut entries = applied();
    entries.pop();
    MigrationRunner::validate_upgrade_source(&entries, &source, &hash).unwrap();
    assert!(MigrationRunner::validate_upgrade_source(&applied(), &source, &hash).is_err());
}
