WITH valid AS (
 SELECT r.*,item.ordinal,false AS include_services,$1::uuid AS platform_id,$3::text AS node_id,p.platformdescriptor->>'nodeID' AS manager_node_id
 FROM jsonb_array_elements($2::jsonb) WITH ORDINALITY item(value,ordinal)
 CROSS JOIN LATERAL jsonb_to_record(item.value) r("dockerContainerId" text,"memoryActive" double precision,"memoryCache" double precision, "memoryActiveCache" double precision,"cpuUsage" double precision,"memoryLimit" double precision,"rxBytes" double precision,"txBytes" double precision,"blockReadBytes" double precision,"blockWriteBytes" double precision,created bigint)
 JOIN platforms p ON p.id=$1
)
