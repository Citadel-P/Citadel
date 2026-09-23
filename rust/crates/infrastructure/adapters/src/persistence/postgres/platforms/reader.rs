use super::*;

#[derive(Clone)]
pub struct PostgresPlatformReader {
    pool: PgPool,
}

impl PostgresPlatformReader {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl PlatformReader for PostgresPlatformReader {
    fn swarm_summary(
        &self,
        platform: Uuid,
    ) -> BoxFuture<'_, Result<citadel_platforms::swarm_overview::SwarmSummary, AuthorizedReadError>>
    {
        Box::pin(async move {
            let row = sqlx::query(include_str!("swarm_summary.sql"))
                .bind(platform)
                .fetch_one(&self.pool)
                .await
                .map_err(storage)?;
            Ok(citadel_platforms::swarm_overview::SwarmSummary {
                is_stale: row.try_get("isstale").map_err(storage)?,
                node_count: row.try_get("nodecount").map_err(storage)?,
                manager_count: row.try_get("managercount").map_err(storage)?,
                reachable_managers: row.try_get("reachablemanagers").map_err(storage)?,
                has_leader: row.try_get("hasleader").map_err(storage)?,
                managers_stale: row.try_get("managersstale").map_err(storage)?,
                service_counts: WorkloadStatusCounts {
                    total: row.try_get("total").map_err(storage)?,
                    healthy: row.try_get("healthy").map_err(storage)?,
                    degraded: row.try_get("degraded").map_err(storage)?,
                    failed: row.try_get("failed").map_err(storage)?,
                    stopped: row.try_get("stopped").map_err(storage)?,
                    unknown: row.try_get("unknown").map_err(storage)?,
                    ..Default::default()
                },
                running_tasks: row.try_get("runningtasks").map_err(storage)?,
                desired_tasks: row.try_get("desiredtasks").map_err(storage)?,
                network_count: row.try_get("networkcount").map_err(storage)?,
                local_network_count: row.try_get("localnetworkcount").map_err(storage)?,
                volume_count: row.try_get("volumecount").map_err(storage)?,
                image_count: row.try_get("imagecount").map_err(storage)?,
            })
        })
    }
    fn list_authorized<'a>(
        &'a self,
        actor_id: ActorId,
        is_administrator: bool,
        tag_ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<PlatformDetails>, AuthorizedReadError>> {
        Box::pin(async move {
            let query = format!(
                r#"
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
), global_access AS (
    SELECT EXISTS (
        SELECT 1 FROM actor_scope scope
        JOIN actorroles assignment ON assignment.actorid = scope.actorid
        JOIN permissions permission ON permission.roleid = assignment.roleid
        WHERE permission.resourcetype = $2
          AND (permission.permissionlevel & $3) <> 0
    ) AS allowed
)
{PLATFORM_SELECT}
WHERE ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
    SELECT 1 FROM actor_scope scope
    JOIN resourceaccesses access ON access.actorid = scope.actorid
    WHERE access.resourcetype = $2
      AND access.resourceid = p.id
      AND (access.permissionlevel & $3) <> 0
))
AND (cardinality($5::uuid[]) = 0 OR EXISTS (
    SELECT 1 FROM resourcetags tag
    WHERE tag.resourcetype = 'Platform'
      AND tag.resourceid = p.id
      AND tag.tagid = ANY($5::uuid[])
))
ORDER BY p.name, p.id
"#
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor_id.value())
                .bind(PLATFORM_RESOURCE_TYPE)
                .bind(READ_PERMISSION_MASK)
                .bind(is_administrator)
                .bind(tag_ids)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_platform)
                .collect()
        })
    }

    fn get_platform(
        &self,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Option<PlatformDetails>, AuthorizedReadError>> {
        Box::pin(async move {
            let query = format!("{PLATFORM_SELECT} WHERE p.id = $1");
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .map(map_platform)
                .transpose()
        })
    }

    fn permissions_for_platforms<'a>(
        &'a self,
        actor_id: ActorId,
        platform_ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<BTreeMap<Uuid, EffectivePlatformPermission>, AuthorizedReadError>>
    {
        Box::pin(async move {
            if platform_ids.is_empty() {
                return Ok(BTreeMap::new());
            }
            sqlx::query(
                r#"
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
)
SELECT requested.id,
       COALESCE(bit_or(effective_grant.permissionlevel), 0)::integer AS levelmask,
       COALESCE(bit_or(effective_grant.specificpermissions), 0)::integer AS specificmask
FROM unnest($2::uuid[]) requested(id)
LEFT JOIN LATERAL (
    SELECT permission.permissionlevel, permission.specificpermissions
    FROM actor_scope scope
    JOIN actorroles assignment ON assignment.actorid = scope.actorid
    JOIN permissions permission ON permission.roleid = assignment.roleid
    WHERE permission.resourcetype = $3
    UNION ALL
    SELECT access.permissionlevel, access.specificpermissions
    FROM actor_scope scope
    JOIN resourceaccesses access ON access.actorid = scope.actorid
    WHERE access.resourcetype = $3 AND access.resourceid = requested.id
) effective_grant ON TRUE
GROUP BY requested.id
"#,
            )
            .bind(actor_id.value())
            .bind(platform_ids)
            .bind(PLATFORM_RESOURCE_TYPE)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(|row| {
                Ok((
                    row.try_get("id").map_err(storage)?,
                    EffectivePlatformPermission {
                        level_mask: row.try_get("levelmask").map_err(storage)?,
                        specific_mask: row.try_get("specificmask").map_err(storage)?,
                    },
                ))
            })
            .collect()
        })
    }

    fn list_containers(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<ContainerDetails>, AuthorizedReadError>> {
        Box::pin(async move {
            sqlx::query(AssertSqlSafe(format!("{CONTAINER_SELECT} WHERE container.platformid=$1 ORDER BY container.name,container.id")))
                .bind(platform_id)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_container)
                .collect()
        })
    }

    fn get_container(
        &self,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Option<ContainerDetails>, AuthorizedReadError>> {
        Box::pin(async move {
            sqlx::query(AssertSqlSafe(format!(
                "{CONTAINER_SELECT} WHERE container.id=$1"
            )))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .map(map_container)
            .transpose()
        })
    }

    fn resolve_container_reference<'a>(
        &'a self,
        reference: &'a str,
    ) -> BoxFuture<'a, Result<Option<Uuid>, AuthorizedReadError>> {
        Box::pin(async move {
            if let Ok(id) = Uuid::parse_str(reference) {
                return Ok(Some(id));
            }
            if !matches!(reference.len(), 12 | 64)
                || !reference.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return Ok(None);
            }
            let ids: Vec<Uuid> = sqlx::query_scalar(
                "SELECT id FROM containers WHERE dockercontainerid LIKE $1 || '%' LIMIT 2",
            )
            .bind(reference.to_ascii_lowercase())
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;
            // Docker IDs are daemon-local. Never choose an arbitrary Platform or node.
            Ok(if ids.len() == 1 { Some(ids[0]) } else { None })
        })
    }

    fn list_stack_containers(
        &self,
        stack_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<ContainerDetails>, AuthorizedReadError>> {
        Box::pin(async move {
            sqlx::query(AssertSqlSafe(format!(
                "{CONTAINER_SELECT} WHERE container.stackid=$1 ORDER BY container.name,container.id"
            )))
            .bind(stack_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(map_container)
            .collect()
        })
    }

    fn list_images(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<ImageDetails>, AuthorizedReadError>> {
        Box::pin(async move {
            sqlx::query(
                r#"
SELECT image.id, image.tags::jsonb AS tags, image.name, image.dockerimageid,
       image.size, image.containers, image.platformid, image.createdat,
       image.controlstate, image.updatedat, image.registryid,
       NULL::text AS contentidentity, NULL::text AS dockernodeid,
       NULL::text AS nodehostname, FALSE AS isstale, NULL::text AS stalereason,
       NULL::jsonb AS repodigests
FROM images image
WHERE image.platformid = $1
UNION ALL
SELECT image.id, image.resource->'repo_tags',
       COALESCE(image.resource->'repo_tags'->>0, image.dockerimageid), image.dockerimageid,
       (image.resource->>'size')::double precision,
       LEAST(GREATEST((image.resource->>'containers')::bigint, 0), 2147483647)::integer,
       image.platformid, to_timestamp((image.resource->>'created')::bigint),
       'Idle', image.observedat, NULL::uuid,
       image.contentidentity, image.dockernodeid, node.hostname, image.isstale,
       CASE WHEN image.isstale THEN 'Node Agent is disconnected or unavailable.' END,
       image.resource->'repo_digests'
FROM swarmnodeimageprojections image
LEFT JOIN swarmnodeprojections node
  ON node.platformid=image.platformid AND node.dockernodeid=image.dockernodeid
WHERE image.platformid=$1
ORDER BY name, id
"#,
            )
            .bind(platform_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(map_image)
            .collect()
        })
    }

    fn list_swarm_nodes(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<SwarmNodeSummary>, AuthorizedReadError>> {
        Box::pin(list_swarm_nodes(&self.pool, platform_id, None))
    }

    fn list_node_volumes(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<NodeResourceProjection<RuntimeVolumeSummary>>, AuthorizedReadError>>
    {
        Box::pin(async move {
            sqlx::query("SELECT resource.resource,resource.dockernodeid,resource.isstale,node.hostname FROM swarmnodevolumeprojections resource LEFT JOIN swarmnodeprojections node ON node.platformid=resource.platformid AND node.dockernodeid=resource.dockernodeid WHERE resource.platformid=$1 ORDER BY resource.volumename,resource.dockernodeid")
                .bind(platform_id).fetch_all(&self.pool).await.map_err(storage)?
                .into_iter().map(map_node_resource).collect()
        })
    }

    fn list_node_networks(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<
        '_,
        Result<Vec<NodeResourceProjection<RuntimeNetworkSummary>>, AuthorizedReadError>,
    > {
        Box::pin(async move {
            sqlx::query("SELECT resource.resource,resource.dockernodeid,resource.isstale,node.hostname FROM swarmnodenetworkprojections resource LEFT JOIN swarmnodeprojections node ON node.platformid=resource.platformid AND node.dockernodeid=resource.dockernodeid WHERE resource.platformid=$1 ORDER BY resource.dockernetworkid,resource.dockernodeid")
                .bind(platform_id).fetch_all(&self.pool).await.map_err(storage)?
                .into_iter().map(map_node_resource).collect()
        })
    }

    fn get_swarm_node<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmNodeSummary>, AuthorizedReadError>> {
        Box::pin(async move {
            Ok(list_swarm_nodes(&self.pool, platform_id, Some(id))
                .await?
                .pop())
        })
    }

    fn list_swarm_services(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<SwarmServiceSummary>, AuthorizedReadError>> {
        Box::pin(list_swarm_services(&self.pool, platform_id, None))
    }

    fn get_swarm_service<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmServiceSummary>, AuthorizedReadError>> {
        Box::pin(async move {
            Ok(list_swarm_services(&self.pool, platform_id, Some(id))
                .await?
                .pop())
        })
    }

    fn list_swarm_tasks<'a>(
        &'a self,
        platform_id: Uuid,
        service_id: Option<&'a str>,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<SwarmTaskSummary>, AuthorizedReadError>> {
        Box::pin(list_swarm_tasks(
            &self.pool,
            platform_id,
            None,
            service_id,
            Some(limit),
        ))
    }

    fn get_swarm_task<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmTaskSummary>, AuthorizedReadError>> {
        Box::pin(async move {
            Ok(
                list_swarm_tasks(&self.pool, platform_id, Some(id), None, Some(1))
                    .await?
                    .pop(),
            )
        })
    }

    fn list_swarm_configs(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<SwarmConfigSummary>, AuthorizedReadError>> {
        Box::pin(list_swarm_configs(&self.pool, platform_id, None))
    }

    fn get_swarm_config<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmConfigSummary>, AuthorizedReadError>> {
        Box::pin(async move {
            Ok(list_swarm_configs(&self.pool, platform_id, Some(id))
                .await?
                .pop())
        })
    }

    fn list_swarm_networks(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<SwarmNetworkSummary>, AuthorizedReadError>> {
        Box::pin(list_swarm_networks(&self.pool, platform_id, None))
    }

    fn get_swarm_network<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmNetworkSummary>, AuthorizedReadError>> {
        Box::pin(async move {
            Ok(list_swarm_networks(&self.pool, platform_id, Some(id))
                .await?
                .pop())
        })
    }

    fn list_swarm_secrets(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<SwarmSecretSummary>, AuthorizedReadError>> {
        Box::pin(list_swarm_secrets(&self.pool, platform_id, None))
    }

    fn get_swarm_secret<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmSecretSummary>, AuthorizedReadError>> {
        Box::pin(async move {
            Ok(list_swarm_secrets(&self.pool, platform_id, Some(id))
                .await?
                .pop())
        })
    }
}
