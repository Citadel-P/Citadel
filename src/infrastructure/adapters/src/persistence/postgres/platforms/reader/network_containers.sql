
, actual AS (
    SELECT c.id,c.dockercontainerid,c.dockernodeid,c.name,c.state,c.projectionstalesince,c.hascitadelownershiplabels,
        COALESCE(c.deploymentid,c.stackid,service.id,projection.stackid) ownerid,
        CASE WHEN c.deploymentid IS NOT NULL THEN 1 WHEN c.stackid IS NOT NULL THEN 2
            WHEN service.id IS NOT NULL THEN 20 WHEN projection.stackid IS NOT NULL THEN 2 END kind
    FROM containers c
    JOIN platforms platform ON platform.id=c.platformid
    -- Manager containers use daemon scope; task projections retain the actual node ID.
    LEFT JOIN swarmtaskprojections task ON task.platformid=c.platformid
        AND NULLIF(task.dockernodeid,'') IS NOT NULL
        AND c.dockernodeid IS NOT DISTINCT FROM (CASE
            WHEN task.dockernodeid=NULLIF(COALESCE(platform.platformdescriptor->>'nodeID',platform.platformdescriptor->>'NodeID'),'') THEN NULL
            ELSE task.dockernodeid END)
        AND task.dockercontainerid=c.dockercontainerid
    LEFT JOIN swarmservices service ON service.platformid=c.platformid AND service.dockerserviceid=task.dockerserviceid
    LEFT JOIN swarmserviceprojections projection ON projection.platformid=c.platformid AND projection.dockerserviceid=task.dockerserviceid
    WHERE c.platformid=$3 AND ($4::uuid IS NULL OR c.id=$4)
)
SELECT a.id,a.dockercontainerid,a.dockernodeid,a.name,a.state,a.projectionstalesince IS NOT NULL isstale,w.kind,w.id ownerid,w.name ownername,w.status ownerstatus,COALESCE(w.controlstate='Processing',false) ownerprocessing
FROM actual a LEFT JOIN authorized w ON w.kind=a.kind AND w.id=a.ownerid
WHERE (a.ownerid IS NULL AND NOT a.hascitadelownershiplabels) OR w.id IS NOT NULL
ORDER BY a.name,a.id LIMIT 501
