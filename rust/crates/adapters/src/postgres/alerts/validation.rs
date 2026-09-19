use super::*;
pub(super) fn normalized_ids(ids: &[Uuid], label: &str) -> Result<Vec<Uuid>, AlertError> {
    if ids.is_empty() || ids.iter().any(Uuid::is_nil) {
        return Err(AlertError::Validation(format!(
            "{label} IDs must not be empty."
        )));
    }
    let mut normalized = ids.to_vec();
    normalized.sort_unstable();
    normalized.dedup();
    Ok(normalized)
}

pub(super) async fn require_complete_set(
    tx: &mut Transaction<'_, Postgres>,
    table: &str,
    ids: &[Uuid],
) -> Result<(), AlertError> {
    let sql = match table {
        "alertchannels" => "SELECT id FROM alertchannels WHERE id=ANY($1) FOR SHARE",
        "alertrules" => "SELECT id FROM alertrules WHERE id=ANY($1) FOR SHARE",
        "alertevents" => "SELECT id FROM alertevents WHERE id=ANY($1) FOR UPDATE",
        _ => unreachable!("closed Alert table inventory"),
    };
    let found: Vec<Uuid> = sqlx::query_scalar(sql)
        .bind(ids)
        .fetch_all(&mut **tx)
        .await
        .map_err(storage)?;
    if found.len() != ids.len() {
        return Err(AlertError::NotFound);
    }
    Ok(())
}
