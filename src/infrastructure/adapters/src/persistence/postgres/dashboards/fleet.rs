use citadel_dashboards::{
    AttentionTab, DashboardError, DashboardViewer, FleetFilter, FleetPage, FleetRead,
};
use sqlx::{PgPool, types::Json};

// Only the selected branches enter the SQL statement. A state histogram never reads alerts,
// backup history, or update evidence. Authorization and scope precede every aggregation.
pub(super) async fn read(
    pool: &PgPool,
    viewer: DashboardViewer,
    f: FleetFilter,
) -> Result<FleetPage, DashboardError> {
    use citadel_primitives::ResourceType;
    let mut branches = Vec::new();
    let service_state = include_str!("../swarm_services/effective_health.sql");
    let resources = [
        (
            "Platform",
            ResourceType::Platform,
            "p.id,p.id,p.id,p.name",
            "p.status",
            "platforms p",
        ),
        (
            "Deployment",
            ResourceType::Deployment,
            "d.id,d.id,d.platformid,d.name",
            "d.status",
            "deployments d",
        ),
        (
            "Stack",
            ResourceType::Stack,
            "s.id,s.id,r.platformid,s.name",
            "r.status",
            "stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid",
        ),
        (
            "SwarmService",
            ResourceType::SwarmService,
            "s.id,s.id,s.platformid,s.name",
            service_state,
            "swarmservices s LEFT JOIN swarmserviceprojections projection ON projection.platformid=s.platformid AND projection.dockerserviceid=s.dockerserviceid",
        ),
        (
            "Container",
            ResourceType::Platform,
            "c.id,c.platformid,c.platformid,c.name",
            "CASE WHEN c.projectionstalesince IS NOT NULL OR p.status<>'Online' THEN 'Unknown' ELSE c.state END",
            "containers c JOIN platforms p ON p.id=c.platformid",
        ),
        (
            "BackupPolicy",
            ResourceType::BackupPolicy,
            "id,id,NULL::uuid,name",
            "'Unknown'",
            "backuppolicies WHERE archivedat IS NULL",
        ),
        (
            "Build",
            ResourceType::Build,
            "id,id,NULL::uuid,name",
            "'Unknown'",
            "buildprojects WHERE archivedat IS NULL",
        ),
        (
            "BuildAgentPool",
            ResourceType::BuildAgentPool,
            "id,id,NULL::uuid,name",
            "'Unknown'",
            "buildagentpools WHERE archivedat IS NULL",
        ),
        (
            "Registry",
            ResourceType::Registry,
            "id,id,NULL::uuid,name",
            "'Unknown'",
            "registries",
        ),
        (
            "GitRepository",
            ResourceType::GitRepository,
            "id,id,NULL::uuid,name",
            "'Unknown'",
            "gitrepositories",
        ),
        (
            "Script",
            ResourceType::Script,
            "id,id,NULL::uuid,name",
            "'Unknown'",
            "scripts",
        ),
    ];
    for (kind, permission, identity, state, source) in resources {
        let include = match f.read {
            FleetRead::Attention | FleetRead::Summary | FleetRead::OpenAlerts => {
                kind != "Container"
            }
            FleetRead::States | FleetRead::Members => matches!(
                kind,
                "Platform" | "Deployment" | "Stack" | "SwarmService" | "Container"
            ),
        };
        if !include || (!f.kinds.is_empty() && !f.kinds.iter().any(|k| k == kind)) {
            continue;
        }
        // Carry the recorded comparison with the authorized row. Rejoining the workload
        // table here cost one extra lookup per healthy workload on a large fleet.
        let comparison = match (f.read, kind) {
            (FleetRead::Attention, "Deployment") => {
                "d.autoupdatestate_lastcheckedat,CASE WHEN d.autoupdatestate_status='UpdateAvailable' AND nullif(d.autoupdatestate_currentdigest,'')=d.spec->'Image'->>'ResolvedDigest' AND nullif(d.autoupdatestate_remotedigest,'')<>d.autoupdatestate_currentdigest THEN d.autoupdatestate_remotedigest END"
            }
            (FleetRead::Attention, "SwarmService") => {
                "s.autoupdatestate_lastcheckedat,CASE WHEN s.autoupdatestate_status='UpdateAvailable' AND nullif(s.autoupdatestate_currentdigest,'')=s.appliedimagedigest AND nullif(s.autoupdatestate_remotedigest,'')<>s.autoupdatestate_currentdigest THEN s.autoupdatestate_remotedigest END"
            }
            _ => "NULL::timestamptz,NULL::text",
        };
        branches.push(format!(
            "SELECT '{kind}',{},{identity},{state},{comparison} FROM {source}",
            permission as i32
        ));
    }
    if branches.is_empty() {
        branches.push("SELECT NULL::text,0,NULL::uuid,NULL::uuid,NULL::uuid,NULL::text,NULL::text,NULL::timestamptz,NULL::text WHERE false".to_owned());
    }
    let mut sql = include_str!("../enabled_actor_scope.sql").to_owned();
    sql.push_str(include_str!("fleet_scope.sql"));
    sql.push_str(&branches.join(" UNION ALL "));
    sql.push_str(r#"), visible AS (
       SELECT r.* FROM population r WHERE EXISTS(SELECT FROM enabled_actor)
       AND ($2 OR r.permission IN (SELECT permission FROM global_grants))
       UNION ALL
       SELECT r.* FROM grants g JOIN population r ON (r.permission,r.owner_id)=(g.permission,g.id)
       WHERE NOT $2 AND g.permission NOT IN (SELECT permission FROM global_grants)
    ), owners AS (
       SELECT r.* FROM visible r WHERE (cardinality($3::text[])=0 OR (r.kind,r.id) IN (SELECT kind,id FROM tagged))
    )"#);
    if matches!(
        f.read,
        FleetRead::Summary | FleetRead::Attention | FleetRead::OpenAlerts
    ) {
        sql.push_str(include_str!("fleet_alerts.sql"));
    }
    let result = match f.read {
        FleetRead::Summary => {
            ", groups AS (SELECT kind,state,count(*) AS count FROM owners WHERE kind IN ('Platform','Deployment','Stack','SwarmService') GROUP BY kind,state) SELECT jsonb_build_object('states',coalesce((SELECT jsonb_agg(to_jsonb(g) ORDER BY kind,state) FROM groups g),'[]'),'alerts',(SELECT count(*) FROM events),'rows','[]'::jsonb,'total',(SELECT count(*) FROM owners),'problems',0,'updates',0)"
        }
        FleetRead::States => {
            ", groups AS (SELECT kind,state,count(*) AS count FROM owners GROUP BY kind,state) SELECT jsonb_build_object('states',coalesce((SELECT jsonb_agg(to_jsonb(g) ORDER BY count DESC,state) FROM groups g),'[]'),'alerts',0,'rows','[]'::jsonb,'total',0,'problems',0,'updates',0)"
        }
        FleetRead::Members => {
            ", members AS (SELECT * FROM owners WHERE $4::text IS NULL OR state=$4), page AS (SELECT kind,id,platform_id AS \"platformId\",name,state,'[]'::jsonb AS reasons FROM members ORDER BY name,id LIMIT $5 OFFSET $6) SELECT jsonb_build_object('states','[]'::jsonb,'alerts',0,'rows',coalesce((SELECT jsonb_agg(to_jsonb(p) ORDER BY name,id) FROM page p),'[]'),'total',(SELECT count(*) FROM members),'problems',0,'updates',0)"
        }
        FleetRead::OpenAlerts => ", page AS (SELECT e.kind,e.target AS id,o.name,e.severity AS state,e.createdat FROM events e JOIN owners o USING(kind,id) ORDER BY e.createdat DESC,e.target LIMIT $4 OFFSET $5)
            SELECT jsonb_build_object('states','[]'::jsonb,'alerts',0,'total',(SELECT count(*) FROM events),'problems',0,'updates',0,'rows',coalesce((SELECT jsonb_agg((to_jsonb(p)-'createdat')||jsonb_build_object('reasons','[]'::jsonb) ORDER BY createdat DESC,id) FROM page p),'[]'))",
        FleetRead::Attention => include_str!("attention.sql"),
    };
    sql.push_str(result);
    let mut query = sqlx::query_scalar::<_, Json<FleetPage>>(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(viewer.actor.value())
        .bind(viewer.administrator)
        .bind(f.tags);
    match f.read {
        FleetRead::OpenAlerts => query = query.bind(f.limit).bind(f.offset),
        FleetRead::Members => query = query.bind(f.state).bind(f.limit).bind(f.offset),
        FleetRead::Attention => {
            query = query
                .bind(match f.tab {
                    AttentionTab::All => "All",
                    AttentionTab::Problems => "Problems",
                    AttentionTab::Updates => "Updates",
                })
                .bind(f.limit)
                .bind(f.offset)
        }
        _ => {}
    }
    query
        .fetch_one(pool)
        .await
        .map(|v| v.0)
        .map_err(|e| DashboardError::Storage(e.to_string()))
}
