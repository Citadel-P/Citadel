use super::*;
use citadel_primitives::AuthorizedResource;
impl PostgresDeploymentRepository {
    pub(super) fn list_authorized_impl<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        filter: &'a DeploymentFilter,
    ) -> BoxFuture<
        'a,
        Result<Vec<AuthorizedResource<citadel_deployments::Deployment>>, DeploymentError>,
    > {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTES}{PROJECTION}
WHERE ($2 OR GREATEST(COALESCE(g.level_mask, 0), COALESCE(r.level_mask, 0)) = ANY($5))
  AND ($3::uuid IS NULL OR d.platformid = $3)
  AND NOT EXISTS (
      SELECT 1 FROM unnest($4::text[]) requested(name)
      WHERE NOT EXISTS (
          SELECT 1 FROM resourcetags filter_link
          JOIN tags filter_tag ON filter_tag.id = filter_link.tagid
          WHERE filter_link.resourcetype = 'Deployment'
            AND filter_link.resourceid = d.id
            AND (lower(filter_tag.name) = lower(requested.name)
                 OR filter_tag.id::text = requested.name)))
ORDER BY d.createdat DESC, d.name, d.id"#
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor_id.value())
                .bind(administrator)
                .bind(filter.platform_id)
                .bind(&filter.tags)
                .bind(ReadDeployment::REQUIREMENT.level.accepted_database_levels())
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_deployment)
                .collect()
        })
    }

    pub(super) fn get_authorized_impl<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'a, Result<AuthorizedResource<citadel_deployments::Deployment>, DeploymentError>>
    {
        Box::pin(async move { get_authorized(&self.pool, actor_id, administrator, id).await })
    }

    pub(super) fn duplicate_draft_impl<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
        id: Uuid,
    ) -> BoxFuture<'a, Result<DeploymentDuplicateDraft, DeploymentError>> {
        Box::pin(async move {
            let source = get_authorized(&self.pool, actor_id, administrator, id)
                .await?
                .resource;
            let name =
                available_duplicate_name(&self.pool, &source.name, source.platform_id).await?;
            let warnings = has_likely_host_bind(source.spec.volumes.as_deref())
                .then(|| DuplicateWarning {
                    code: "HOST_BIND_MOUNT".to_owned(),
                    message: "This deployment contains host paths that may not exist on another platform.".to_owned(),
                    field_path: Some("spec.volumes".to_owned()),
                })
                .into_iter()
                .collect();
            Ok(DeploymentDuplicateDraft {
                draft: DeploymentDraft {
                    name,
                    platform_id: source.platform_id,
                    description: source.description,
                    spec: source.spec.for_create(),
                    tag_ids: source.tags.iter().map(|tag| tag.id).collect(),
                    duplicate_source: DuplicateSource {
                        resource_type: "Deployment".to_owned(),
                        resource_id: source.id,
                        resource_name: source.name,
                    },
                },
                warnings,
            })
        })
    }
}
pub(super) const AUTHORIZED_CTES: &str = r#"
WITH actor_scope AS (
    SELECT actor.id AS actorid
    FROM actors actor
    WHERE actor.id = $1 AND actor.isenabled
    UNION
    SELECT team.actorid
    FROM actorteammemberships membership
    JOIN teams team ON team.id = membership.teamid
    JOIN actors team_actor ON team_actor.id = team.actorid AND team_actor.isenabled
    WHERE membership.memberactorid = $1
), global_permission AS (
    SELECT COALESCE(MAX(permission.permissionlevel), 0)::integer AS level_mask,
           COALESCE(bit_or(permission.specificpermissions), 0)::integer AS specific_mask
    FROM actor_scope scope
    JOIN actorroles assignment ON assignment.actorid = scope.actorid
    JOIN permissions permission ON permission.roleid = assignment.roleid
    WHERE permission.resourcetype = 1
), resource_permissions AS (
    SELECT access.resourceid,
           COALESCE(MAX(access.permissionlevel), 0)::integer AS level_mask,
           COALESCE(bit_or(access.specificpermissions), 0)::integer AS specific_mask
    FROM actor_scope scope
    JOIN resourceaccesses access ON access.actorid = scope.actorid
    WHERE access.resourcetype = 1
    GROUP BY access.resourceid
)
"#;

pub(super) const PROJECTION: &str = r#"
SELECT d.id, d.name, d.description, d.platformid, d.createdat, d.createdbyactorid,
       d.status, d.controlstate, d.rowversion, d.spec,
       CASE WHEN d.autoupdatestate_lastcheckedat = '-infinity'::timestamptz
            THEN TIMESTAMPTZ '0001-01-01 00:00:00+00'
            ELSE d.autoupdatestate_lastcheckedat END AS autoupdatestate_lastcheckedat,
       d.autoupdatestate_status,
       d.autoupdatestate_currentdigest, d.autoupdatestate_remotedigest,
       d.autoupdatestate_lasterror,
       p.status AS platform_status, p.name AS platform_name,
       c.id AS container_id, c.dockercontainerid, c.dockerimageid,
       i.id AS image_id, i.name AS image_name,
       COALESCE(tags.value, '[]'::jsonb) AS tags,
       activity.value AS latest_activity,
       GREATEST(COALESCE(g.level_mask, 0), COALESCE(r.level_mask, 0)) AS permission_level,
       $2 AS permission_administrator,
       (COALESCE(g.specific_mask, 0) | COALESCE(r.specific_mask, 0)) AS permission_specific
