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
), owners AS (
    SELECT c.id,1 kind,d.id ownerid,d.name
    FROM containers c JOIN deployments d ON d.id=c.deploymentid AND d.platformid=c.platformid
    WHERE c.id=ANY($3)
    UNION
    SELECT c.id,2,s.id,s.name
    FROM containers c JOIN stacks s ON s.id=c.stackid
    JOIN stackreleases r ON r.id=s.currentstackreleaseid AND r.platformid=c.platformid
    WHERE c.id=ANY($3)
    UNION
    SELECT c.id,20,s.id,s.name
    FROM containers c
    JOIN swarmtaskprojections task ON task.platformid=c.platformid
        AND task.dockernodeid=c.dockernodeid AND task.dockercontainerid=c.dockercontainerid
    JOIN swarmservices s ON s.platformid=c.platformid AND s.dockerserviceid=task.dockerserviceid
    WHERE c.id=ANY($3)
    UNION
    SELECT c.id,2,s.id,s.name
    FROM containers c
    JOIN swarmtaskprojections task ON task.platformid=c.platformid
        AND task.dockernodeid=c.dockernodeid AND task.dockercontainerid=c.dockercontainerid
    JOIN swarmserviceprojections service ON service.platformid=c.platformid AND service.dockerserviceid=task.dockerserviceid
    JOIN stacks s ON s.id=service.stackid
    JOIN stackreleases r ON r.id=s.currentstackreleaseid AND r.platformid=c.platformid
    WHERE c.id=ANY($3)
)
SELECT id,array_agg(DISTINCT name ORDER BY name) names FROM owners o
WHERE $2 OR EXISTS (SELECT 1 FROM grants g WHERE g.kind=o.kind
    AND (g.resourceid IS NULL OR g.resourceid=o.ownerid) AND g.permissionlevel IN (1,2,4))
GROUP BY id
