use super::super::authorization::AUTHORIZED_BY_LEVELS_CTE;
use citadel_dashboards::{
    DashboardError, DashboardSource, DashboardViewer, SourceFilter, SourceKind,
};
use citadel_primitives::{PermissionLevel, ResourceType};
use sqlx::{PgPool, Row};

pub(super) async fn read(
    pool: &PgPool,
    viewer: DashboardViewer,
    filter: SourceFilter,
) -> Result<Vec<DashboardSource>, DashboardError> {
    let (resource, table, status, platform, docker_id, tag_type) = match filter.kind {
        SourceKind::Platform => (
            ResourceType::Platform,
            "platforms r",
            "r.status",
            "r.id",
            "NULL::text",
            "Platform",
        ),
        SourceKind::Deployment => (
            ResourceType::Deployment,
            "deployments r",
            "r.status",
            "r.platformid",
            "NULL::text",
            "Deployment",
        ),
        SourceKind::Stack => (
            ResourceType::Stack,
            "stacks r JOIN stackreleases release ON release.id=r.currentstackreleaseid",
            "release.status",
            "release.platformid",
            "NULL::text",
            "Stack",
        ),
        SourceKind::SwarmService => (
            ResourceType::SwarmService,
            "swarmservices r",
            "r.health",
            "r.platformid",
            "r.dockerserviceid",
            "SwarmService",
        ),
    };
    let container = if matches!(filter.kind, SourceKind::Deployment) && !filter.ids.is_empty() {
        "(SELECT c.id FROM containers c WHERE c.platformid=r.platformid AND c.deploymentid=r.id ORDER BY c.updated DESC,c.id DESC LIMIT 1)"
    } else {
        "NULL::uuid"
    };
    // Only pinned stacks need a series budget. Check indexed owner identities
    // and current projections before a widget requests the full history.
    let exceeds_budget = if matches!(filter.kind, SourceKind::Stack) && !filter.ids.is_empty() {
        r#"EXISTS (SELECT 1 FROM (
            SELECT servicename FROM statisticsowners WHERE stackid=r.id AND kind='Workload'
            UNION SELECT servicename FROM statisticsowners WHERE servicestackid=r.id AND kind='Service'
            UNION SELECT coalesce(nullif(c.composeservicename,''),c.id::text)
                FROM containers c WHERE c.stackid=r.id AND NOT c.isswarmtask
            UNION SELECT s.name FROM swarmserviceprojections s WHERE s.stackid=r.id
        ) names OFFSET 8)"#
    } else {
        "FALSE"
    };
    let pinned_workload = !filter.ids.is_empty() && !matches!(filter.kind, SourceKind::Platform);
    let (joins, effective_status) = if pinned_workload {
        if matches!(filter.kind, SourceKind::SwarmService) {
            (
                " JOIN platforms p ON p.id=v.platform_id JOIN swarmservices s ON s.id=v.id LEFT JOIN swarmserviceprojections projection ON projection.platformid=v.platform_id AND projection.dockerserviceid=v.docker_id",
                include_str!("../swarm_services/effective_health.sql"),
            )
        } else {
            (" JOIN platforms p ON p.id=v.platform_id", "v.status")
        }
    } else {
        ("", "v.status")
    };
    let logs = "($4 OR EXISTS(SELECT FROM actor_scope s JOIN actorroles ar ON ar.actorid=s.actorid JOIN permissions p ON p.roleid=ar.roleid WHERE p.resourcetype=$2 AND (p.specificpermissions & $10)<>0)
          OR EXISTS(SELECT FROM actor_scope s JOIN resourceaccesses a ON a.actorid=s.actorid WHERE a.resourcetype=$2 AND a.resourceid=v.id AND (a.specificpermissions & $10)<>0))";
    let projection = if pinned_workload {
        format!(
            "v.*, {effective_status} AS source_status, (p.status='Online' AND v.matches_tags AND {logs}) AS logs"
        )
    } else {
        "v.*, v.status AS source_status, FALSE AS logs".to_owned()
    };
    let sql = format!(
        r#"{AUTHORIZED_BY_LEVELS_CTE}, visible AS (
        SELECT r.id,r.name,{status} AS status,{platform} AS platform_id,{docker_id} AS docker_id,{container} AS container_id,{exceeds_budget} AS exceeds_budget,
        NOT EXISTS (SELECT FROM unnest($6::text[]) requested(name) WHERE NOT EXISTS (
            SELECT FROM resourcetags link JOIN tags t ON t.id=link.tagid WHERE link.resourceid=r.id
            AND link.resourcetype=$5 AND t.normalizedname=lower(requested.name))) AS matches_tags
        FROM {table} WHERE EXISTS(SELECT FROM enabled_actor)
        AND ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
          SELECT FROM actor_scope s JOIN resourceaccesses a ON a.actorid=s.actorid
          WHERE a.resourcetype=$2 AND a.resourceid=r.id AND a.permissionlevel=ANY($3)))
        AND (cardinality($7::uuid[])=0 OR r.id=ANY($7))
        AND ($8='' OR strpos(lower(r.name),lower($8))>0)
    ) SELECT {projection} FROM (SELECT * FROM visible WHERE matches_tags OR cardinality($7::uuid[])>0 ORDER BY name,id LIMIT $9) v{joins} ORDER BY v.name,v.id"#
    );
    let mut query = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(viewer.actor.value())
        .bind(resource as i32)
        .bind(PermissionLevel::Read.accepted_database_levels())
        .bind(viewer.administrator)
        .bind(tag_type)
        .bind(filter.tags)
        .bind(filter.ids)
        .bind(filter.search)
        .bind(filter.limit);
    if pinned_workload {
        query = query.bind(citadel_primitives::SpecificPermission::Logs as i32);
    }
    let rows = query
        .fetch_all(pool)
        .await
        .map_err(|e| DashboardError::Storage(e.to_string()))?;
    rows.iter()
        .map(|r| {
            Ok(DashboardSource {
                id: r.try_get("id")?,
                name: r.try_get("name")?,
                status: r.try_get("source_status")?,
                platform_id: r.try_get("platform_id")?,
                container_projection_id: r.try_get("container_id")?,
                docker_service_id: r.try_get("docker_id")?,
                can_view_logs: r.try_get("logs")?,
                matches_tags: r.try_get("matches_tags")?,
                exceeds_chart_series_limit: r.try_get("exceeds_budget")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()
        .map_err(|e| DashboardError::Storage(e.to_string()))
}
