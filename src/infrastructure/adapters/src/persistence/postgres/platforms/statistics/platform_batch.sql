WITH incoming AS (
 SELECT r.*, item.ordinal FROM jsonb_array_elements($1::jsonb) WITH ORDINALITY item(value, ordinal)
 CROSS JOIN LATERAL jsonb_to_record(item.value) r(platform_id uuid, node_id text, connector text, address text, agent_id uuid, connected_at timestamptz, created bigint, memory_active double precision, cpu_usage double precision, rx_bytes double precision, tx_bytes double precision, metadata jsonb, container_io jsonb)
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
), open AS (
 SELECT v.* FROM valid v WHERE NOT EXISTS (
     SELECT 1 FROM platformhistorysources src WHERE src.platformid=v.platform_id
     AND src.nodeid=COALESCE(v.node_id,'') AND v.created<src.compacteduntil)
), sources AS (
 INSERT INTO platformhistorysources(platformid,nodeid)
 SELECT DISTINCT platform_id,COALESCE(node_id,'') FROM open
 WHERE node_id IS NULL OR node_id IS DISTINCT FROM descriptor->>'nodeID'
 ON CONFLICT(platformid,nodeid) DO NOTHING
), node_latest AS (
 SELECT DISTINCT ON(platform_id,node_id,created) * FROM open
 WHERE node_id IS NOT NULL AND node_id IS DISTINCT FROM descriptor->>'nodeID'
 ORDER BY platform_id,node_id,created,ordinal DESC
), node_written AS (
 INSERT INTO platformnodeiostats(platformid,nodeid,created,containerio)
 SELECT platform_id,node_id,created,container_io FROM node_latest
 ON CONFLICT(platformid,nodeid,created) DO UPDATE SET containerio=EXCLUDED.containerio
 WHERE platformnodeiostats.containerio IS DISTINCT FROM EXCLUDED.containerio
), latest AS (
 SELECT DISTINCT ON(platform_id,created) * FROM open WHERE node_id IS NULL ORDER BY platform_id,created,ordinal DESC
), written AS (
 INSERT INTO platformstats(id,platformid,memoryusage,cpuusage,rxbytes,txbytes,created,diskusedbytes,disktotalbytes,diskusage,alertpending,containerio)
 SELECT gen_random_uuid(),platform_id,
 CASE WHEN COALESCE((metadata->>'memTotal')::bigint,memtotal)>0 THEN memory_active/COALESCE((metadata->>'memTotal')::bigint,memtotal)*100.0 ELSE 0 END,
 CASE WHEN cpucount>0 THEN cpu_usage/cpucount ELSE cpu_usage END,rx_bytes,tx_bytes,created,
 (metadata->>'diskUsedBytes')::bigint,(metadata->>'diskTotalBytes')::bigint,(metadata->>'diskUsage')::double precision,true,container_io FROM latest
 ON CONFLICT(platformid,created) DO UPDATE SET memoryusage=EXCLUDED.memoryusage,cpuusage=EXCLUDED.cpuusage,rxbytes=EXCLUDED.rxbytes,txbytes=EXCLUDED.txbytes,diskusedbytes=EXCLUDED.diskusedbytes,disktotalbytes=EXCLUDED.disktotalbytes,diskusage=EXCLUDED.diskusage,alertpending=true,containerio=EXCLUDED.containerio
 WHERE (platformstats.memoryusage,platformstats.cpuusage,platformstats.rxbytes,platformstats.txbytes,platformstats.diskusedbytes,platformstats.disktotalbytes,platformstats.diskusage,platformstats.containerio)
 IS DISTINCT FROM (EXCLUDED.memoryusage,EXCLUDED.cpuusage,EXCLUDED.rxbytes,EXCLUDED.txbytes,EXCLUDED.diskusedbytes,EXCLUDED.disktotalbytes,EXCLUDED.diskusage,EXCLUDED.containerio)
)
SELECT count(*)::bigint FROM open WHERE node_id IS NULL OR node_id IS DISTINCT FROM descriptor->>'nodeID'
