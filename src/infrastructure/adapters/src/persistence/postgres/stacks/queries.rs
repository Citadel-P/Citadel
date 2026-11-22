use super::*;
use citadel_primitives::AuthorizedResource;

pub(super) const AUTHORIZED_CTES: &str = super::super::authorization::workload_grants!(Stack, 2);

pub(super) const PROJECTION: &str = concat!(
    r#"
SELECT s.id,s.name,s.description,s.stacksource,s.stackupdatestate,s.driftpolicy,
       s.createdat,s.createdbyactorid,s.controlstate,s.currentstackreleaseid,s.rowversion,
       EXISTS(SELECT 1 FROM resourcecopies c WHERE c.destinationid=s.id AND c.status='Succeeded') has_copied_storage,
       r.platformid,r.status release_status,r.version,r.spec,r.source,r.resourcebindings,
       p.status platform_status,p.name platform_name,p.platformdescriptor,
       CASE WHEN s.healthappliedplatformid=r.platformid THEN s.healthappliedspec END health_applied_spec,
       COALESCE((SELECT jsonb_agg(jsonb_build_object('service',sp.dockerserviceid,'mode',sp.mode,'desired',sp.desiredtaskcount,'running',sp.runningtaskcount,'stale',sp.isstale,'updateState',sp.updatestate,'ownership',sp.ownership,'stackServiceCount',sp.labels->>'com.citadel.stack-service-count','releaseId',sp.labels->>'com.citadel.release-id'))
        FROM swarmserviceprojections sp WHERE sp.platformid=r.platformid AND sp.stackid=s.id), '[]'::jsonb) health_services,
       CASE WHEN p.platformdescriptor->>'$type'='DockerSwarm' THEN COALESCE(("#,
    include_str!("../swarm_health_members.sql"),
    r#"task.platformid=r.platformid AND EXISTS(SELECT 1 FROM swarmserviceprojections sp WHERE sp.platformid=task.platformid AND sp.dockerserviceid=task.dockerserviceid AND sp.stackid=s.id AND sp.ownership<>'OwnershipConflict')
"#,
    include_str!("../swarm_health_container.sql"),
    r#"), '[]'::jsonb)
        ELSE COALESCE((SELECT jsonb_agg(jsonb_build_object('containerId',c.id,'dockerContainerId',c.dockercontainerid,'platformId',c.platformid,'nodeId',c.dockernodeid,'name',c.name,'service',COALESCE(c.composeservicename,''),'state',c.state,'available',(c.id IS NOT NULL AND c.projectionstalesince IS NULL AND (c.dockernodeid IS NULL OR EXISTS(SELECT 1 FROM swarmnoderuntimeprojectionstates ns WHERE ns.platformid=c.platformid AND ns.dockernodeid=c.dockernodeid AND NOT ns.isstale))),'stopRequestedAt',c.healthstoprequestedat,'observation',c.health) ORDER BY c.id) FROM containers c
        WHERE c.platformid=r.platformid AND c.stackid=s.id AND NOT c.isswarmtask), '[]'::jsonb) END health_members,

       COALESCE(tags.value,'[]'::jsonb) tags,
       activity.value latest_activity,
       $2::boolean permission_administrator, GREATEST(COALESCE(g.level_mask,0),COALESCE(rp.level_mask,0)) permission_level,
       (COALESCE(g.specific_mask,0)|COALESCE(rp.specific_mask,0)) permission_specific
FROM stacks s
JOIN stackreleases r ON r.id=s.currentstackreleaseid
JOIN platforms p ON p.id=r.platformid
LEFT JOIN global_permission g ON TRUE
LEFT JOIN resource_permissions rp ON rp.resourceid=s.id
LEFT JOIN LATERAL (
  SELECT jsonb_agg(jsonb_build_object('id',t.id,'name',t.name,'color',t.color) ORDER BY t.name,t.id) value
  FROM resourcetags rt JOIN tags t ON t.id=rt.tagid
  WHERE rt.resourcetype='Stack' AND rt.resourceid=s.id
) tags ON TRUE
LEFT JOIN LATERAL (
  SELECT jsonb_build_object('id',a.id,'resourceType',a.resourcetype,'eventType',a.eventtype,
    'status',a.status,'info',a.info::jsonb,'createdAt',a.createdat) value
  FROM activityevents a WHERE a.resourceid=s.id AND a.resourcetype='Stack'
    AND a.eventtype NOT IN ('StackUpdated','StackRenamed')
  ORDER BY a.createdat DESC,a.id DESC LIMIT 1
) activity ON TRUE
"#
);
impl PostgresStackRepository {
    pub(super) fn list_authorized_impl<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        filter: &'a StackFilter,
    ) -> BoxFuture<'a, Result<Vec<AuthorizedResource<citadel_stacks::Stack>>, StackError>> {
        Box::pin(async move {
            const FILTER: &str = r#"WHERE ($2 OR GREATEST(COALESCE(g.level_mask,0),COALESCE(rp.level_mask,0)) >= 1)
  AND ($3::uuid IS NULL OR r.platformid=$3)
  AND ($5::uuid[] IS NULL OR s.id=ANY($5))
  AND ($6::text[] IS NULL OR s.id IN (SELECT c.stackid FROM containers c WHERE c.platformid=$3 AND c.dockercontainerid=ANY($6)
     UNION ALL SELECT sp.stackid FROM swarmtaskprojections t JOIN swarmserviceprojections sp USING(platformid,dockerserviceid)
       WHERE t.platformid=$3 AND t.dockercontainerid=ANY($6))
       OR EXISTS (SELECT 1 FROM unnest($6) changed(id)
          WHERE NOT EXISTS (SELECT 1 FROM containers c WHERE c.platformid=$3 AND c.dockercontainerid=changed.id)
          AND NOT EXISTS (SELECT 1 FROM swarmtaskprojections t WHERE t.platformid=$3 AND t.dockercontainerid=changed.id)))
  AND NOT EXISTS (SELECT 1 FROM unnest($4::text[]) requested(name) WHERE NOT EXISTS (
    SELECT 1 FROM resourcetags link JOIN tags tag ON tag.id=link.tagid
    WHERE link.resourcetype='Stack' AND link.resourceid=s.id
      AND (lower(tag.name)=lower(requested.name) OR tag.id::text=requested.name)))"#;
            const ORDER: &str = "s.createdat DESC, s.name, s.id";
            // Bound authorized keys before projecting activity, tags and runtime observations.
            let sql = if filter.limit.is_some() {
                format!(
                    r#"{AUTHORIZED_CTES}, page AS MATERIALIZED (
                    SELECT s.id FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid
                    LEFT JOIN global_permission g ON TRUE
                    LEFT JOIN resource_permissions rp ON rp.resourceid=s.id
                    {FILTER} ORDER BY {ORDER} LIMIT $7
                )
                SELECT item.* FROM page CROSS JOIN LATERAL (
                    {PROJECTION} WHERE s.id=page.id LIMIT 1
                ) item ORDER BY item.createdat DESC,item.name,item.id"#
                )
            } else {
                format!("{AUTHORIZED_CTES}{PROJECTION} {FILTER} ORDER BY {ORDER} LIMIT $7")
            };
            sqlx::query(AssertSqlSafe(sql))
                .bind(actor.value())
                .bind(administrator)
                .bind(filter.platform_id)
                .bind(&filter.tags)
                .bind(filter.resource_ids.as_deref())
                .bind(filter.container_ids.as_deref())
                .bind(filter.limit)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_stack)
                .collect()
        })
    }
}
impl PostgresStackRepository {
    pub(super) fn get_authorized_impl(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'_, Result<AuthorizedResource<citadel_stacks::Stack>, StackError>> {
        Box::pin(async move { get_authorized(&self.pool, actor, administrator, id).await })
    }
}
impl PostgresStackRepository {
    pub(super) fn releases_impl(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<StackRelease>, StackError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            ensure_access(
                &mut tx,
                actor,
                administrator,
                id,
                policy::ViewStackReleases::REQUIREMENT,
            )
            .await?;
            let rows=sqlx::query("WITH history AS (SELECT DISTINCT ON (r.version) r.* FROM stackreleases r JOIN stacks s ON s.id=r.stackid JOIN stackreleases current ON current.id=s.currentstackreleaseid WHERE r.stackid=$1 AND r.id<>current.id AND r.version<>current.version AND r.status='Healthy' ORDER BY r.version,r.createdat,r.id) SELECT r.*,p.status platform_status,p.name platform_name,a.type actor_type,COALESCE(u.name,sa.name,t.name,'Unknown') actor_name FROM history r JOIN platforms p ON p.id=r.platformid JOIN actors a ON a.id=r.createdbyactorid LEFT JOIN users u ON u.actorid=a.id LEFT JOIN serviceaccounts sa ON sa.actorid=a.id LEFT JOIN teams t ON t.actorid=a.id ORDER BY r.createdat DESC,r.id DESC")
                .bind(id).fetch_all(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(storage)?;
            rows.into_iter().map(map_release).collect()
        })
    }
}

pub(super) async fn get_authorized(
    pool: &PgPool,
    actor: ActorId,
    administrator: bool,
    id: Uuid,
) -> Result<AuthorizedResource<citadel_stacks::Stack>, StackError> {
    let sql = format!(
        r#"{AUTHORIZED_CTES}{PROJECTION} WHERE s.id=$3 AND ($2 OR GREATEST(COALESCE(g.level_mask,0),COALESCE(rp.level_mask,0)) >= {READ})"#
    );
    sqlx::query(AssertSqlSafe(sql))
        .bind(actor.value())
        .bind(administrator)
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .map(map_stack)
        .transpose()?
        .ok_or(StackError::NotFound)
}
pub(super) async fn ensure_name_available(
    tx: &mut Transaction<'_, Postgres>,
    name: &str,
    exclude: Option<Uuid>,
) -> Result<(), StackError> {
    let exists:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM stacks WHERE lower(name)=lower($1) AND ($2::uuid IS NULL OR id<>$2))").bind(name).bind(exclude).fetch_one(&mut **tx).await.map_err(storage)?;
    if exists {
        Err(StackError::Conflict(
            "A Stack with this name already exists.".to_owned(),
        ))
    } else {
        Ok(())
    }
}
