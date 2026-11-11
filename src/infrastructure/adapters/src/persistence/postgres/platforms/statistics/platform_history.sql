-- Expand stored arrays only on summaries. Raw counters stay scalar through the
-- window sort, avoiding array allocation for every recent physical observation.
WITH history AS NOT MATERIALIZED (
    SELECT created,cpuusage,memoryusage,rxbytes,txbytes,diskusedbytes,disktotalbytes,diskusage,
        1 AS samplecount,ARRAY[(diskusedbytes IS NOT NULL)::integer,(disktotalbytes IS NOT NULL)::integer,(diskusage IS NOT NULL)::integer] AS diskcounts
    FROM platformstats WHERE platformid=$1 AND created>=$2 AND created<=$4
    UNION ALL
    SELECT r.created,r.cpuusage,r.memoryusage,r.rxbytes,r.txbytes,r.diskusedbytes,r.disktotalbytes,r.diskusage,r.samplecount,r.diskcounts
    FROM platformgaugerollups r JOIN platformhistorysources src ON src.id=r.sourceid
    WHERE src.platformid=$1 AND src.nodeid='' AND r.created>=$2 AND r.created<=$4 AND r.lastcreated<=$4
), gauges AS (
    SELECT created/$3 AS bucket,MIN(created) AS created,
        SUM(cpuusage*samplecount)/SUM(samplecount) AS cpuusage,
        SUM(memoryusage*samplecount)/SUM(samplecount) AS memoryusage,
        SUM(rxbytes*samplecount)/SUM(samplecount) AS rxbytes,
        SUM(txbytes*samplecount)/SUM(samplecount) AS txbytes,
        ROUND(SUM(diskusedbytes*diskcounts[1])/NULLIF(SUM(diskcounts[1]),0))::bigint AS diskusedbytes,
        ROUND(SUM(disktotalbytes*diskcounts[2])/NULLIF(SUM(diskcounts[2]),0))::bigint AS disktotalbytes,
        SUM(diskusage*diskcounts[3])/NULLIF(SUM(diskcounts[3]),0) AS diskusage
    FROM history GROUP BY created/$3
), captures AS (
    SELECT created,''::text COLLATE "C" AS nodeid,containerio FROM platformstats
    WHERE platformid=$1 AND created>=$2 AND created<=$4
    UNION ALL
    SELECT created,nodeid,containerio FROM platformnodeiostats
    WHERE platformid=$1 AND created>=$2 AND created<=$4
), counters AS (
    SELECT h.created,h.created AS lastcreated,h.nodeid,c."containerId" COLLATE "C" AS containerid,
        c."rxBytes" AS first_rx,
        c."txBytes" AS first_tx,
        c."blockReadBytes" AS first_read,
        c."blockWriteBytes" AS first_write,
        c."rxBytes" AS last_rx,
        c."txBytes" AS last_tx,
        c."blockReadBytes" AS last_read,
        c."blockWriteBytes" AS last_write,
        NULL::double precision AS internal_rx,
        NULL::double precision AS internal_tx,
        NULL::double precision AS internal_read,
        NULL::double precision AS internal_write,
        0::bigint AS duration_rx,
        0::bigint AS duration_tx,
        0::bigint AS duration_read,
        0::bigint AS duration_write,
        c."blockReadBytes" AS blockreadbytes,c."blockWriteBytes" AS blockwritebytes,
        (c."blockReadBytes" IS NOT NULL)::integer AS readcount,(c."blockWriteBytes" IS NOT NULL)::integer AS writecount
    FROM captures h CROSS JOIN LATERAL jsonb_to_recordset(h.containerio) AS c(
        "containerId" text,"rxBytes" double precision,"txBytes" double precision,
        "blockReadBytes" double precision,"blockWriteBytes" double precision)
    UNION ALL
    SELECT r.created,r.lastcreated,src.nodeid,r.containerid,
        r.firstcounters[1],r.firstcounters[2],r.firstcounters[3],r.firstcounters[4],
        r.lastcounters[1],r.lastcounters[2],r.lastcounters[3],r.lastcounters[4],
        r.counterdeltas[1],r.counterdeltas[2],r.counterdeltas[3],r.counterdeltas[4],
        r.counterdurations[1],r.counterdurations[2],r.counterdurations[3],r.counterdurations[4],
        r.blockreadbytes,r.blockwritebytes,r.readcount,r.writecount
    FROM platformiorollups r JOIN platformhistorysources src ON src.id=r.sourceid
    WHERE src.platformid=$1 AND r.created>=$2 AND r.created<=$4 AND r.lastcreated<=$4
), samples AS (
    SELECT *,LAG(lastcreated) OVER observations AS previous_created,
        LAG(last_rx) OVER observations AS previous_rx,
        LAG(last_tx) OVER observations AS previous_tx,
        LAG(last_read) OVER observations AS previous_read,
        LAG(last_write) OVER observations AS previous_write
    FROM counters WINDOW observations AS (PARTITION BY nodeid,containerid ORDER BY created)
), deltas AS (
    -- Bridge adjacent physical-container blocks before summing across nodes.
    -- Internal summary deltas are added once per bucket below, not per raw row.
    SELECT *,
        statistics_delta(first_rx,previous_rx) AS rxbytesdelta,
        duration_rx+CASE WHEN first_rx IS NOT NULL AND previous_rx IS NOT NULL THEN created-previous_created ELSE 0 END AS rxbytesduration,
        statistics_delta(first_tx,previous_tx) AS txbytesdelta,
        duration_tx+CASE WHEN first_tx IS NOT NULL AND previous_tx IS NOT NULL THEN created-previous_created ELSE 0 END AS txbytesduration,
        statistics_delta(first_read,previous_read) AS blockreadbytesdelta,
        duration_read+CASE WHEN first_read IS NOT NULL AND previous_read IS NOT NULL THEN created-previous_created ELSE 0 END AS blockreadbytesduration,
        statistics_delta(first_write,previous_write) AS blockwritebytesdelta,
        duration_write+CASE WHEN first_write IS NOT NULL AND previous_write IS NOT NULL THEN created-previous_created ELSE 0 END AS blockwritebytesduration
    FROM samples
), per_container AS (
    SELECT created/$3 AS bucket,nodeid,containerid,MIN(created) AS created,
        SUM(blockreadbytes*readcount)/NULLIF(SUM(readcount),0) AS blockreadbytes,
        SUM(blockwritebytes*writecount)/NULLIF(SUM(writecount),0) AS blockwritebytes,
        statistics_sum(SUM(internal_rx),SUM(rxbytesdelta)) AS rxbytesdelta,statistics_sum(SUM(internal_tx),SUM(txbytesdelta)) AS txbytesdelta,
        statistics_sum(SUM(internal_read),SUM(blockreadbytesdelta)) AS blockreadbytesdelta,statistics_sum(SUM(internal_write),SUM(blockwritebytesdelta)) AS blockwritebytesdelta,
        statistics_sum(SUM(internal_rx),SUM(rxbytesdelta))/NULLIF(SUM(rxbytesduration),0) AS rxbytespersecond,
        statistics_sum(SUM(internal_tx),SUM(txbytesdelta))/NULLIF(SUM(txbytesduration),0) AS txbytespersecond,
        statistics_sum(SUM(internal_read),SUM(blockreadbytesdelta))/NULLIF(SUM(blockreadbytesduration),0) AS blockreadbytespersecond,
        statistics_sum(SUM(internal_write),SUM(blockwritebytesdelta))/NULLIF(SUM(blockwritebytesduration),0) AS blockwritebytespersecond
    FROM deltas GROUP BY created/$3,nodeid,containerid
), io AS (
    SELECT bucket,MIN(created) AS created,SUM(blockreadbytes) AS blockreadbytes,SUM(blockwritebytes) AS blockwritebytes,
        SUM(rxbytesdelta) AS rxbytesdelta,SUM(txbytesdelta) AS txbytesdelta,
        SUM(blockreadbytesdelta) AS blockreadbytesdelta,SUM(blockwritebytesdelta) AS blockwritebytesdelta,
        SUM(rxbytespersecond) AS rxbytespersecond,SUM(txbytespersecond) AS txbytespersecond,
        SUM(blockreadbytespersecond) AS blockreadbytespersecond,SUM(blockwritebytespersecond) AS blockwritebytespersecond
    FROM per_container GROUP BY bucket
)
SELECT COALESCE(gauges.created,io.created) AS created,
    gauges.cpuusage,gauges.memoryusage,gauges.rxbytes,gauges.txbytes,
    gauges.diskusedbytes,gauges.disktotalbytes,gauges.diskusage,io.blockreadbytes,io.blockwritebytes,io.rxbytesdelta,io.txbytesdelta,
    io.blockreadbytesdelta,io.blockwritebytesdelta,io.rxbytespersecond,io.txbytespersecond,
    io.blockreadbytespersecond,io.blockwritebytespersecond
FROM gauges FULL OUTER JOIN io USING(bucket) ORDER BY created
