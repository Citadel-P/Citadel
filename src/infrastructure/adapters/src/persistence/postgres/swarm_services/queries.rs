use super::*;
use citadel_primitives::AuthorizedResource;

pub(super) const AUTHORIZED_CTES: &str = r#"
WITH actor_scope AS (
    SELECT id actorid FROM actors WHERE id=$1 AND isenabled
    UNION
    SELECT team.actorid FROM actorteammemberships membership
    JOIN teams team ON team.id=membership.teamid
    JOIN actors actor ON actor.id=team.actorid AND actor.isenabled
    WHERE membership.memberactorid=$1
), global_permission AS (
    SELECT COALESCE(MAX(permission.permissionlevel),0)::integer level_mask,
           COALESCE(bit_or(permission.specificpermissions),0)::integer specific_mask
    FROM actor_scope scope JOIN actorroles assignment ON assignment.actorid=scope.actorid
    JOIN permissions permission ON permission.roleid=assignment.roleid
    WHERE permission.resourcetype=20
), resource_permissions AS (
    SELECT access.resourceid, COALESCE(MAX(access.permissionlevel),0)::integer level_mask,
           COALESCE(bit_or(access.specificpermissions),0)::integer specific_mask
    FROM actor_scope scope JOIN resourceaccesses access ON access.actorid=scope.actorid
    WHERE access.resourcetype=20 GROUP BY access.resourceid
)
"#;

pub(super) const PROJECTION: &str = r#"
SELECT s.*, p.name platform_name, p.status platform_status,
       projection.runningtaskcount, projection.desiredtaskcount,
       projection.updatestate projection_update_state, projection.updatemessage,
       projection.liveruntimehash, projection.isstale,
       COALESCE(tags.value,'[]'::jsonb) tags,
       COALESCE(tasks.value,'[]'::jsonb) tasks,
       CASE WHEN s.autoupdatestate_lastcheckedat='-infinity'::timestamptz
            THEN TIMESTAMPTZ '0001-01-01 00:00:00+00'
            ELSE s.autoupdatestate_lastcheckedat
       END effective_last_checked_at,
       CASE
         WHEN s.dockerserviceid IS NULL THEN s.health
         WHEN projection.dockerserviceid IS NULL OR projection.isstale THEN 'Unknown'
         WHEN lower(COALESCE(projection.updatestate,'')) IN ('paused','rollback_paused','rollback_completed') THEN 'Failed'
         WHEN lower(COALESCE(projection.updatestate,'')) NOT IN ('','none','completed') THEN 'Progressing'
         WHEN projection.desiredtaskcount=0 THEN 'Stopped'
         WHEN projection.runningtaskcount>=projection.desiredtaskcount THEN 'Healthy'
         WHEN projection.runningtaskcount>0 THEN 'Degraded'
         ELSE 'Progressing'
       END effective_health,
       CASE
         WHEN s.dockerserviceid IS NULL THEN s.synchronizationstate
         WHEN projection.dockerserviceid IS NULL OR projection.isstale THEN 'RuntimeMissing'
         WHEN projection.ownership='OwnershipConflict' THEN 'OwnershipConflict'
         WHEN s.lastappliedruntimehash IS DISTINCT FROM projection.liveruntimehash THEN 'Drifted'
         WHEN s.lastapplieddesiredspechash IS DISTINCT FROM s.desiredspechash THEN 'DesiredChangesPending'
         ELSE 'InSync'
       END effective_synchronization_state,
       $2::boolean permission_administrator, GREATEST(COALESCE(g.level_mask,0),COALESCE(r.level_mask,0)) permission_level,
       (COALESCE(g.specific_mask,0)|COALESCE(r.specific_mask,0)) permission_specific
FROM swarmservices s
JOIN platforms p ON p.id=s.platformid
LEFT JOIN global_permission g ON TRUE
LEFT JOIN resource_permissions r ON r.resourceid=s.id
LEFT JOIN swarmserviceprojections projection
  ON projection.platformid=s.platformid AND projection.dockerserviceid=s.dockerserviceid
