use super::*;

pub(super) const AUTHORIZED_CTES: &str = r#"
WITH actor_scope AS (
    SELECT actor.id actorid FROM actors actor WHERE actor.id=$1 AND actor.isenabled
    UNION
    SELECT team.actorid FROM actorteammemberships membership
    JOIN teams team ON team.id=membership.teamid
    JOIN actors team_actor ON team_actor.id=team.actorid AND team_actor.isenabled
    WHERE membership.memberactorid=$1
), global_permission AS (
    SELECT COALESCE(MAX(permission.permissionlevel),0)::integer level_mask,
           COALESCE(bit_or(permission.specificpermissions),0)::integer specific_mask
    FROM actor_scope scope JOIN actorroles assignment ON assignment.actorid=scope.actorid
    JOIN permissions permission ON permission.roleid=assignment.roleid
    WHERE permission.resourcetype=2
), resource_permissions AS (
    SELECT access.resourceid,COALESCE(MAX(access.permissionlevel),0)::integer level_mask,
           COALESCE(bit_or(access.specificpermissions),0)::integer specific_mask
    FROM actor_scope scope JOIN resourceaccesses access ON access.actorid=scope.actorid
    WHERE access.resourcetype=2 GROUP BY access.resourceid
)
"#;

pub(super) const PROJECTION: &str = r#"
SELECT s.id,s.name,s.description,s.stacksource,s.stackupdatestate,s.driftpolicy,
       s.createdat,s.createdbyactorid,s.controlstate,s.currentstackreleaseid,s.rowversion,
       r.platformid,r.status release_status,r.version,r.spec,r.source,r.resourcebindings,
       p.status platform_status,p.name platform_name,p.platformdescriptor,
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
"#;
impl PostgresStackRepository {
    pub(super) fn list_authorized_impl<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        filter: &'a StackFilter,
    ) -> BoxFuture<'a, Result<Vec<StackDetails>, StackError>> {
        Box::pin(async move {
            let sql = format!(
                r#"{AUTHORIZED_CTES}{PROJECTION}
WHERE ($2 OR GREATEST(COALESCE(g.level_mask,0),COALESCE(rp.level_mask,0)) >= {READ})
  AND ($3::uuid IS NULL OR r.platformid=$3)
  AND NOT EXISTS (SELECT 1 FROM unnest($4::text[]) requested(name) WHERE NOT EXISTS (
    SELECT 1 FROM resourcetags link JOIN tags tag ON tag.id=link.tagid
    WHERE link.resourcetype='Stack' AND link.resourceid=s.id
      AND (lower(tag.name)=lower(requested.name) OR tag.id::text=requested.name)))
ORDER BY s.createdat DESC,s.name,s.id"#
            );
            sqlx::query(AssertSqlSafe(sql))
                .bind(actor.value())
                .bind(administrator)
                .bind(filter.platform_id)
                .bind(&filter.tags)
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
    ) -> BoxFuture<'_, Result<StackDetails, StackError>> {
        Box::pin(async move { get_authorized(&self.pool, actor, administrator, id).await })
    }
}
impl PostgresStackRepository {
    pub(super) fn releases_impl(
        &self,
        actor: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<StackReleaseDetails>, StackError>> {
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
) -> Result<StackDetails, StackError> {
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
