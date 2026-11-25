use citadel_dashboards::SourceKind;

// The dashboard reports observed probes only. In particular, Compose replica
// expectations cannot be inferred from the containers currently in inventory.
const CONTAINER: &str = "SELECT c.state,c.health AS observation,
    (c.projectionstalesince IS NULL AND (c.dockernodeid IS NULL OR EXISTS(SELECT FROM swarmnoderuntimeprojectionstates ns WHERE ns.platformid=c.platformid AND ns.dockernodeid=c.dockernodeid AND NOT ns.isstale))) AS available";

macro_rules! swarm_probes {
    ($owner:literal) => {concat!(
        "SELECT coalesce(c.state,initcap(t.state)) AS state,c.health AS observation,
        (NOT t.isstale AND c.id IS NOT NULL AND c.projectionstalesince IS NULL AND (c.dockernodeid IS NULL OR EXISTS(SELECT FROM swarmnoderuntimeprojectionstates ns WHERE ns.platformid=t.platformid AND ns.dockernodeid=t.dockernodeid AND NOT ns.isstale))) AS available",
        include_str!("../swarm_health_tasks.sql"), $owner,
        include_str!("../swarm_health_container.sql")
    )};
}

pub(super) fn selected(kind: SourceKind) -> String {
    let rows = match kind {
        SourceKind::Container => format!("{CONTAINER} FROM containers c WHERE c.id=v.id"),
        SourceKind::Deployment => format!("{CONTAINER} FROM containers c WHERE c.platformid=v.platform_id AND c.deploymentid=v.id ORDER BY c.updated DESC,c.id DESC LIMIT 1"),
        SourceKind::SwarmService => swarm_probes!("task.platformid=v.platform_id AND task.dockerserviceid=v.docker_id
            AND EXISTS(SELECT FROM swarmserviceprojections sp WHERE sp.platformid=v.platform_id AND sp.dockerserviceid=v.docker_id AND sp.swarmserviceid=v.id AND sp.ownership<>'OwnershipConflict')").into(),
        SourceKind::Stack => format!("{CONTAINER} FROM containers c WHERE p.platformdescriptor->>'$type' IS DISTINCT FROM 'DockerSwarm'
            AND c.platformid=v.platform_id AND c.stackid=v.id AND NOT c.isswarmtask UNION ALL {}",
            swarm_probes!("p.platformdescriptor->>'$type'='DockerSwarm' AND task.platformid=v.platform_id AND EXISTS(SELECT FROM swarmserviceprojections sp WHERE sp.platformid=task.platformid AND sp.dockerserviceid=task.dockerserviceid AND sp.stackid=v.id AND sp.ownership<>'OwnershipConflict')")),
        SourceKind::Platform => unreachable!("workload reads exclude platforms"),
    };
    format!(
        "COALESCE((SELECT jsonb_agg(to_jsonb(probe)) FROM (SELECT * FROM ({rows}) members LIMIT 17) probe),'[]'::jsonb)"
    )
}
