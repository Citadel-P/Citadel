WITH actor_scope AS (
    SELECT id actorid FROM actors WHERE id=$1 AND isenabled
    UNION SELECT team.actorid FROM actorteammemberships membership
    JOIN teams team ON team.id=membership.teamid
    JOIN actors a ON a.id=team.actorid AND a.isenabled WHERE membership.memberactorid=$1
), grants AS (
    SELECT p.resourcetype kind,NULL::uuid resourceid,p.permissionlevel
    FROM actor_scope scope JOIN actorroles ar ON ar.actorid=scope.actorid
    JOIN permissions p ON p.roleid=ar.roleid
    UNION ALL SELECT a.resourcetype,a.resourceid,a.permissionlevel
    FROM actor_scope scope JOIN resourceaccesses a ON a.actorid=scope.actorid
), workloads AS (
    SELECT 1 kind,id,name FROM deployments WHERE platformid=$3
    UNION ALL SELECT 2,s.id,s.name FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE r.platformid=$3
    UNION ALL SELECT 20,id,name FROM swarmservices WHERE platformid=$3
), authorized AS (
    SELECT * FROM workloads w WHERE $2 OR EXISTS(SELECT 1 FROM grants g
        WHERE g.kind=w.kind AND (g.resourceid IS NULL OR g.resourceid=w.id) AND g.permissionlevel IN (1,2,4))
), actual AS (
    SELECT DISTINCT c.dockerimageid,c.dockernodeid,c.dockercontainerid,c.name,c.state,c.projectionobservedat,c.projectionstalesince,
        COALESCE(c.deploymentid,c.stackid,service.id,stack.id) ownerid,
        CASE WHEN c.deploymentid IS NOT NULL THEN 1 WHEN c.stackid IS NOT NULL THEN 2
             WHEN service.id IS NOT NULL THEN 20 WHEN stack.id IS NOT NULL THEN 2 END kind
    FROM containers c
    LEFT JOIN swarmtaskprojections task ON task.platformid=c.platformid AND task.dockernodeid=c.dockernodeid AND task.dockercontainerid=c.dockercontainerid
    LEFT JOIN swarmservices service ON service.platformid=c.platformid AND service.dockerserviceid=task.dockerserviceid
    LEFT JOIN swarmserviceprojections projection ON projection.platformid=c.platformid AND projection.dockerserviceid=task.dockerserviceid
    LEFT JOIN stacks stack ON stack.id=projection.stackid
    WHERE c.platformid=$3 AND c.dockerimageid=ANY($4)
), candidates AS (
    SELECT 'Container' relationship,a.kind,a.ownerid,w.name ownername,a.dockercontainerid containerid,
        a.name containername,a.state,NULL::text reference,to_timestamp(a.projectionobservedat/1000.0) observedat,
        a.projectionstalesince IS NOT NULL OR a.projectionobservedat IS NULL isstale,NULL::text compose,NULL::text registryhost,a.dockerimageid,a.dockernodeid
    FROM actual a LEFT JOIN authorized w ON w.kind=a.kind AND w.id=a.ownerid
    WHERE a.ownerid IS NULL OR w.id IS NOT NULL
    UNION ALL
    SELECT CASE WHEN d.spec->'Image'->>'$type'='Local' THEN 'Local' ELSE 'Reference' END,
        1,d.id,w.name,NULL,NULL,NULL,COALESCE(d.spec->'Image'->>'ImageTag',d.spec->'Image'->>'ImageId'),NULL,false,NULL,registry.registryhost,NULL,NULL
    FROM deployments d JOIN authorized w ON w.kind=1 AND w.id=d.id
    LEFT JOIN registries registry ON registry.id::text=d.spec->'Image'->>'RegistryId'
    WHERE d.spec->'Image'->>'$type'='Local' OR d.spec->'Image'->>'ImageTag' IS NOT NULL
    UNION ALL
    SELECT 'Reference',20,s.id,w.name,NULL,NULL,NULL,s.spec->'image'->>'imageTag',NULL,false,NULL,registry.registryhost,NULL,NULL
    FROM swarmservices s JOIN authorized w ON w.kind=20 AND w.id=s.id
    LEFT JOIN registries registry ON registry.id::text=s.spec->'image'->>'registryId'
    WHERE s.spec->'image'->>'imageTag' IS NOT NULL
    UNION ALL
    SELECT 'Compose',2,s.id,w.name,NULL,NULL,NULL,NULL,NULL,false,r.spec->>'composeFile',NULL,NULL,NULL
    FROM stacks s JOIN authorized w ON w.kind=2 AND w.id=s.id
    JOIN stackreleases r ON r.id=s.currentstackreleaseid
    WHERE r.spec->>'composeFile' IS NOT NULL
)
SELECT * FROM candidates ORDER BY relationship,kind,ownerid,containerid LIMIT 10001
