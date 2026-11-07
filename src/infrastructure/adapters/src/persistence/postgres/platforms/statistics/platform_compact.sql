-- One host/node and at most one hour, serialized with captures by the Platform lock.
-- The caller stops before the first pending-alert minute and the newest minute.
WITH source AS (
 SELECT * FROM platformhistorysources WHERE id=$1
), captures AS MATERIALIZED (
 SELECT s.ctid AS tupleid,s.created,s.cpuusage,s.memoryusage,s.rxbytes,s.txbytes,
     s.diskusedbytes,s.disktotalbytes,s.diskusage,s.containerio
 FROM platformstats s JOIN source src ON src.platformid=s.platformid AND src.nodeid=''
 WHERE s.created>=$2 AND s.created<$3 AND NOT s.alertpending
 UNION ALL
 SELECT s.ctid,s.created,NULL,NULL,NULL,NULL,NULL,NULL,NULL,s.containerio
 FROM platformnodeiostats s JOIN source src ON src.platformid=s.platformid AND src.nodeid=s.nodeid AND src.nodeid<>''
 WHERE s.created>=$2 AND s.created<$3
), minutes AS (
 SELECT created/60 AS minute FROM captures GROUP BY created/60 HAVING count(*)>=4
), selected AS MATERIALIZED (
 SELECT c.* FROM captures c JOIN minutes m ON m.minute=c.created/60
), gauges AS (
 INSERT INTO platformgaugerollups(sourceid,created,lastcreated,samplecount,
     cpuusage,memoryusage,rxbytes,txbytes,diskusedbytes,disktotalbytes,diskusage,diskcounts)
 SELECT $1,min(created),max(created),count(*)::integer,
     avg(cpuusage),avg(memoryusage),avg(rxbytes),avg(txbytes),avg(diskusedbytes),avg(disktotalbytes),avg(diskusage),
     ARRAY[count(diskusedbytes),count(disktotalbytes),count(diskusage)]::integer[]
 FROM selected WHERE (SELECT nodeid='' FROM source) GROUP BY created/60
), counters AS (
 SELECT h.created,c.* FROM selected h
 CROSS JOIN LATERAL jsonb_to_recordset(h.containerio) AS c(
     "containerId" text COLLATE "C", "rxBytes" double precision, "txBytes" double precision,
     "blockReadBytes" double precision, "blockWriteBytes" double precision)
), samples AS (
 SELECT *,lag(created) OVER w AS previous_created,
     lag("rxBytes") OVER w AS previous_rx,
     lag("txBytes") OVER w AS previous_tx,
     lag("blockReadBytes") OVER w AS previous_read,
     lag("blockWriteBytes") OVER w AS previous_write
 FROM counters WINDOW w AS (PARTITION BY created/60,"containerId" ORDER BY created)
), deltas AS (
 SELECT *,
     statistics_delta("rxBytes",previous_rx) AS delta_rx,
     statistics_delta("txBytes",previous_tx) AS delta_tx,
     statistics_delta("blockReadBytes",previous_read) AS delta_read,
     statistics_delta("blockWriteBytes",previous_write) AS delta_write
 FROM samples
), io AS (
 INSERT INTO platformiorollups(sourceid,containerid,created,lastcreated,
     firstcounters,lastcounters,counterdeltas,counterdurations,blockreadbytes,blockwritebytes,readcount,writecount)
 SELECT $1,"containerId",min(created),max(created),
     ARRAY[(array_agg("rxBytes" ORDER BY created))[1],(array_agg("txBytes" ORDER BY created))[1],(array_agg("blockReadBytes" ORDER BY created))[1],(array_agg("blockWriteBytes" ORDER BY created))[1]],
     ARRAY[(array_agg("rxBytes" ORDER BY created DESC))[1],(array_agg("txBytes" ORDER BY created DESC))[1],(array_agg("blockReadBytes" ORDER BY created DESC))[1],(array_agg("blockWriteBytes" ORDER BY created DESC))[1]],
     ARRAY[sum(delta_rx),sum(delta_tx),sum(delta_read),sum(delta_write)],
     ARRAY[COALESCE(sum(created-previous_created) FILTER(WHERE delta_rx IS NOT NULL),0)::bigint,COALESCE(sum(created-previous_created) FILTER(WHERE delta_tx IS NOT NULL),0)::bigint,COALESCE(sum(created-previous_created) FILTER(WHERE delta_read IS NOT NULL),0)::bigint,COALESCE(sum(created-previous_created) FILTER(WHERE delta_write IS NOT NULL),0)::bigint],
     avg("blockReadBytes"),avg("blockWriteBytes"),count("blockReadBytes")::integer,count("blockWriteBytes")::integer
 FROM deltas GROUP BY created/60,"containerId"
), removed_host AS (
 -- Statement-local tuple IDs target one non-partitioned table per source. This
 -- avoids a DELETE join choosing a scan of the entire retained history.
 DELETE FROM platformstats s WHERE (SELECT nodeid='' FROM source)
 AND s.ctid=ANY(ARRAY(SELECT tupleid FROM selected)) AND NOT s.alertpending
 RETURNING s.created
), removed_node AS (
 DELETE FROM platformnodeiostats s WHERE (SELECT nodeid<>'' FROM source)
 AND s.ctid=ANY(ARRAY(SELECT tupleid FROM selected))
 RETURNING s.created
), sealed AS (
 UPDATE platformhistorysources SET compacteduntil=$3 WHERE id=$1
)
SELECT (SELECT count(*) FROM removed_host)+(SELECT count(*) FROM removed_node)
