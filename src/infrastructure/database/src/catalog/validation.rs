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
