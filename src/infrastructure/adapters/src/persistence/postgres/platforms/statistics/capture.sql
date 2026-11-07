-- Caller holds the platform scope lock before this statement starts.
, projected AS (
 SELECT v.*, c.id AS container_id, c.deploymentid, c.stackid AS container_stackid,
 c.isswarmtask,c.composeservicename,c.composereplicanumber,
 t.dockertaskid,s.dockerserviceid,s.name AS servicename,s.swarmserviceid,s.stackid,
 CASE WHEN t.slot IS NOT NULL THEN 'slot:'||t.slot::text ELSE 'node:'||t.dockernodeid END AS taskkey
 FROM valid v LEFT JOIN containers c ON c.platformid=v.platform_id
 AND c.dockernodeid IS NOT DISTINCT FROM v.node_id AND c.dockercontainerid=v."dockerContainerId"
 LEFT JOIN swarmtaskprojections t ON t.platformid=v.platform_id AND t.dockercontainerid=v."dockerContainerId"
 AND t.dockernodeid=COALESCE(v.node_id,v.descriptor->>'nodeID')
 LEFT JOIN swarmserviceprojections s ON s.platformid=t.platformid AND s.dockerserviceid=t.dockerserviceid AND s.ownership<>'System'
 WHERE c.id IS NOT NULL OR s.dockerserviceid IS NOT NULL
), sourced AS MATERIALIZED (
 SELECT p.*,statistics_source(platform_id,container_id,CASE WHEN dockerserviceid IS NOT NULL THEN dockertaskid END) AS source_id
 FROM projected p ORDER BY ordinal
), owned AS MATERIALIZED (
 SELECT s.*, CASE
 WHEN dockerserviceid IS NOT NULL THEN statistics_owner(source_id,'Service',NULL,stackid,swarmserviceid,dockerserviceid,servicename,taskkey,dockertaskid)
 WHEN NOT isswarmtask AND (deploymentid IS NOT NULL OR container_stackid IS NOT NULL)
 THEN statistics_owner(source_id,'Workload',deploymentid,container_stackid,NULL,NULL,
 CASE WHEN deploymentid IS NOT NULL THEN '' ELSE COALESCE(NULLIF(composeservicename,''),container_id::text) END,
 CASE WHEN deploymentid IS NOT NULL THEN 'deployment' WHEN composereplicanumber IS NOT NULL THEN 'replica:'||composereplicanumber ELSE 'instance:'||container_id::text END)
 END AS owner_id
 FROM sourced s
 -- Newly created sources are not visible to this statement's outer snapshot;
 -- the lookup function checks the watermark with its own fresh snapshot.
 WHERE statistics_capture_open(source_id,created)
), latest AS (
 SELECT DISTINCT ON(source_id,created) * FROM owned ORDER BY source_id,created,ordinal DESC
), inserted AS (
 INSERT INTO statisticssamples(sourceid,ownerid,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes)
 SELECT source_id,owner_id,created,"cpuUsage","memoryActive","memoryCache","memoryLimit","rxBytes","txBytes","blockReadBytes","blockWriteBytes" FROM latest
 ON CONFLICT(sourceid,created) DO UPDATE SET
 ownerid=COALESCE(statisticssamples.ownerid,EXCLUDED.ownerid),
 cpuusage=EXCLUDED.cpuusage,memoryactive=EXCLUDED.memoryactive,memorycache=EXCLUDED.memorycache,
 memorylimit=EXCLUDED.memorylimit,rxbytes=EXCLUDED.rxbytes,txbytes=EXCLUDED.txbytes,
 blockreadbytes=EXCLUDED.blockreadbytes,blockwritebytes=EXCLUDED.blockwritebytes
 WHERE (statisticssamples.cpuusage,statisticssamples.memoryactive,statisticssamples.memorycache,statisticssamples.memorylimit,statisticssamples.rxbytes,statisticssamples.txbytes,statisticssamples.blockreadbytes,statisticssamples.blockwritebytes,statisticssamples.ownerid)
 IS DISTINCT FROM (EXCLUDED.cpuusage,EXCLUDED.memoryactive,EXCLUDED.memorycache,EXCLUDED.memorylimit,EXCLUDED.rxbytes,EXCLUDED.txbytes,EXCLUDED.blockreadbytes,EXCLUDED.blockwritebytes,COALESCE(statisticssamples.ownerid,EXCLUDED.ownerid))
)
SELECT count(DISTINCT ordinal)::bigint FROM owned WHERE container_id IS NOT NULL OR include_services
