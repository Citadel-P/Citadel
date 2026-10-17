,s.controlstate
    FROM swarmservices s
    LEFT JOIN swarmserviceprojections projection ON projection.platformid=s.platformid AND projection.dockerserviceid=s.dockerserviceid
    WHERE s.platformid=$3
), authorized AS (
    SELECT * FROM workloads w WHERE $2 OR EXISTS (SELECT 1 FROM grants g
        WHERE g.kind=w.kind AND (g.resourceid IS NULL OR g.resourceid=w.id) AND g.permissionlevel IN (1,2,4))
)
