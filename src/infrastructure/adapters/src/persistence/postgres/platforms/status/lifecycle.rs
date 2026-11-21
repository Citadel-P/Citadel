use super::*;

pub async fn platform_online(pool: &PgPool, platform: Uuid) -> Result<()> {
    set_health(pool, platform, true, None).await.map(|_| ())
}

pub async fn platform_offline(pool: &PgPool, platform: Uuid) -> Result<()> {
    set_health(pool, platform, false, None).await.map(|_| ())
}

pub(crate) async fn set_health(
    pool: &PgPool,
    platform: Uuid,
    online: bool,
    edge_binding: Option<Uuid>,
) -> Result<bool> {
    let write = if online {
        None
    } else {
        Some(ProjectionWrite::begin(platform, None, ProjectionKind::Containers).await)
    };
    let mut tx = pool.begin().await?;
    if let Some(binding) = edge_binding {
        // Lock in the same order as revocation. A queued observation cannot
        // change the status of a revoked or replacement manager binding.
        let current = sqlx::query_scalar::<_, Uuid>("SELECT id FROM edgeagentbindings WHERE id=$1 AND platformid=$2 AND resourcetype='Platform' AND dockernodeid IS NULL AND revokedatutc IS NULL FOR SHARE")
            .bind(binding).bind(platform).fetch_optional(&mut *tx).await?;
        if current.is_none() {
            return Ok(false);
        }
    }
    if online {
        platform_status(
            &mut tx,
            platform,
            citadel_primitives::PlatformStatus::Online,
        )
        .await?;
    } else {
        platform_offline_in(&mut tx, platform).await?;
    }
    tx.commit().await?;
    if let Some(write) = write {
        write.committed();
    }
    reconcile_deployments(pool, platform, None, false).await?;
    Ok(true)
}
