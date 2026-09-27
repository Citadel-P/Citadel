//! One bounded operation's observations, fenced by its durable claim and identity.
use citadel_platforms::{
    RuntimeCapabilityError, RuntimeErrorKind,
    containers::*,
    jobs::{ProjectionKind, ProjectionWrite},
};
use citadel_runtime::runtime_metrics::RuntimeWork;
use sqlx::PgPool;
use uuid::Uuid;

pub(super) async fn persist(
    pool: &PgPool,
    claim: Uuid,
    observations: &[ContainerObservation],
) -> Result<(), RuntimeCapabilityError> {
    if observations.is_empty() {
        return Ok(());
    }
    if observations.len() > MAX_CONTAINER_BATCH {
        return Err(RuntimeCapabilityError::new(
            RuntimeErrorKind::InvalidRequest,
            "Too many Container observations.",
            false,
        ));
    }
    let _batch = RuntimeWork::ContainerObservationBatch.start();
    RuntimeWork::ContainerObservationBatch.units(observations.len() as u64);
    let mut ordered: Vec<_> = observations.iter().collect();
    ordered.sort_by_key(|o| o.target.id);
    if ordered
        .windows(2)
        .any(|pair| pair[0].target.id == pair[1].target.id)
    {
        return Err(RuntimeCapabilityError::new(
            RuntimeErrorKind::InvalidRequest,
            "Duplicate Container observations.",
            false,
        ));
    }
    let writes = ProjectionWrite::begin_many(ordered.iter().map(|o| {
        (
            o.target.platform_id,
            o.target.node_id.clone(),
            ProjectionKind::Containers,
        )
    }))
    .await;
    let ids: Vec<_> = ordered.iter().map(|o| o.target.id).collect();
    let platforms: Vec<_> = ordered.iter().map(|o| o.target.platform_id).collect();
    let nodes: Vec<_> = ordered
        .iter()
        .map(|o| o.target.node_id.as_deref())
        .collect();
    let docker: Vec<_> = ordered
        .iter()
        .map(|o| o.target.docker_id.as_str())
        .collect();
    let states: Vec<_> = ordered.iter().map(|o| o.state.as_deref()).collect();
    let mut tx = pool.begin().await.map_err(storage)?;
    // Same platform-before-container lock order as events and finalization.
    sqlx::query("SELECT id FROM platforms WHERE id=ANY($1) ORDER BY id FOR SHARE")
        .bind(&platforms)
        .fetch_all(&mut *tx)
        .await
        .map_err(storage)?;
    sqlx::query("SELECT id FROM containers WHERE id=ANY($1) AND containeroperationid=$2 ORDER BY id FOR UPDATE")
        .bind(&ids).bind(claim).fetch_all(&mut *tx).await.map_err(storage)?;
    let observed = chrono::Utc::now().timestamp();
    sqlx::query("UPDATE containers c SET state=initcap(o.state),updated=$7,projectionobservedat=GREATEST(COALESCE(c.projectionobservedat,0),$7),projectionstalesince=NULL,projectionstalereason=NULL,rowversion=c.rowversion+1 FROM unnest($1::uuid[],$2::uuid[],$3::text[],$4::text[],$5::text[]) o(id,platform,node,docker,state) WHERE c.id=o.id AND c.platformid=o.platform AND c.dockernodeid IS NOT DISTINCT FROM o.node AND c.dockercontainerid=o.docker AND c.containeroperationid=$6 AND o.state IS NOT NULL AND (c.state,c.projectionstalesince,c.projectionstalereason) IS DISTINCT FROM (initcap(o.state),NULL::bigint,NULL::text)")
        .bind(&ids).bind(&platforms).bind(&nodes).bind(&docker).bind(&states).bind(claim).bind(observed).execute(&mut *tx).await.map_err(storage)?;
    let deleted: Vec<(Uuid, Option<String>, String)> = sqlx::query_as("DELETE FROM containers c USING unnest($1::uuid[],$2::uuid[],$3::text[],$4::text[],$5::text[]) o(id,platform,node,docker,state) WHERE c.id=o.id AND c.platformid=o.platform AND c.dockernodeid IS NOT DISTINCT FROM o.node AND c.dockercontainerid=o.docker AND c.containeroperationid=$6 AND o.state IS NULL RETURNING c.platformid,c.dockernodeid,c.dockercontainerid")
        .bind(&ids).bind(&platforms).bind(&nodes).bind(&docker).bind(&states).bind(claim).fetch_all(&mut *tx).await.map_err(storage)?;
    sqlx::query("UPDATE containers c SET projectionobservedat=$7 FROM unnest($1::uuid[],$2::uuid[],$3::text[],$4::text[],$5::text[]) o(id,platform,node,docker,state) WHERE c.id=o.id AND c.platformid=o.platform AND c.dockernodeid IS NOT DISTINCT FROM o.node AND c.dockercontainerid=o.docker AND c.containeroperationid=$6 AND o.state IS NOT NULL AND COALESCE(c.projectionobservedat,0)<$7")
        .bind(&ids).bind(&platforms).bind(&nodes).bind(&docker).bind(&states).bind(claim).bind(observed).execute(&mut *tx).await.map_err(storage)?;
    tx.commit().await.map_err(storage)?;
    for (platform, node, docker) in deleted {
        super::super::runtime_index::committed_identity(
            pool,
            platform,
            node.as_deref(),
            &docker,
            None,
        );
    }
    for write in writes {
        write.committed();
    }
    Ok(())
}
fn storage(error: sqlx::Error) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, error.to_string(), false)
}
