WITH incoming AS (
 SELECT r.*, item.ordinal FROM jsonb_array_elements($1::jsonb) WITH ORDINALITY item(value, ordinal)
 CROSS JOIN LATERAL jsonb_to_record(item.value) r(platform_id uuid, node_id text, connector text, address text, agent_id uuid, connected_at timestamptz, "dockerContainerId" text, "memoryActive" double precision, "memoryCache" double precision, "cpuUsage" double precision, "memoryLimit" double precision, "rxBytes" double precision, "txBytes" double precision, created bigint)
), valid AS (
 SELECT i.*, p.memtotal, p.cpucount, p.platformdescriptor::jsonb descriptor, p.connectortype
 FROM incoming i JOIN platforms p ON p.id=i.platform_id
 WHERE (i.connector IN ('Local','Agent') AND p.connectortype=i.connector
        AND i.node_id IS NULL AND rtrim(p.address,'/')=rtrim(i.address,'/'))
 OR (i.connector='EdgeAgent' AND (i.node_id IS NOT NULL OR p.connectortype='EdgeAgent')
     AND EXISTS (SELECT 1 FROM edgeagentbindings b WHERE b.platformid=i.platform_id
        AND b.agentid=i.agent_id AND b.lastconnectedatutc=i.connected_at
        AND b.dockernodeid IS NOT DISTINCT FROM i.node_id AND b.resourcetype='Platform'
        AND b.revokedatutc IS NULL AND b.connectionstatus='Connected'))
), eligible AS (
 SELECT v.*, c.id container_id FROM valid v JOIN containers c ON c.platformid=v.platform_id
 AND c.dockernodeid IS NOT DISTINCT FROM v.node_id AND c.dockercontainerid=v."dockerContainerId"
), latest AS (
 SELECT DISTINCT ON(container_id,created) * FROM eligible ORDER BY container_id,created,ordinal DESC
), inserted AS (
 INSERT INTO containerstats(id,containerid,memoryactive,memorycache,cpuusage,memorylimit,rxbytes,txbytes,created)
 SELECT gen_random_uuid(),container_id,"memoryActive","memoryCache","cpuUsage","memoryLimit","rxBytes","txBytes",created FROM latest
 ON CONFLICT(containerid,created) DO UPDATE SET memoryactive=EXCLUDED.memoryactive,memorycache=EXCLUDED.memorycache,
 cpuusage=EXCLUDED.cpuusage,memorylimit=EXCLUDED.memorylimit,rxbytes=EXCLUDED.rxbytes,txbytes=EXCLUDED.txbytes
 WHERE (containerstats.memoryactive,containerstats.memorycache,containerstats.cpuusage,containerstats.memorylimit,containerstats.rxbytes,containerstats.txbytes)
 IS DISTINCT FROM (EXCLUDED.memoryactive,EXCLUDED.memorycache,EXCLUDED.cpuusage,EXCLUDED.memorylimit,EXCLUDED.rxbytes,EXCLUDED.txbytes)
), service_samples AS (
 SELECT v.*, t.dockertaskid, s.dockerserviceid, s.name servicename, s.swarmserviceid, s.stackid,
 CASE WHEN t.slot IS NOT NULL THEN 'slot:'||t.slot::text ELSE 'node:'||t.dockernodeid END taskkey
 FROM valid v JOIN swarmtaskprojections t ON t.platformid=v.platform_id AND t.dockercontainerid=v."dockerContainerId"
 AND t.dockernodeid=COALESCE(v.node_id,v.descriptor->>'nodeID')
 JOIN swarmserviceprojections s ON s.platformid=t.platformid AND s.dockerserviceid=t.dockerserviceid
 WHERE s.ownership<>'System'
), service_latest AS (
 SELECT DISTINCT ON(platform_id,dockertaskid,created) * FROM service_samples ORDER BY platform_id,dockertaskid,created,ordinal DESC
), inserted_services AS (
 INSERT INTO swarmservicestats(id,platformid,dockerserviceid,dockertaskid,servicename,swarmserviceid,stackid,taskkey,created,memoryactive,memorycache,cpuusage,memorylimit,rxbytes,txbytes)
 SELECT gen_random_uuid(),platform_id,dockerserviceid,dockertaskid,servicename,swarmserviceid,stackid,taskkey,created,"memoryActive","memoryCache","cpuUsage","memoryLimit","rxBytes","txBytes" FROM service_latest
 ON CONFLICT(platformid,dockertaskid,created) DO UPDATE SET memoryactive=EXCLUDED.memoryactive,memorycache=EXCLUDED.memorycache,cpuusage=EXCLUDED.cpuusage,memorylimit=EXCLUDED.memorylimit,rxbytes=EXCLUDED.rxbytes,txbytes=EXCLUDED.txbytes,swarmserviceid=EXCLUDED.swarmserviceid,stackid=EXCLUDED.stackid
 WHERE (swarmservicestats.memoryactive,swarmservicestats.memorycache,swarmservicestats.cpuusage,swarmservicestats.memorylimit,swarmservicestats.rxbytes,swarmservicestats.txbytes,swarmservicestats.swarmserviceid,swarmservicestats.stackid)
 IS DISTINCT FROM (EXCLUDED.memoryactive,EXCLUDED.memorycache,EXCLUDED.cpuusage,EXCLUDED.memorylimit,EXCLUDED.rxbytes,EXCLUDED.txbytes,EXCLUDED.swarmserviceid,EXCLUDED.stackid)
)
SELECT count(*)::bigint FROM (SELECT ordinal FROM eligible UNION SELECT ordinal FROM service_samples) accepted
