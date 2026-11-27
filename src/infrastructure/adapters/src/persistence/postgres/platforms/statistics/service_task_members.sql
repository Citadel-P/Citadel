, current_tasks AS (
    SELECT DISTINCT ON (CASE WHEN slot IS NOT NULL THEN 'slot:'||slot::text ELSE 'node:'||dockernodeid END)
        dockernodeid, dockercontainerid
    FROM swarmtaskprojections
    WHERE (platformid,dockerserviceid) = (SELECT platformid,dockerserviceid FROM scope)
        AND NOT isstale AND lower(state)='running' AND lower(desiredstate)='running'
    ORDER BY CASE WHEN slot IS NOT NULL THEN 'slot:'||slot::text ELSE 'node:'||dockernodeid END,
        COALESCE(statustimestamp,observedat) DESC, dockertaskid DESC
    LIMIT 501
), members AS (
    SELECT task.dockernodeid, task.dockercontainerid, container.id, container.node
    FROM current_tasks task JOIN platforms platform ON platform.id=(SELECT platformid FROM scope)
    LEFT JOIN LATERAL (
        SELECT CASE WHEN COUNT(*)=1 THEN (array_agg(candidate.id))[1] END AS id,
            min(candidate.dockernodeid) AS node
        FROM (
            SELECT id, dockernodeid FROM containers
            WHERE platformid=platform.id AND dockercontainerid=task.dockercontainerid
                AND task.dockernodeid=COALESCE(dockernodeid,
                    CASE WHEN platform.connectortype IN ('Local','Agent') THEN platform.platformdescriptor::jsonb->>'nodeID' END)
                AND projectionstalesince IS NULL
            LIMIT 2
        ) candidate
    ) container ON true
)
