use crate::{MigrationError, MigrationRunner};
use std::collections::BTreeSet;

/// Read-only evidence from the live migration journal; never repaired by validation.
#[derive(Clone, Debug)]
pub struct AppliedMigration {
    pub id: String,
    pub checksum: String,
    pub completed: bool,
    pub failed: bool,
}

impl MigrationRunner {
    /// Validate the exact signed-source catalog and its complete live journal
    /// against an append-only prefix of this target. Never repairs checksums.
    pub fn validate_upgrade_source(
        entries: &[AppliedMigration],
        source_manifest: &str,
        expected_sha256: &str,
    ) -> Result<(), MigrationError> {
        use serde::Deserialize;
        use sha2::{Digest, Sha256};
        #[derive(Deserialize)]
        struct Source {
            migrations: Vec<Migration>,
        }
        #[derive(Deserialize)]
        struct Migration {
            id: String,
            sha256: String,
        }
        let invalid = || {
            MigrationError::InvalidManifest(
                "Source catalog is not the approved append-only transition".into(),
            )
        };
        if source_manifest.len() > 128 * 1024
            || Sha256::digest(source_manifest.as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
                != expected_sha256
        {
            return Err(invalid());
        }
        let source: Source = serde_json::from_str(source_manifest).map_err(|_| invalid())?;
        let target = crate::runner::expected_checksums()?;
        if source.migrations.is_empty()
            || source.migrations.len() != entries.len()
            || source.migrations.len() > target.len()
        {
            return Err(invalid());
        }
        for ((source, actual), (id, checksum)) in
            source.migrations.iter().zip(entries).zip(target.iter())
        {
            if source.id != *id
                || source.sha256 != *checksum
                || actual.id != source.id
                || actual.checksum != source.sha256
                || !actual.completed
                || actual.failed
            {
                return Err(invalid());
            }
        }
        Ok(())
    }

    /// Return the embedded catalog identity only when the live journal matches
    /// every migration exactly. This does not verify arbitrary schema edits.
    pub fn validate_applied_catalog(
        entries: &[AppliedMigration],
    ) -> Result<String, MigrationError> {
        let expected = crate::runner::expected_checksums()?;
        let mut seen = BTreeSet::new();
        for entry in entries {
            let checksum = expected
                .get(&entry.id)
                .ok_or_else(|| MigrationError::UnknownMigration(entry.id.clone()))?;
            if !seen.insert(&entry.id) {
                return Err(MigrationError::InvalidJournal(entry.id.clone()));
            }
            if checksum != &entry.checksum {
                return Err(MigrationError::AppliedChecksum {
                    id: entry.id.clone(),
                    database: entry.checksum.clone(),
                    embedded: checksum.clone(),
                });
            }
            if !entry.completed || entry.failed {
                return Err(MigrationError::InvalidJournal(entry.id.clone()));
            }
        }
        for id in expected.keys() {
            if !seen.contains(id) {
                return Err(MigrationError::InvalidJournal(id.clone()));
            }
        }
        Self::catalog_sha256()
    }
}
