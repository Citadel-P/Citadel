use super::*;
use citadel_backups::policies::read_models::BackupPreviewKind as Kind;
use std::collections::{BTreeMap, BTreeSet};

pub(super) async fn read(
    pool: &PgPool,
    platforms: &[Uuid],
    snapshot: &mut CoverageSnapshot,
) -> Result<(), BackupError> {
    let rows=sqlx::query("SELECT id,platformid,'Deployment' AS kind,spec FROM deployments WHERE platformid=ANY($1) UNION ALL SELECT s.id,r.platformid,'Stack',NULL FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE r.platformid=ANY($1) UNION ALL SELECT id,platformid,'SwarmService',NULL FROM swarmservices WHERE platformid=ANY($1)")
        .bind(platforms).fetch_all(pool).await.map_err(storage)?;
    let mut declared = BTreeMap::<Uuid, BTreeSet<String>>::new();
    for row in rows {
        let id = row.try_get("id").map_err(storage)?;
        let kind = match row.try_get::<&str, _>("kind").map_err(storage)? {
            "Deployment" => Kind::Deployment,
            "Stack" => Kind::Stack,
            _ => Kind::SwarmService,
        };
        if kind == Kind::Deployment {
            let spec = citadel_deployments::DeploymentSpec::from_storage_value(
                row.try_get("spec").map_err(storage)?,
            )
            .map_err(storage)?;
            declared.insert(
                id,
                spec.volumes
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|s| super::super::source_planner::named_deployment_volume(s))
                    .collect(),
            );
        }
        snapshot.workloads.push(WorkloadStorage {
            kind,
            id,
            platform_id: row.try_get("platformid").map_err(storage)?,
            volumes: vec![],
            complete: false,
        });
    }
    // Current desired tasks retain their owner-node identity even when inventory is incomplete.
    let rows=sqlx::query(r#"SELECT c.id,c.platformid,c.dockercontainerid,c.dockernodeid,c.mounts,c.deploymentid,
        CASE WHEN c.dockernodeid IS NULL THEN c.stackid ELSE s.stackid END AS stackid,COALESCE(s.swarmserviceid,managed.id) AS swarmserviceid,
        (c.projectionstalesince IS NOT NULL OR COALESCE(t.isstale,false) OR COALESCE(s.isstale,false)) AS stale
        FROM containers c
        LEFT JOIN swarmtaskprojections t ON t.platformid=c.platformid AND t.dockercontainerid=c.dockercontainerid AND t.dockernodeid=c.dockernodeid AND lower(t.desiredstate)='running'
        LEFT JOIN swarmserviceprojections s ON s.platformid=t.platformid AND s.dockerserviceid=t.dockerserviceid
        LEFT JOIN swarmservices managed ON managed.platformid=s.platformid AND managed.dockerserviceid=s.dockerserviceid
        WHERE c.platformid=ANY($1)"#).bind(platforms).fetch_all(pool).await.map_err(storage)?;
    let mut incomplete = BTreeSet::new();
    let mut deployment_counts = BTreeMap::<Uuid, usize>::new();
    for row in rows {
        let platform = row.try_get("platformid").map_err(storage)?;
        let node: Option<String> = row.try_get("dockernodeid").map_err(storage)?;
        let mounts: Option<Value> = row.try_get("mounts").map_err(storage)?;
        let complete = mounts.as_ref().is_some_and(Value::is_array)
            && !row.try_get::<bool, _>("stale").map_err(storage)?;
        let mut targets = mounts
            .as_ref()
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|m| {
                let kind = field(m, "type")?;
                let host = kind.eq_ignore_ascii_case("bind");
                if !host && !kind.eq_ignore_ascii_case("volume") {
                    return None;
                }
                let name = field(m, if host { "source" } else { "name" })?.to_owned();
                if name.is_empty() {
                    return None;
                }
                Some(ObservedStorage {
                    target: StorageTarget {
                        platform_id: platform,
                        name: name.clone(),
                        host_path: host.then_some(name),
                        docker_node_id: node.clone(),
                    },
                    included: false,
                })
            })
            .collect::<Vec<_>>();
        targets.sort_by(|a, b| a.target.cmp(&b.target));
        targets.dedup_by(|a, b| a.target == b.target);
        for (kind, column) in [
            (Kind::Deployment, "deploymentid"),
            (Kind::Stack, "stackid"),
            (Kind::SwarmService, "swarmserviceid"),
        ] {
            if let Some(id) = row.try_get::<Option<Uuid>, _>(column).map_err(storage)? {
                if !complete {
                    incomplete.insert(id);
                }
                if kind == Kind::Deployment {
                    let count = deployment_counts.entry(id).or_default();
                    *count += 1;
                    if *count > 1 {
                        incomplete.insert(id);
                    }
                }
                if let Some(w) = snapshot
                    .workloads
                    .iter_mut()
                    .find(|w| w.kind == kind && w.id == id)
                {
                    w.complete = true;
                    for t in &targets {
                        let mut t = t.clone();
                        t.included = citadel_backups::coverage::workload_includes(
                            kind,
                            &t.target,
                            declared
                                .get(&id)
                                .is_some_and(|d| d.contains(&t.target.name)),
                        );
                        if !w.volumes.iter().any(|v| v.target == t.target) {
                            w.volumes.push(t);
                        }
                    }
                }
            }
        }
        snapshot.containers.push(ContainerStorage {
            id: row.try_get("id").map_err(storage)?,
            platform_id: platform,
            docker_id: row.try_get("dockercontainerid").map_err(storage)?,
            docker_node_id: node,
            volumes: targets,
            complete,
        });
    }
    // A missing current Swarm task must not allow the remaining node to appear fully protected.
    let missing=sqlx::query("SELECT DISTINCT s.stackid,s.swarmserviceid FROM swarmtaskprojections t JOIN swarmserviceprojections s USING(platformid,dockerserviceid) LEFT JOIN containers c ON c.platformid=t.platformid AND c.dockercontainerid=t.dockercontainerid AND c.dockernodeid=t.dockernodeid WHERE t.platformid=ANY($1) AND lower(t.desiredstate)='running' AND (c.id IS NULL OR t.isstale OR s.isstale OR lower(t.state)<>'running')")
        .bind(platforms).fetch_all(pool).await.map_err(storage)?;
    for row in missing {
        for column in ["stackid", "swarmserviceid"] {
            if let Some(id) = row.try_get::<Option<Uuid>, _>(column).map_err(storage)? {
                incomplete.insert(id);
            }
        }
    }
    let unstable=sqlx::query("SELECT s.stackid,COALESCE(s.swarmserviceid,m.id) AS swarmserviceid FROM swarmserviceprojections s LEFT JOIN swarmservices m ON m.platformid=s.platformid AND m.dockerserviceid=s.dockerserviceid WHERE s.platformid=ANY($1) AND (s.isstale OR s.desiredtaskcount<=0 OR s.desiredtaskcount<>s.runningtaskcount OR s.desiredtaskcount<>(SELECT count(*) FROM swarmtaskprojections t WHERE t.platformid=s.platformid AND t.dockerserviceid=s.dockerserviceid AND lower(t.desiredstate)='running'))")
        .bind(platforms).fetch_all(pool).await.map_err(storage)?;
    for row in unstable {
        for column in ["stackid", "swarmserviceid"] {
            if let Some(id) = row.try_get::<Option<Uuid>, _>(column).map_err(storage)? {
                incomplete.insert(id);
            }
        }
    }
    // Applied standalone bindings preserve volume identity when containers are temporarily absent.
    let bindings=sqlx::query("SELECT s.id,b.platformid,b.volumename FROM stacks s JOIN stackreleasevolumebindings b ON b.stackreleaseid=s.currentstackreleaseid WHERE b.platformid=ANY($1)")
        .bind(platforms).fetch_all(pool).await.map_err(storage)?;
    for row in bindings {
        let id: Uuid = row.try_get("id").map_err(storage)?;
        if let Some(w) = snapshot
            .workloads
            .iter_mut()
            .find(|w| w.kind == Kind::Stack && w.id == id)
        {
            let target = StorageTarget {
                platform_id: w.platform_id,
                name: row.try_get("volumename").map_err(storage)?,
                host_path: None,
                docker_node_id: None,
            };
            if !w.volumes.iter().any(|v| v.target == target) {
                w.volumes.push(ObservedStorage {
                    target,
                    included: true,
                });
            }
        }
    }
    let offline: Vec<Uuid> =
        sqlx::query_scalar("SELECT id FROM platforms WHERE id=ANY($1) AND status<>'Online'")
            .bind(platforms)
            .fetch_all(pool)
            .await
            .map_err(storage)?;
    for c in &mut snapshot.containers {
        if offline.contains(&c.platform_id) {
            c.complete = false;
        }
    }
    for w in &mut snapshot.workloads {
        if incomplete.contains(&w.id) || offline.contains(&w.platform_id) {
            w.complete = false;
        }
        if w.kind == Kind::Deployment {
            for name in declared.get(&w.id).into_iter().flatten() {
                let target = StorageTarget {
                    platform_id: w.platform_id,
                    name: name.clone(),
                    host_path: None,
                    docker_node_id: None,
                };
                if !w.volumes.iter().any(|v| v.target == target) {
                    w.volumes.push(ObservedStorage {
                        target,
                        included: true,
                    });
                }
            }
        }
    }
    Ok(())
}
fn field<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.as_object()?
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))?
        .1
        .as_str()
}
