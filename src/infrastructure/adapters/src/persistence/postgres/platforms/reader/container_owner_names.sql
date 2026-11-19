owners AS (
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
WHERE $2
   OR o.kind IN (SELECT kind FROM readable_types)
   OR (o.kind, o.ownerid) IN (SELECT kind, resourceid FROM readable_resources)
GROUP BY id
