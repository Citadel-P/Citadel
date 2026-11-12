-- Use the narrow raw counter path when this snapshot has no I/O summaries.
-- Mixed windows retain summary endpoints, weights and durations, with summary-
-- only fields NULL on raw observations to keep their sort tuples compact.
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
), captures AS NOT MATERIALIZED (
    SELECT created,''::text COLLATE "C" AS nodeid,containerio FROM platformstats
    WHERE platformid=$1 AND created>=$2 AND created<=$4
    UNION ALL
    SELECT created,nodeid,containerio FROM platformnodeiostats
    WHERE platformid=$1 AND created>=$2 AND created<=$4
), summaries AS NOT MATERIALIZED (
    SELECT r.created,r.lastcreated,src.nodeid,r.containerid,
        r.firstcounters[1],r.firstcounters[2],r.firstcounters[3],r.firstcounters[4],
        r.lastcounters[1],r.lastcounters[2],r.lastcounters[3],r.lastcounters[4],
        r.counterdeltas[1],r.counterdeltas[2],r.counterdeltas[3],r.counterdeltas[4],
        r.counterdurations[1],r.counterdurations[2],r.counterdurations[3],r.counterdurations[4],
        r.blockreadbytes,r.blockwritebytes,r.readcount,r.writecount
    FROM platformiorollups r JOIN platformhistorysources src ON src.id=r.sourceid
    WHERE r.sourceid=ANY(ARRAY(SELECT id FROM platformhistorysources WHERE platformid=$1))
        AND src.platformid=$1 AND r.created>=$2 AND r.created<=$4 AND r.lastcreated<=$4
), has_summaries AS MATERIALIZED (
    SELECT EXISTS(SELECT 1 FROM summaries) AS present
), raw_counters AS (
    SELECT h.created, h.nodeid, c."containerId" COLLATE "C" AS "containerId", c."rxBytes", c."txBytes", c."blockReadBytes", c."blockWriteBytes" FROM captures h
    CROSS JOIN LATERAL jsonb_to_recordset(h.containerio) AS c(
        "containerId" text, "rxBytes" double precision, "txBytes" double precision,
        "blockReadBytes" double precision, "blockWriteBytes" double precision)
    WHERE NOT (SELECT present FROM has_summaries)
), raw_samples AS (
    SELECT *, LAG("rxBytes") OVER observations AS previous_rx,
        LAG("txBytes") OVER observations AS previous_tx,
        LAG("blockReadBytes") OVER observations AS previous_read,
        LAG("blockWriteBytes") OVER observations AS previous_write,
        LAG(created) OVER observations AS previous_created
    FROM raw_counters WINDOW observations AS (PARTITION BY nodeid,"containerId" ORDER BY created)
), raw_deltas AS (
    -- Each physical container has its own baseline. Removing a container is not a reset.
    SELECT *,
        CASE WHEN previous_rx IS NULL THEN NULL WHEN "rxBytes">=previous_rx THEN "rxBytes"-previous_rx ELSE "rxBytes" END AS rxbytesdelta,
        CASE WHEN previous_tx IS NULL THEN NULL WHEN "txBytes">=previous_tx THEN "txBytes"-previous_tx ELSE "txBytes" END AS txbytesdelta,
        CASE WHEN previous_read IS NULL THEN NULL WHEN "blockReadBytes">=previous_read THEN "blockReadBytes"-previous_read ELSE "blockReadBytes" END AS blockreadbytesdelta,
        CASE WHEN previous_write IS NULL THEN NULL WHEN "blockWriteBytes">=previous_write THEN "blockWriteBytes"-previous_write ELSE "blockWriteBytes" END AS blockwritebytesdelta
    FROM raw_samples
), raw_per_container AS (
    SELECT created/$3 AS bucket, nodeid, "containerId", MIN(created) AS created,
        AVG("blockReadBytes") AS blockreadbytes, AVG("blockWriteBytes") AS blockwritebytes,
        SUM(rxbytesdelta) AS rxbytesdelta, SUM(txbytesdelta) AS txbytesdelta,
        SUM(blockreadbytesdelta) AS blockreadbytesdelta, SUM(blockwritebytesdelta) AS blockwritebytesdelta,
        SUM(rxbytesdelta)/NULLIF(SUM(created-previous_created) FILTER (WHERE rxbytesdelta IS NOT NULL),0) AS rxbytespersecond,
        SUM(txbytesdelta)/NULLIF(SUM(created-previous_created) FILTER (WHERE txbytesdelta IS NOT NULL),0) AS txbytespersecond,
        SUM(blockreadbytesdelta)/NULLIF(SUM(created-previous_created) FILTER (WHERE blockreadbytesdelta IS NOT NULL),0) AS blockreadbytespersecond,
        SUM(blockwritebytesdelta)/NULLIF(SUM(created-previous_created) FILTER (WHERE blockwritebytesdelta IS NOT NULL),0) AS blockwritebytespersecond
    FROM raw_deltas GROUP BY created/$3,nodeid,"containerId"
), counters AS (
    SELECT h.created,NULL::bigint AS lastcreated,h.nodeid,c."containerId" COLLATE "C" AS containerid,
        c."rxBytes" AS first_rx,
        c."txBytes" AS first_tx,
        c."blockReadBytes" AS first_read,
        c."blockWriteBytes" AS first_write,
        NULL::double precision AS last_rx,
        NULL::double precision AS last_tx,
        NULL::double precision AS last_read,
        NULL::double precision AS last_write,
        NULL::double precision AS internal_rx,
        NULL::double precision AS internal_tx,
        NULL::double precision AS internal_read,
        NULL::double precision AS internal_write,
        NULL::bigint AS duration_rx,
        NULL::bigint AS duration_tx,
        NULL::bigint AS duration_read,
        NULL::bigint AS duration_write,
        NULL::double precision AS blockreadbytes,NULL::double precision AS blockwritebytes,
        NULL::integer AS readcount,NULL::integer AS writecount
    FROM captures h CROSS JOIN LATERAL jsonb_to_recordset(h.containerio) AS c(
        "containerId" text,"rxBytes" double precision,"txBytes" double precision,
        "blockReadBytes" double precision,"blockWriteBytes" double precision)
    WHERE (SELECT present FROM has_summaries)
    UNION ALL
    SELECT * FROM summaries
), samples AS (
    SELECT *,LAG(COALESCE(lastcreated,created)) OVER observations AS previous_created,
        LAG(CASE WHEN lastcreated IS NULL THEN first_rx ELSE last_rx END) OVER observations AS previous_rx,
        LAG(CASE WHEN lastcreated IS NULL THEN first_tx ELSE last_tx END) OVER observations AS previous_tx,
        LAG(CASE WHEN lastcreated IS NULL THEN first_read ELSE last_read END) OVER observations AS previous_read,
        LAG(CASE WHEN lastcreated IS NULL THEN first_write ELSE last_write END) OVER observations AS previous_write
    FROM counters WINDOW observations AS (PARTITION BY nodeid,containerid ORDER BY created)
), deltas AS (
    -- Bridge adjacent physical-container blocks before summing across nodes.
    -- Internal summary deltas are added once per bucket below, not per raw row.
    SELECT *,
        statistics_delta(first_rx,previous_rx) AS rxbytesdelta,
        COALESCE(duration_rx,0)+CASE WHEN first_rx IS NOT NULL AND previous_rx IS NOT NULL THEN created-previous_created ELSE 0 END AS rxbytesduration,
        statistics_delta(first_tx,previous_tx) AS txbytesdelta,
        COALESCE(duration_tx,0)+CASE WHEN first_tx IS NOT NULL AND previous_tx IS NOT NULL THEN created-previous_created ELSE 0 END AS txbytesduration,
        statistics_delta(first_read,previous_read) AS blockreadbytesdelta,
        COALESCE(duration_read,0)+CASE WHEN first_read IS NOT NULL AND previous_read IS NOT NULL THEN created-previous_created ELSE 0 END AS blockreadbytesduration,
        statistics_delta(first_write,previous_write) AS blockwritebytesdelta,
        COALESCE(duration_write,0)+CASE WHEN first_write IS NOT NULL AND previous_write IS NOT NULL THEN created-previous_created ELSE 0 END AS blockwritebytesduration
    FROM samples
), mixed_per_container AS (
    SELECT created/$3 AS bucket,nodeid,containerid,MIN(created) AS created,
        SUM((CASE WHEN lastcreated IS NULL THEN first_read ELSE blockreadbytes END)*COALESCE(readcount,(first_read IS NOT NULL)::integer))/NULLIF(SUM(COALESCE(readcount,(first_read IS NOT NULL)::integer)),0) AS blockreadbytes,
        SUM((CASE WHEN lastcreated IS NULL THEN first_write ELSE blockwritebytes END)*COALESCE(writecount,(first_write IS NOT NULL)::integer))/NULLIF(SUM(COALESCE(writecount,(first_write IS NOT NULL)::integer)),0) AS blockwritebytes,
        statistics_sum(SUM(internal_rx),SUM(rxbytesdelta)) AS rxbytesdelta,statistics_sum(SUM(internal_tx),SUM(txbytesdelta)) AS txbytesdelta,
        statistics_sum(SUM(internal_read),SUM(blockreadbytesdelta)) AS blockreadbytesdelta,statistics_sum(SUM(internal_write),SUM(blockwritebytesdelta)) AS blockwritebytesdelta,
        statistics_sum(SUM(internal_rx),SUM(rxbytesdelta))/NULLIF(SUM(rxbytesduration),0) AS rxbytespersecond,
        statistics_sum(SUM(internal_tx),SUM(txbytesdelta))/NULLIF(SUM(txbytesduration),0) AS txbytespersecond,
        statistics_sum(SUM(internal_read),SUM(blockreadbytesdelta))/NULLIF(SUM(blockreadbytesduration),0) AS blockreadbytespersecond,
        statistics_sum(SUM(internal_write),SUM(blockwritebytesdelta))/NULLIF(SUM(blockwritebytesduration),0) AS blockwritebytespersecond
    FROM deltas GROUP BY created/$3,nodeid,containerid
), per_container AS (
    SELECT * FROM raw_per_container
    UNION ALL
    SELECT * FROM mixed_per_container
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