LEFT JOIN LATERAL (
    SELECT jsonb_agg(jsonb_build_object('id',tag.id,'name',tag.name,'color',tag.color) ORDER BY tag.name,tag.id) value
    FROM resourcetags link JOIN tags tag ON tag.id=link.tagid
    WHERE link.resourcetype='SwarmService' AND link.resourceid=s.id
) tags ON TRUE
LEFT JOIN LATERAL (
    SELECT jsonb_agg(jsonb_build_object(
        'id', task.dockertaskid,
        'name', task.name,
        'serviceId', task.dockerserviceid,
        'serviceName', task.servicename,
        'nodeId', task.dockernodeid,
        'nodeHostname', task.nodehostname,
        'containerId', task.dockercontainerid,
        'image', task.image,
        'desiredState', task.desiredstate,
        'state', task.state,
        'statusMessage', task.statusmessage,
        'error', task.error,
        'slot', task.slot,
        'versionIndex', task.versionindex,
        'createdAt', task.dockercreatedat,
        'updatedAt', task.dockerupdatedat,
        'statusTimestamp', task.statustimestamp,
        'ports', task.ports
    ) ORDER BY task.slot NULLS LAST, task.name, task.dockertaskid) value
    FROM swarmtaskprojections task
    WHERE task.platformid=s.platformid
      AND task.dockerserviceid=s.dockerserviceid
      AND NOT task.isstale
) tasks ON TRUE
"#;
impl PostgresSwarmServiceRepository {
    pub(super) fn duplicate_draft_impl(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'_, Result<citadel_swarm_services::SwarmServiceDuplicateDraft, SwarmServiceError>>
    {
        Box::pin(async move {
            let source = get_authorized(&self.pool, actor, administrator, id)
                .await?
                .resource;
            let name =
                available_service_name(&self.pool, source.platform_id, &source.name, true).await?;
            let mut spec = source.spec.for_create();
            spec.webhook = None;
            Ok(citadel_swarm_services::SwarmServiceDuplicateDraft {
                name,
                source_name: source.name,
                platform_id: source.platform_id,
                description: source.description,
                spec,
                tag_ids: source.tags.into_iter().map(|t| t.id).collect(),
                warnings: Vec::new(),
            })
        })
    }
}
impl PostgresSwarmServiceRepository {
    pub(super) fn list_authorized_impl<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        filter: &'a SwarmServiceFilter,
    ) -> BoxFuture<
        'a,
        Result<Vec<AuthorizedResource<citadel_swarm_services::SwarmService>>, SwarmServiceError>,
    > {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTES}{PROJECTION}
WHERE ($2 OR GREATEST(COALESCE(g.level_mask,0),COALESCE(r.level_mask,0)) >= 1)
  AND ($3::uuid IS NULL OR s.platformid=$3)
  AND NOT EXISTS (SELECT 1 FROM unnest($4::text[]) requested(name) WHERE NOT EXISTS (
      SELECT 1 FROM resourcetags filter_link JOIN tags filter_tag ON filter_tag.id=filter_link.tagid
      WHERE filter_link.resourcetype='SwarmService' AND filter_link.resourceid=s.id
        AND (lower(filter_tag.name)=lower(requested.name) OR filter_tag.id::text=requested.name)))
ORDER BY s.createdat DESC,s.name,s.id"#
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor_id.value())
                .bind(administrator)
                .bind(filter.platform_id)
                .bind(&filter.tags)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_service)
                .collect()
        })
    }
}
impl PostgresSwarmServiceRepository {
    pub(super) fn get_authorized_impl(
        &self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<
        '_,
        Result<AuthorizedResource<citadel_swarm_services::SwarmService>, SwarmServiceError>,
    > {
        Box::pin(async move { get_authorized(&self.pool, actor_id, administrator, id).await })
    }
}

pub(super) async fn get_authorized(
    pool: &PgPool,
    actor_id: ActorId,
    administrator: bool,
    id: Uuid,
) -> Result<AuthorizedResource<citadel_swarm_services::SwarmService>, SwarmServiceError> {
    let query = format!(
        "{AUTHORIZED_CTES}{PROJECTION} WHERE s.id=$3 AND ($2 OR GREATEST(COALESCE(g.level_mask,0),COALESCE(r.level_mask,0)) >= 1) LIMIT 1"
    );
    sqlx::query(AssertSqlSafe(query.as_str()))
        .bind(actor_id.value())
        .bind(administrator)
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .map(map_service)
        .transpose()?
        .ok_or(SwarmServiceError::NotFound)
}

pub(super) async fn available_service_name(
    pool: &PgPool,
    platform: Uuid,
    name: &str,
    duplicate: bool,
) -> Result<String, SwarmServiceError> {
    let normalized: String = name
        .chars()
        .take(64)
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let base = normalized.trim_matches(['-', '_']);
    let base = if base.is_empty() { "service" } else { base };
    for index in 1..=101 {
        let suffix = match (duplicate, index) {
            (_, 101) => format!("-{}", &Uuid::now_v7().simple().to_string()[24..]),
            (true, 1) => "-copy".into(),
            (true, n) => format!("-copy-{n}"),
            (false, 1) => String::new(),
            (false, n) => format!("-{n}"),
        };
        let prefix = &base[..base.len().min(64 - suffix.len())];
        let candidate = format!("{}{suffix}", prefix.trim_end_matches(['-', '_']));
        let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM swarmservices WHERE platformid=$1 AND lower(name)=lower($2))")
            .bind(platform).bind(&candidate).fetch_one(pool).await.map_err(storage)?;
        if !exists {
            return Ok(candidate);
        }
    }
    Err(SwarmServiceError::Conflict(
        "Could not allocate a Service name.".into(),
    ))
}
