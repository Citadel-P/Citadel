, requested AS (
    SELECT (ordinality-1)::integer network_index, value->>'scope' scope,
        value->>'docker_node_id' node, jsonb_array_elements_text(value->'container_ids') dockerid
    FROM jsonb_array_elements($4::jsonb) WITH ORDINALITY
), actual AS (
    SELECT DISTINCT r.network_index,c.id,c.dockercontainerid,c.dockernodeid,c.name,c.hascitadelownershiplabels,c.state,c.projectionstalesince,
        COALESCE(c.deploymentid,c.stackid,service.id,projection.stackid) ownerid,
        CASE WHEN c.deploymentid IS NOT NULL THEN 1 WHEN c.stackid IS NOT NULL THEN 2
            WHEN service.id IS NOT NULL THEN 20 WHEN projection.stackid IS NOT NULL THEN 2 END kind
    FROM requested r JOIN containers c ON c.platformid=$3 AND c.dockercontainerid=r.dockerid
        AND (c.dockernodeid IS NOT DISTINCT FROM r.node OR
            (r.node IS NULL AND r.scope='swarm' AND
                (SELECT count(*) FROM containers same WHERE same.platformid=$3 AND same.dockercontainerid=r.dockerid)=1))
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
), candidates AS (
    SELECT a.network_index,a.id containerid,a.dockercontainerid,a.dockernodeid,a.name containername,
        w.kind,w.id ownerid,w.name ownername,'[]'::jsonb refs,NULL::text compose,
        NULL::text dockerserviceid,NULL::text servicename,NULL::bigint observedat,a.projectionstalesince IS NOT NULL isstale,a.state containerstate,w.status ownerstatus,COALESCE(w.controlstate='Processing',false) ownerprocessing
    FROM actual a LEFT JOIN authorized w ON w.kind=a.kind AND w.id=a.ownerid
    -- Match container-map visibility, including unresolved ownership for administrators.
    WHERE $2 OR (a.ownerid IS NULL AND NOT a.hascitadelownershiplabels) OR w.id IS NOT NULL
    UNION ALL
    SELECT NULL,NULL,NULL,NULL,NULL,w.kind,w.id,w.name,w.refs,
        CASE WHEN sum(COALESCE(octet_length(w.compose),0)) OVER (ORDER BY w.kind,w.id)<=4194304 THEN w.compose END, NULL,NULL,NULL,false,NULL::text,w.status,COALESCE(w.controlstate='Processing',false)
    FROM authorized w
    UNION ALL
    SELECT (r.ordinality-1)::integer,NULL,NULL,NULL,NULL,w.kind,w.id,w.name,'[]'::jsonb,NULL,
        s.dockerserviceid,s.name,extract(epoch FROM s.observedat)::bigint,s.isstale,NULL::text,w.status,COALESCE(w.controlstate='Processing',false)
    FROM jsonb_array_elements($4::jsonb) WITH ORDINALITY r
    JOIN swarmserviceprojections s ON s.platformid=$3 AND s.networkids ? (r.value->>'id')
    LEFT JOIN authorized w ON (w.kind=20 AND w.id=s.swarmserviceid) OR (w.kind=2 AND w.id=s.stackid)
    WHERE r.value->>'scope'='swarm' AND (w.id IS NOT NULL OR (s.ownership='Unmanaged' AND s.swarmserviceid IS NULL AND s.stackid IS NULL))
)
SELECT * FROM candidates ORDER BY network_index NULLS LAST,kind,ownerid,containerid LIMIT 2001
