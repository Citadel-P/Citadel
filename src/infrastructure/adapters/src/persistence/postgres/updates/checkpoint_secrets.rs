//! Bounded read-only enumeration of stored encrypted recovery material.
use citadel_updates::UpdateError;
use serde::Deserialize;
use uuid::Uuid;

pub(crate) const CHECKPOINT_SECRET_PAGE_SIZE: usize = 2;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct CheckpointSecret {
    pub kind: u8,
    pub id: Uuid,
    pub envelope: String,
}

pub(crate) fn checkpoint_secrets_query(cursor: (u8, Uuid)) -> String {
    // Two maximum-size internal secrets (256 KiB each before Base64) fit the
    // Docker reader's 1 MiB output budget. Replacements are typed, never raw SQL.
    include_str!("checkpoint_secrets.sql")
        .replace("__PAGE_SIZE__", &CHECKPOINT_SECRET_PAGE_SIZE.to_string())
        .replace("__KIND__", &cursor.0.to_string())
        .replace("__ID__", &cursor.1.to_string())
}

pub(crate) fn checkpoint_secret_page(
    bytes: &[u8],
    mut cursor: (u8, Uuid),
) -> Result<Vec<CheckpointSecret>, UpdateError> {
    let failed =
        || UpdateError::Unavailable("Restored encrypted data is malformed or incomplete.".into());
    let rows: Vec<CheckpointSecret> = serde_json::from_slice(bytes).map_err(|_| failed())?;
    if rows.len() > CHECKPOINT_SECRET_PAGE_SIZE {
        return Err(failed());
    }
    for row in &rows {
        if !(1..=6).contains(&row.kind) || (row.kind, row.id) <= cursor || row.envelope.is_empty() {
            return Err(failed());
        }
        cursor = (row.kind, row.id);
    }
    Ok(rows)
}
