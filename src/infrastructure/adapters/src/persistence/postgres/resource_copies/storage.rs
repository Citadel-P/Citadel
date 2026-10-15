use super::*;
pub async fn destination_receipts(
    pool: &PgPool,
    id: Uuid,
) -> Result<Vec<RestoredCopyVolume>, CopyError> {
    sqlx::query(
        "SELECT p.platformid, c.targetvolume, r.id, r.targetdockernodeid
         FROM resourcecopies p
         JOIN resourcecopyrestores c ON c.copyid = p.id
         JOIN backuprestoreruns r ON r.id = c.restoreid
         WHERE p.destinationid = $1 AND p.status = 'Succeeded'
         UNION ALL
         SELECT p.platformid, job->>'target', (job->>'id')::uuid, volume->>'dockerNodeId'
         FROM resourcecopies p
         CROSS JOIN LATERAL jsonb_array_elements(COALESCE(p.plan->'current_volumes', '[]'::jsonb)) job
         CROSS JOIN LATERAL jsonb_array_elements(p.plan->'preview'->'volumes') volume
         WHERE p.destinationid = $1 AND p.status = 'Succeeded'
           AND job->>'status' = 'Succeeded' AND volume->>'target' = job->>'target'"
    ).bind(id).fetch_all(pool).await.map_err(storage_error)?.into_iter().map(|row| Ok(RestoredCopyVolume {
        platform_id:row.try_get("platformid").map_err(storage_error)?,restore_id:row.try_get("id").map_err(storage_error)?,node_id:row.try_get("targetdockernodeid").map_err(storage_error)?,name:row.try_get("targetvolume").map_err(storage_error)?,
    })).collect()
}
