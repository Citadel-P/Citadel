WITH incoming AS (
 SELECT r.*, item.ordinal FROM jsonb_array_elements($1::jsonb) WITH ORDINALITY item(value, ordinal)
 CROSS JOIN LATERAL jsonb_to_record(item.value) r(platform_id uuid, node_id text, connector text, address text, agent_id uuid, connected_at timestamptz, "dockerContainerId" text, "memoryActive" double precision, "memoryCache" double precision, "memoryActiveCache" double precision, "cpuUsage" double precision, "memoryLimit" double precision, "rxBytes" double precision, "txBytes" double precision, "blockReadBytes" double precision, "blockWriteBytes" double precision, created bigint)
), valid AS (
 SELECT i.platform_id,i.node_id,i.ordinal,i."dockerContainerId",i.created,
 i."memoryActive",i."memoryCache",i."memoryActiveCache",i."cpuUsage",i."memoryLimit",
 i."rxBytes",i."txBytes",i."blockReadBytes",i."blockWriteBytes",
 true AS include_services, p.platformdescriptor->>'nodeID' AS manager_node_id
 FROM incoming i JOIN platforms p ON p.id=i.platform_id
 WHERE (i.connector IN ('Local','Agent') AND p.connectortype=i.connector
        AND i.node_id IS NULL AND rtrim(p.address,'/')=rtrim(i.address,'/'))
 OR (i.connector='EdgeAgent' AND (i.node_id IS NOT NULL OR p.connectortype='EdgeAgent')
     AND EXISTS (SELECT 1 FROM edgeagentbindings b WHERE b.platformid=i.platform_id
        AND b.agentid=i.agent_id AND b.lastconnectedatutc=i.connected_at
        AND b.dockernodeid IS NOT DISTINCT FROM i.node_id AND b.resourcetype='Platform'
        AND b.revokedatutc IS NULL AND b.connectionstatus='Connected'))
)