FROM deployments d
JOIN platforms p ON p.id = d.platformid
LEFT JOIN global_permission g ON TRUE
LEFT JOIN resource_permissions r ON r.resourceid = d.id
LEFT JOIN LATERAL (
    SELECT container.id, container.dockercontainerid, container.dockerimageid, container.imageid
    FROM containers container
    WHERE container.deploymentid = d.id
    ORDER BY container.updated DESC, container.id DESC
    LIMIT 1
) c ON TRUE
LEFT JOIN images i ON i.id = COALESCE(
    c.imageid,
    CASE WHEN d.spec -> 'Image' ->> '$type' = 'Local'
              AND d.spec -> 'Image' ->> 'ImageId' ~* '^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$'
         THEN CAST(d.spec -> 'Image' ->> 'ImageId' AS uuid) END)
LEFT JOIN LATERAL (
    SELECT jsonb_agg(jsonb_build_object('id', tag.id, 'name', tag.name, 'color', tag.color)
                     ORDER BY tag.name, tag.id) AS value
    FROM resourcetags link
    JOIN tags tag ON tag.id = link.tagid
    WHERE link.resourcetype = 'Deployment' AND link.resourceid = d.id
) tags ON TRUE
LEFT JOIN LATERAL (
    SELECT jsonb_build_object(
        'id', event.id, 'resourceType', event.resourcetype, 'eventType', event.eventtype,
        'status', event.status, 'info', event.info::jsonb, 'createdAt', event.createdat) AS value
    FROM activityevents event
    WHERE event.resourceid = d.id AND event.resourcetype = 'Deployment'
      AND event.eventtype NOT IN ('DeploymentUpdated', 'DeploymentRenamed')
    ORDER BY event.createdat DESC, event.id DESC
    LIMIT 1
) activity ON TRUE
"#;

pub(super) async fn get_authorized(
    pool: &PgPool,
    actor_id: ActorId,
    administrator: bool,
    id: Uuid,
) -> Result<AuthorizedResource<citadel_deployments::Deployment>, DeploymentError> {
    let query = format!(
        r#"{AUTHORIZED_CTES}{PROJECTION}
WHERE d.id=$3 AND ($2 OR GREATEST(COALESCE(g.level_mask,0),COALESCE(r.level_mask,0)) = ANY($4))
LIMIT 1"#
    );
    sqlx::query(AssertSqlSafe(query.as_str()))
        .bind(actor_id.value())
        .bind(administrator)
        .bind(id)
        .bind(ReadDeployment::REQUIREMENT.level.accepted_database_levels())
        .fetch_optional(pool)
        .await
        .map_err(storage)?
        .map(map_deployment)
        .transpose()?
        .ok_or(DeploymentError::NotFound)
}

pub(super) async fn available_duplicate_name(
    pool: &PgPool,
    source: &str,
    platform_id: Uuid,
) -> Result<String, DeploymentError> {
    for attempt in 1..=100 {
        let suffix = if attempt == 1 {
            "copy".to_owned()
        } else {
            format!("copy-{attempt}")
        };
        let candidate = duplicate_name(source, &suffix);
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM deployments WHERE platformid=$1 AND name=$2)",
        )
        .bind(platform_id)
        .bind(&candidate)
        .fetch_one(pool)
        .await
        .map_err(storage)?;
        if !exists {
            return Ok(candidate);
        }
    }
    Ok(duplicate_name(
        source,
        &Uuid::now_v7().simple().to_string()[..8],
    ))
}

pub(super) fn duplicate_name(source: &str, suffix: &str) -> String {
    let suffix = format!("-{suffix}");
    let maximum = 64_usize.saturating_sub(suffix.len()).max(1);
    let mut source = source.chars().take(maximum).collect::<String>();
    source = source.trim_end_matches(['-', '_']).to_owned();
    if source.is_empty() {
        source = "resource".to_owned();
    }
    format!("{source}{suffix}")
}

pub(super) fn has_likely_host_bind(volumes: Option<&[String]>) -> bool {
    volumes.is_some_and(|volumes| {
        volumes.iter().any(|volume| {
            let volume = volume.trim();
            volume.starts_with('/')
                || volume
                    .split_once(':')
                    .is_some_and(|(source, _)| source.starts_with('.') || source.starts_with('~'))
        })
    })
}
