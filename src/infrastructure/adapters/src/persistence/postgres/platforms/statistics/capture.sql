-- Caller holds the platform scope lock before this statement starts.
, projected AS (
 SELECT v.platform_id,v.ordinal,v.include_services,v.created,
 v."memoryActive",v."memoryCache",v."memoryActiveCache",v."cpuUsage",v."memoryLimit",
 v."rxBytes",v."txBytes",v."blockReadBytes",v."blockWriteBytes",dense_rank() OVER (ORDER BY v.platform_id,c.id,t.dockertaskid) AS identity_id,
 c.id AS container_id, c.deploymentid, c.stackid AS container_stackid,
 c.isswarmtask,c.composeservicename,c.composereplicanumber,
 t.dockertaskid,s.dockerserviceid,s.name AS servicename,s.swarmserviceid,s.stackid,
 CASE WHEN t.slot IS NOT NULL THEN 'slot:'||t.slot::text ELSE 'node:'||t.dockernodeid END AS taskkey
 FROM valid v LEFT JOIN containers c ON c.platformid=v.platform_id
 AND c.dockernodeid IS NOT DISTINCT FROM v.node_id AND c.dockercontainerid=v."dockerContainerId"
 LEFT JOIN swarmtaskprojections t ON t.platformid=v.platform_id AND t.dockercontainerid=v."dockerContainerId"
 AND t.dockernodeid=COALESCE(v.node_id,v.manager_node_id)
 LEFT JOIN swarmserviceprojections s ON s.platformid=t.platformid AND s.dockerserviceid=t.dockerserviceid AND s.ownership<>'System'
 WHERE c.id IS NOT NULL OR s.dockerserviceid IS NOT NULL
), identities AS (
 -- Resolve repeated captures once. Last-occurrence order retains the final
 -- projection association when a batch contains both sides of a Task handover.
 SELECT DISTINCT ON(identity_id) identity_id,ordinal,platform_id,container_id,
 deploymentid,container_stackid,isswarmtask,composeservicename,composereplicanumber,
 dockertaskid,dockerserviceid,servicename,swarmserviceid,stackid,taskkey
 FROM projected ORDER BY identity_id,ordinal DESC
), sourced AS MATERIALIZED (
 SELECT i.*,COALESCE(src.id,statistics_source(i.platform_id,i.container_id,
     CASE WHEN i.dockerserviceid IS NOT NULL THEN i.dockertaskid END)) AS source_id
 FROM identities i LEFT JOIN statisticssources src
 ON src.platformid=i.platform_id AND src.containerid=i.container_id
 -- Task reassociation must observe earlier function writes in this batch.
 -- Keep that path in statistics_source, which validates both live projections.
 AND i.dockerserviceid IS NULL
 ORDER BY i.ordinal
), owner_keys AS (
 -- Ordinary ownership can change while the physical source remains the same.
 SELECT s.*,
 CASE WHEN s.dockerserviceid IS NULL AND NOT s.isswarmtask
     AND (s.deploymentid IS NOT NULL OR s.container_stackid IS NOT NULL)
     THEN s.source_id END AS workload_source_id,
 CASE WHEN s.deploymentid IS NOT NULL THEN '' ELSE COALESCE(NULLIF(s.composeservicename,''),s.container_id::text) END AS owner_service,
 CASE WHEN s.deploymentid IS NOT NULL THEN 'deployment' WHEN s.composereplicanumber IS NOT NULL THEN 'replica:'||s.composereplicanumber ELSE 'instance:'||s.container_id::text END AS owner_replica
 FROM sourced s
), owned_identities AS MATERIALIZED (
 SELECT s.identity_id,s.source_id,COALESCE(persisted.compacteduntil,0) AS compacteduntil,CASE
 WHEN s.dockerserviceid IS NOT NULL THEN statistics_owner(s.source_id,'Service',NULL,s.stackid,s.swarmserviceid,s.dockerserviceid,s.servicename,s.taskkey,s.dockertaskid)
 WHEN NOT s.isswarmtask AND (s.deploymentid IS NOT NULL OR s.container_stackid IS NOT NULL)
 THEN COALESCE(existing_owner.id, statistics_owner(s.source_id,'Workload',s.deploymentid,s.container_stackid,NULL,NULL,s.owner_service,s.owner_replica))
 END AS owner_id
 FROM owner_keys s LEFT JOIN statisticssources persisted ON persisted.id=s.source_id
 -- Match the complete Workload key, including NULLs; Service reassociation stays
 -- in statistics_owner, and owners created in this statement use its miss path.
 -- NULL lookup keys avoid probing Workload owners for unmanaged containers/Tasks.
 LEFT JOIN statisticsowners existing_owner ON existing_owner.sourceid=s.workload_source_id
 AND existing_owner.kind='Workload'
 AND existing_owner.deploymentid IS NOT DISTINCT FROM s.deploymentid
 AND existing_owner.stackid IS NOT DISTINCT FROM s.container_stackid
 AND existing_owner.servicestackid IS NULL AND existing_owner.swarmserviceid IS NULL
 AND existing_owner.dockerserviceid IS NULL AND existing_owner.dockertaskid IS NULL
 AND existing_owner.servicename=s.owner_service AND existing_owner.replicakey=s.owner_replica
 -- The caller already holds the Platform lock. Existing watermarks cannot move
 -- during this statement; sources created by statistics_source start at zero
 -- and are intentionally absent from the outer statement snapshot.
 WHERE EXISTS (SELECT 1 FROM projected p WHERE p.identity_id=s.identity_id
     AND p.created>=COALESCE(persisted.compacteduntil,0))
), owned AS (
 SELECT p.*,i.source_id,i.owner_id FROM projected p JOIN owned_identities i USING(identity_id)
 WHERE p.created>=i.compacteduntil
), latest AS (
 SELECT DISTINCT ON(source_id,created) * FROM owned ORDER BY source_id,created,ordinal DESC
), inserted AS (
 INSERT INTO statisticssamples(sourceid,ownerid,created,cpuusage,memoryactive,memoryactivecache,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes)
 SELECT source_id,owner_id,created,"cpuUsage","memoryActive","memoryActiveCache","memoryCache","memoryLimit","rxBytes","txBytes","blockReadBytes","blockWriteBytes" FROM latest
 ON CONFLICT(sourceid,created) DO UPDATE SET
 ownerid=COALESCE(statisticssamples.ownerid,EXCLUDED.ownerid),
 cpuusage=EXCLUDED.cpuusage,memoryactive=EXCLUDED.memoryactive,memoryactivecache=EXCLUDED.memoryactivecache,memorycache=EXCLUDED.memorycache,
 memorylimit=EXCLUDED.memorylimit,rxbytes=EXCLUDED.rxbytes,txbytes=EXCLUDED.txbytes,
 blockreadbytes=EXCLUDED.blockreadbytes,blockwritebytes=EXCLUDED.blockwritebytes
 WHERE (statisticssamples.cpuusage,statisticssamples.memoryactive,statisticssamples.memoryactivecache,statisticssamples.memorycache,statisticssamples.memorylimit,statisticssamples.rxbytes,statisticssamples.txbytes,statisticssamples.blockreadbytes,statisticssamples.blockwritebytes,statisticssamples.ownerid)
 IS DISTINCT FROM (EXCLUDED.cpuusage,EXCLUDED.memoryactive,EXCLUDED.memoryactivecache,EXCLUDED.memorycache,EXCLUDED.memorylimit,EXCLUDED.rxbytes,EXCLUDED.txbytes,EXCLUDED.blockreadbytes,EXCLUDED.blockwritebytes,COALESCE(statisticssamples.ownerid,EXCLUDED.ownerid))
)
SELECT count(DISTINCT ordinal)::bigint FROM owned WHERE container_id IS NOT NULL OR include_services
