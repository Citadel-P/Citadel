,s.controlstate
    FROM swarmservices s
    LEFT JOIN swarmserviceprojections projection ON projection.platformid=s.platformid AND projection.dockerserviceid=s.dockerserviceid
    WHERE s.platformid=$3
), authorized AS (
    SELECT * FROM workloads w WHERE $2
       OR w.kind IN (SELECT kind FROM readable_types)
       OR (w.kind, w.id) IN (SELECT kind, resourceid FROM readable_resources)
)
