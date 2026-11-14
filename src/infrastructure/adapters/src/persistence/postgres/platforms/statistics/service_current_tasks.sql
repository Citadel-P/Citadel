-- Resolve physical identity before seeking the latest eligible raw/rollup timestamp.
WITH current_tasks AS (
    SELECT DISTINCT ON (CASE WHEN slot IS NOT NULL THEN 'slot:'||slot::text ELSE 'node:'||dockernodeid END)
        dockernodeid,dockercontainerid
    FROM swarmtaskprojections WHERE platformid=$1 AND dockerserviceid=$2 AND NOT isstale
        AND lower(state)='running' AND lower(desiredstate)='running'
    ORDER BY CASE WHEN slot IS NOT NULL THEN 'slot:'||slot::text ELSE 'node:'||dockernodeid END,
        COALESCE(statustimestamp,observedat) DESC,dockertaskid DESC LIMIT 501
)
SELECT task.dockernodeid,container.id,latest.created
FROM current_tasks task JOIN platforms platform ON platform.id=$1
LEFT JOIN LATERAL (
    SELECT CASE WHEN COUNT(*)=1 THEN (array_agg(candidate.id))[1] END AS id
    FROM (SELECT id FROM containers
        WHERE platformid=$1 AND task.dockernodeid=COALESCE(dockernodeid,
            CASE WHEN platform.connectortype IN ('Local','Agent') THEN platform.platformdescriptor::jsonb->>'nodeID' END)
        AND dockercontainerid=task.dockercontainerid AND projectionstalesince IS NULL LIMIT 2) candidate
) container ON true
LEFT JOIN LATERAL (
    SELECT latest.created FROM statisticssources src
    CROSS JOIN LATERAL (
        -- Keep sourceid in the ordering to use the composite index on misses.
        (SELECT created FROM statisticssamples
         WHERE sourceid>=src.id AND sourceid<=src.id AND created<=$3
         ORDER BY sourceid DESC,created DESC LIMIT 1)
        UNION ALL
        (SELECT created FROM statisticsrollups
         WHERE sourceid>=src.id AND sourceid<=src.id AND created<=$3
         ORDER BY sourceid DESC,created DESC LIMIT 1)
        ORDER BY created DESC LIMIT 1
    ) latest WHERE src.containerid=container.id OFFSET 0
) latest ON true
