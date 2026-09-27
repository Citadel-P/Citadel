WITH incoming AS (
 SELECT r.*, item.ordinal FROM jsonb_array_elements($1::jsonb) WITH ORDINALITY item(value, ordinal)
 CROSS JOIN LATERAL jsonb_to_record(item.value) r(platform_id uuid, node_id text, connector text, address text, agent_id uuid, connected_at timestamptz, created bigint, memory_active double precision, cpu_usage double precision, rx_bytes double precision, tx_bytes double precision, metadata jsonb)
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
), latest AS (
 SELECT DISTINCT ON(platform_id) * FROM valid WHERE node_id IS NULL AND metadata IS NOT NULL
 ORDER BY platform_id,created DESC,ordinal DESC
), summaries AS (
 SELECT *,jsonb_build_object('containerCount',metadata->'containerCount','containersRunning',metadata->'containersRunning',
 'containersPaused',metadata->'containersPaused','containersStopped',metadata->'containersStopped',
 'imageUsedBytes',metadata->'imageUsedBytes','volumeUsedBytes',metadata->'volumeUsedBytes') summary FROM latest
)
UPDATE platforms p SET networkcount=(s.metadata->>'networkCount')::integer,
 volumecount=(s.metadata->>'volumeCount')::integer,imagecount=LEAST((s.metadata->>'imageCount')::bigint,2147483647)::integer,
 memtotal=(s.metadata->>'memTotal')::bigint,platformdescriptor=(p.platformdescriptor::jsonb||s.summary)::json
FROM summaries s WHERE p.id=s.platform_id
 AND NOT EXISTS(SELECT 1 FROM platformstats newer WHERE newer.platformid=p.id AND newer.created>s.created)
 AND (p.networkcount IS DISTINCT FROM (s.metadata->>'networkCount')::integer
 OR p.volumecount IS DISTINCT FROM (s.metadata->>'volumeCount')::integer
 OR p.imagecount IS DISTINCT FROM LEAST((s.metadata->>'imageCount')::bigint,2147483647)::integer
 OR p.memtotal IS DISTINCT FROM (s.metadata->>'memTotal')::bigint
 OR NOT p.platformdescriptor::jsonb @> s.summary)
