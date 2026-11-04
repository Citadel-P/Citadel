use super::*;
use citadel_updates::{AgentUpdateKind, AgentUpdateTarget};
#[derive(Clone)]
pub(crate) struct Registration {
    pub id: Uuid,
    pub name: String,
    pub kind: AgentUpdateKind,
    pub provider: Option<serde_json::Value>,
    pub previous: Option<AgentUpdateTarget>,
}
pub(crate) async fn registrations(pool: &PgPool) -> Result<Vec<Registration>, UpdateError> {
    let cached = sqlx::query_scalar::<_, sqlx::types::Json<Vec<AgentUpdateTarget>>>(
        "SELECT observations FROM agentupdatesettings WHERE id=1",
    )
    .fetch_one(pool)
    .await
    .map_err(storage)?
    .0;
    let rows=sqlx::query("SELECT id,name,'Platform' AS kind,NULL::jsonb AS provider FROM platforms WHERE connectortype IN ('Agent','EdgeAgent') UNION ALL SELECT id,name,'BuildPool',providerspec FROM buildagentpools WHERE provider='SelfManagedVm' AND archivedat IS NULL AND enabled UNION ALL SELECT p.id,p.name,'SwarmService',NULL::jsonb FROM platforms p JOIN swarmnodeagentinstallations i ON i.platformid=p.id WHERE i.desiredstate='Installed' ORDER BY id,kind LIMIT 501").fetch_all(pool).await.map_err(storage)?;
    if rows.len() > 500 {
        return Err(UpdateError::Unavailable(
            "Too many Agent registrations to inspect in one check.".into(),
        ));
    }
    rows.into_iter()
        .map(|r| {
            let id: Uuid = r.try_get("id").map_err(storage)?;
            let kind = match r.try_get::<String, _>("kind").map_err(storage)?.as_str() {
                "BuildPool" => AgentUpdateKind::BuildPool,
                "SwarmService" => AgentUpdateKind::SwarmService,
                _ => AgentUpdateKind::Platform,
            };
            Ok(Registration {
                id,
                name: r.try_get("name").map_err(storage)?,
                kind,
                provider: r.try_get("provider").map_err(storage)?,
                previous: cached
                    .iter()
                    .find(|v| v.resource_id == id && v.kind == kind)
                    .cloned(),
            })
        })
        .collect()
}
pub(crate) async fn remember(
    pool: &PgPool,
    targets: &[AgentUpdateTarget],
) -> Result<(), UpdateError> {
    sqlx::query("UPDATE agentupdatesettings SET observations=$1 WHERE id=1")
        .bind(sqlx::types::Json(targets))
        .execute(pool)
        .await
        .map_err(storage)?;
    Ok(())
}
pub(crate) async fn swarm(
    pool: &PgPool,
    r: &Registration,
) -> Result<AgentUpdateTarget, UpdateError> {
    let row=sqlx::query("SELECT NOT EXISTS(SELECT 1 FROM swarmnodeprojections n WHERE n.platformid=i.platformid AND n.isstale) AS fresh,i.dockerserviceid,i.agentimagedigest,p.status,i.clusterid,i.operationstate FROM swarmnodeagentinstallations i JOIN platforms p ON p.id=i.platformid WHERE i.platformid=$1 AND i.desiredstate='Installed'").bind(r.id).fetch_one(pool).await.map_err(storage)?;
    let versions=sqlx::query_as::<_,(Option<String>,bool)>("SELECT lastseenversion,connectionstatus='Connected' AND lastheartbeatatutc>now()-interval '90 seconds' FROM edgeagentbindings WHERE platformid=$1 AND profile='SwarmNode' AND revokedatutc IS NULL").bind(r.id).fetch_all(pool).await.map_err(storage)?;
    let drain_nodes = swarm_nodes(pool, r.id).await?;
    let expected = drain_nodes.len();
    let connected = row.try_get::<bool, _>("fresh").map_err(storage)?
        && expected == versions.len()
        && !versions.is_empty()
        && versions.iter().all(|(_, connected)| *connected)
        && row.try_get::<String, _>("status").map_err(storage)? == "Online";
    let version = if versions.iter().all(|(version, _)| {
        version
            .as_ref()
            .is_some_and(|v| citadel_updates::release_version(v).is_ok())
    }) && versions.iter().all(|(v, _)| v == &versions[0].0)
    {
        versions
            .iter()
            .filter_map(|(v, _)| v.as_ref())
            .filter_map(|v| {
                citadel_updates::release_version(v)
                    .ok()
                    .map(|parsed| (parsed, v))
            })
            .min_by(|a, b| a.0.cmp(&b.0))
            .map(|(_, v)| v.clone())
    } else {
        None
    };
    let digest: String = row.try_get("agentimagedigest").map_err(storage)?;
    let service: Option<String> = row.try_get("dockerserviceid").map_err(storage)?;
    let busy = row
        .try_get::<Option<String>, _>("operationstate")
        .map_err(storage)?
        .as_deref()
        == Some("Running");
    Ok(AgentUpdateTarget {
        id: format!("swarm:{}", r.id),
        resource_id: r.id,
        kind: r.kind,
        name: format!("{} · node-agent service", r.name),
        installed_version: version,
        connected: connected && !busy,
        supported: service.is_some() && !digest.is_empty(),
        reason: busy.then(|| "Another node-agent operation is running.".into()),
        nodes: expected as u32,
        drain_nodes,
        revision: format!("{}:{}", service.unwrap_or_default(), digest),
    })
}

pub(crate) async fn swarm_nodes(pool: &PgPool, id: Uuid) -> Result<Vec<String>, UpdateError> {
    sqlx::query_scalar("SELECT n.dockernodeid FROM swarmnodeprojections n JOIN swarmnodeagentinstallations i USING(platformid) WHERE n.platformid=$1 AND n.dockernodeid<>i.managerdockernodeid AND lower(n.availability)='active' AND lower(n.operatingsystem)='linux' ORDER BY n.dockernodeid").bind(id).fetch_all(pool).await.map_err(storage)
}
