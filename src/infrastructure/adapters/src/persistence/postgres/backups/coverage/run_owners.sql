-- A run affects its policy's workload and other workloads using the same storage.
-- Old run items matter too: retention can expire a previously available snapshot.
WITH policies AS MATERIALIZED (
    SELECT source::jsonb AS source FROM backuppolicies WHERE id=ANY($2)
), policy_owners AS (
    SELECT kind, owner FROM policies CROSS JOIN LATERAL (VALUES
        ('Deployment', COALESCE(source->>'deploymentId',source->>'DeploymentId')),
        ('Stack', COALESCE(source->>'stackId',source->>'StackId')),
        ('SwarmService', COALESCE(source->>'swarmServiceId',source->>'SwarmServiceId'))
    ) owner(kind,owner) WHERE owner IS NOT NULL
), containers_on_platform AS MATERIALIZED (
    SELECT c.id,c.mounts,c.dockernodeid,c.deploymentid,
        CASE WHEN c.dockernodeid IS NULL THEN c.stackid ELSE service.stackid END AS stackid,
        COALESCE(service.swarmserviceid,managed.id) AS serviceid
    FROM containers c
    LEFT JOIN swarmtaskprojections task ON task.platformid=c.platformid AND task.dockercontainerid=c.dockercontainerid
        AND task.dockernodeid=c.dockernodeid AND lower(task.desiredstate)='running'
    LEFT JOIN swarmserviceprojections service ON service.platformid=task.platformid AND service.dockerserviceid=task.dockerserviceid
    LEFT JOIN swarmservices managed ON managed.platformid=service.platformid AND managed.dockerserviceid=service.dockerserviceid
    WHERE c.platformid=$1
), storage AS MATERIALIZED (
    SELECT owner.kind,owner.id,c.dockernodeid AS node,
        lower(fields.kind)='bind' AS bind,
        CASE lower(fields.kind) WHEN 'bind' THEN fields.path WHEN 'volume' THEN fields.name END AS name
    FROM containers_on_platform c
    CROSS JOIN LATERAL jsonb_array_elements(CASE WHEN jsonb_typeof(c.mounts)='array' THEN c.mounts ELSE '[]' END) mount
    CROSS JOIN LATERAL (
        SELECT max(value) FILTER (WHERE lower(key)='type') AS kind,
               max(value) FILTER (WHERE lower(key)='name') AS name,
               max(value) FILTER (WHERE lower(key)='source') AS path
        FROM jsonb_each_text(CASE WHEN jsonb_typeof(mount)='object' THEN mount ELSE '{}' END)
    ) fields
    CROSS JOIN LATERAL (VALUES ('Container',c.id),('Deployment',c.deploymentid),('Stack',c.stackid),('SwarmService',c.serviceid)) owner(kind,id)
    WHERE owner.id IS NOT NULL AND lower(fields.kind) IN ('bind','volume')
    UNION ALL
    SELECT 'Deployment',d.id,NULL,false,btrim(split_part(mount,':',1))
    FROM deployments d CROSS JOIN LATERAL (SELECT COALESCE(d.spec->'Volumes',d.spec->'volumes') AS volumes) v
    CROSS JOIN LATERAL jsonb_array_elements_text(CASE WHEN jsonb_typeof(v.volumes)='array' THEN v.volumes ELSE '[]' END) mount
    WHERE d.platformid=$1
), targets AS (
    SELECT COALESCE(source->>'dockerNodeId',source->>'DockerNodeId') AS node,
        source->>'$type'='BindMount' AS bind,
        COALESCE(source->>'volumeName',source->>'VolumeName',source->>'hostPath',source->>'HostPath') AS name
    FROM policies WHERE COALESCE(source->>'platformId',source->>'PlatformId')=$1::uuid::text
    UNION
    SELECT s.node,s.bind,s.name FROM storage s JOIN policy_owners p ON p.kind=s.kind AND p.owner=s.id::text
    UNION
    SELECT i.dockernodeid,i.hostpath IS NOT NULL,COALESCE(i.hostpath,i.volumename)
    FROM backupruns r JOIN backuprunitems i ON i.backuprunid=r.id WHERE r.backuppolicyid=ANY($2) AND i.platformid=$1
)
SELECT s.id,s.kind FROM storage s JOIN targets t ON t.name=s.name AND t.bind=s.bind AND t.node IS NOT DISTINCT FROM s.node
UNION
SELECT workloads.id,workloads.kind FROM (
    SELECT d.id,'Deployment' AS kind FROM deployments d WHERE d.platformid=$1
    UNION ALL SELECT s.id,'Stack' FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE r.platformid=$1
    UNION ALL SELECT id,'SwarmService' FROM swarmservices WHERE platformid=$1
) workloads JOIN policy_owners p ON p.owner=workloads.id::text AND p.kind=workloads.kind
