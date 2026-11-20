-- Select coarse I/O only for complete aligned blocks and compatible chart buckets.
-- Fine summaries remain available at range boundaries and wherever raw data remains.
-- Keep observations bounded before sorting; only endpoints carry counter baselines
-- across chunks. All tiers and chunk boundaries share this query snapshot.
WITH sources AS MATERIALIZED (
    SELECT id,nodeid FROM platformhistorysources WHERE platformid=$1
), covered AS MATERIALIZED (
    SELECT r.sourceid,range_agg(int8range(block,block+900,'[)')) AS spans
    FROM (
        SELECT DISTINCT r.sourceid,r.created/900*900 AS block
        FROM platformiocoarserollups r
        WHERE r.sourceid=ANY(ARRAY(SELECT id FROM sources))
          AND $3>=900 AND $3%900=0
          AND r.created>=(($2+899)/900)*900 AND r.created<(($4+1)/900)*900
    ) r GROUP BY r.sourceid
), fine_ranges AS MATERIALIZED (
    SELECT src.id AS sourceid,src.nodeid,lower(span) AS lo,upper(span)-1 AS hi
    FROM sources src LEFT JOIN covered c ON c.sourceid=src.id
    CROSS JOIN LATERAL unnest(int8multirange(int8range($2,$4,'[]'))
        - COALESCE(c.spans,'{}'::int8multirange)) span
), history AS NOT MATERIALIZED (
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
), has_summaries AS MATERIALIZED (
    SELECT CASE WHEN $4-$2<=3600 THEN EXISTS(SELECT FROM covered) OR EXISTS(
        SELECT FROM fine_ranges f CROSS JOIN LATERAL (
            SELECT FROM platformiorollups r WHERE r.sourceid=f.sourceid
              AND r.created BETWEEN f.lo AND f.hi AND r.lastcreated<=$4 LIMIT 1
        ) hit
    ) ELSE FALSE END AS present
), captures AS NOT MATERIALIZED (
    SELECT created,''::text COLLATE "C" AS nodeid,containerio FROM platformstats
    WHERE platformid=$1 AND created>=$2 AND created<=$4
    UNION ALL
    SELECT created,nodeid,containerio FROM platformnodeiostats
    WHERE platformid=$1 AND created>=$2 AND created<=$4
), raw_counters AS (
    -- Short raw windows fit in one narrow sort without boundary bookkeeping.
    SELECT h.created, h.created/$3 AS bucket, h.nodeid, c."containerId" COLLATE "C" AS "containerId", c."rxBytes", c."txBytes", c."blockReadBytes", c."blockWriteBytes" FROM captures h
    CROSS JOIN LATERAL jsonb_to_recordset(h.containerio) AS c(
            "containerId" text, "rxBytes" double precision, "txBytes" double precision,
            "blockReadBytes" double precision, "blockWriteBytes" double precision)
    WHERE NOT (SELECT present FROM has_summaries) AND $4-$2<=3600
), raw_samples AS (
    SELECT *, LAG("rxBytes") OVER observations AS previous_rx,
        LAG("txBytes") OVER observations AS previous_tx,
        LAG("blockReadBytes") OVER observations AS previous_read,
        LAG("blockWriteBytes") OVER observations AS previous_write,
        LAG(created) OVER observations AS previous_created
    FROM raw_counters WINDOW observations AS (PARTITION BY nodeid,"containerId" ORDER BY bucket,created ROWS BETWEEN 1 PRECEDING AND CURRENT ROW)
), raw_deltas AS (
    -- Each physical container has its own baseline. Removing a container is not a reset.
    SELECT *,
        CASE WHEN previous_rx IS NULL THEN NULL WHEN "rxBytes">=previous_rx THEN "rxBytes"-previous_rx ELSE "rxBytes" END AS rxbytesdelta,
        CASE WHEN previous_tx IS NULL THEN NULL WHEN "txBytes">=previous_tx THEN "txBytes"-previous_tx ELSE "txBytes" END AS txbytesdelta,
        CASE WHEN previous_read IS NULL THEN NULL WHEN "blockReadBytes">=previous_read THEN "blockReadBytes"-previous_read ELSE "blockReadBytes" END AS blockreadbytesdelta,
        CASE WHEN previous_write IS NULL THEN NULL WHEN "blockWriteBytes">=previous_write THEN "blockWriteBytes"-previous_write ELSE "blockWriteBytes" END AS blockwritebytesdelta
    FROM raw_samples ORDER BY nodeid,"containerId",bucket
), raw_per_container AS (
    SELECT bucket,MIN(created) AS created,
        AVG("blockReadBytes") AS blockreadbytes, AVG("blockWriteBytes") AS blockwritebytes,
        SUM(rxbytesdelta) AS rxbytesdelta, SUM(txbytesdelta) AS txbytesdelta,
        SUM(blockreadbytesdelta) AS blockreadbytesdelta, SUM(blockwritebytesdelta) AS blockwritebytesdelta,
        SUM(rxbytesdelta)/NULLIF(SUM(created-previous_created) FILTER (WHERE rxbytesdelta IS NOT NULL),0) AS rxbytespersecond,
        SUM(txbytesdelta)/NULLIF(SUM(created-previous_created) FILTER (WHERE txbytesdelta IS NOT NULL),0) AS txbytespersecond,
        SUM(blockreadbytesdelta)/NULLIF(SUM(created-previous_created) FILTER (WHERE blockreadbytesdelta IS NOT NULL),0) AS blockreadbytespersecond,
        SUM(blockwritebytesdelta)/NULLIF(SUM(created-previous_created) FILTER (WHERE blockwritebytesdelta IS NOT NULL),0) AS blockwritebytespersecond
    FROM raw_deltas GROUP BY nodeid,"containerId",bucket
), chunk_ranges AS MATERIALIZED (
    SELECT GREATEST($2,b*span.width) AS since,LEAST($4,(b+1)*span.width-1) AS until,
        (EXISTS(SELECT FROM covered WHERE spans && int8multirange(int8range(b*span.width,(b+1)*span.width,'[)')))
         OR EXISTS(
            SELECT FROM fine_ranges f CROSS JOIN LATERAL (
                SELECT FROM platformiorollups r WHERE r.sourceid=f.sourceid
                  AND r.created BETWEEN GREATEST(f.lo,b*span.width) AND LEAST(f.hi,(b+1)*span.width-1)
                  AND r.lastcreated<=$4 LIMIT 1
            ) hit WHERE f.lo<(b+1)*span.width AND f.hi>=b*span.width
         )) AS summarized
    FROM (SELECT GREATEST(900/$3,1)*$3 AS width) span
    CROSS JOIN LATERAL generate_series($2/span.width,$4/span.width) b
    WHERE $4-$2>3600 OR (SELECT present FROM has_summaries)
), chunks AS MATERIALIZED (
    -- Raw history: align roughly fifteen-minute chunks to whole chart buckets.
    -- Finish every bucket except each container's first observed bucket in a chunk;
    -- that bucket needs the preceding chunk's last observation, even across gaps.
    SELECT reduced.* FROM chunk_ranges ranges CROSS JOIN LATERAL (
        WITH captures AS NOT MATERIALIZED (
            SELECT created,''::text COLLATE "C" AS nodeid,containerio FROM platformstats
            WHERE platformid=$1 AND created>=ranges.since AND created<=ranges.until
            UNION ALL
            SELECT created,nodeid,containerio FROM platformnodeiostats
            WHERE platformid=$1 AND created>=ranges.since AND created<=ranges.until
        ), raw_counters AS (
            SELECT h.created, h.created/$3 AS bucket, h.nodeid, c."containerId" COLLATE "C" AS "containerId", c."rxBytes", c."txBytes", c."blockReadBytes", c."blockWriteBytes" FROM captures h
            CROSS JOIN LATERAL jsonb_to_recordset(h.containerio) AS c(
                    "containerId" text, "rxBytes" double precision, "txBytes" double precision,
                    "blockReadBytes" double precision, "blockWriteBytes" double precision)
        ), raw_samples AS (
            SELECT *, LEAD(created) OVER observations AS next_created, LAG("rxBytes") OVER observations AS previous_rx,
                LAG("txBytes") OVER observations AS previous_tx,
                LAG("blockReadBytes") OVER observations AS previous_read,
                LAG("blockWriteBytes") OVER observations AS previous_write,
                LAG(created) OVER observations AS previous_created
            FROM raw_counters WINDOW observations AS (PARTITION BY nodeid,"containerId" ORDER BY bucket,created ROWS BETWEEN 1 PRECEDING AND CURRENT ROW)
        ), raw_deltas AS (
            -- Each physical container has its own baseline. Removing a container is not a reset.
            SELECT *,
                CASE WHEN previous_rx IS NULL THEN NULL WHEN "rxBytes">=previous_rx THEN "rxBytes"-previous_rx ELSE "rxBytes" END AS rxbytesdelta,
                CASE WHEN previous_tx IS NULL THEN NULL WHEN "txBytes">=previous_tx THEN "txBytes"-previous_tx ELSE "txBytes" END AS txbytesdelta,
                CASE WHEN previous_read IS NULL THEN NULL WHEN "blockReadBytes">=previous_read THEN "blockReadBytes"-previous_read ELSE "blockReadBytes" END AS blockreadbytesdelta,
                CASE WHEN previous_write IS NULL THEN NULL WHEN "blockWriteBytes">=previous_write THEN "blockWriteBytes"-previous_write ELSE "blockWriteBytes" END AS blockwritebytesdelta
            FROM raw_samples ORDER BY nodeid,"containerId",bucket
        ), per_container AS MATERIALIZED (
            SELECT MIN(created) AS created,
                bucket, MAX(created) AS lastcreated, BOOL_OR(previous_created IS NULL) AS needs_bridge, nodeid,
                "containerId" AS containerid,
                MIN("rxBytes") FILTER (WHERE previous_created IS NULL) AS first_rx,
                MIN("txBytes") FILTER (WHERE previous_created IS NULL) AS first_tx,
                MIN("blockReadBytes") FILTER (WHERE previous_created IS NULL) AS first_read,
                MIN("blockWriteBytes") FILTER (WHERE previous_created IS NULL) AS first_write,
                MIN("rxBytes") FILTER (WHERE next_created IS NULL) AS last_rx,
                MIN("txBytes") FILTER (WHERE next_created IS NULL) AS last_tx,
                MIN("blockReadBytes") FILTER (WHERE next_created IS NULL) AS last_read,
                MIN("blockWriteBytes") FILTER (WHERE next_created IS NULL) AS last_write,
                SUM(rxbytesdelta) AS internal_rx, SUM(txbytesdelta) AS internal_tx, SUM(blockreadbytesdelta) AS internal_read,
                SUM(blockwritebytesdelta) AS internal_write,
                SUM(created-previous_created) FILTER(WHERE rxbytesdelta IS NOT NULL)::bigint AS duration_rx,
                SUM(created-previous_created) FILTER(WHERE txbytesdelta IS NOT NULL)::bigint AS duration_tx,
                SUM(created-previous_created) FILTER(WHERE blockreadbytesdelta IS NOT NULL)::bigint AS duration_read,
                SUM(created-previous_created) FILTER(WHERE blockwritebytesdelta IS NOT NULL)::bigint AS duration_write,
                AVG("blockReadBytes") AS blockreadbytes,
                AVG("blockWriteBytes") AS blockwritebytes FROM raw_deltas GROUP BY nodeid,"containerId",bucket)
        SELECT FALSE AS needs_bridge,
            bucket, MIN(created) AS created, NULL::bigint AS lastcreated, NULL::text AS nodeid, NULL::text AS containerid,
            NULL::double precision AS first_rx, NULL::double precision AS first_tx, NULL::double precision AS first_read,
            NULL::double precision AS first_write, NULL::double precision AS last_rx, NULL::double precision AS last_tx,
            NULL::double precision AS last_read, NULL::double precision AS last_write, SUM(internal_rx) AS internal_rx,
            SUM(internal_tx) AS internal_tx, SUM(internal_read) AS internal_read, SUM(internal_write) AS internal_write,
            NULL::bigint AS duration_rx, NULL::bigint AS duration_tx, NULL::bigint AS duration_read,
            NULL::bigint AS duration_write, SUM(blockreadbytes) AS blockreadbytes, SUM(blockwritebytes) AS blockwritebytes,
            SUM(internal_rx/NULLIF(duration_rx,0)) AS rxbytespersecond,
            SUM(internal_tx/NULLIF(duration_tx,0)) AS txbytespersecond,
            SUM(internal_read/NULLIF(duration_read,0)) AS blockreadbytespersecond,
            SUM(internal_write/NULLIF(duration_write,0)) AS blockwritebytespersecond FROM per_container WHERE NOT needs_bridge GROUP BY bucket
        UNION ALL
        -- The first bucket carries the last observation of the entire chunk. Other
        -- buckets are already complete and must not enter the boundary window again.
        SELECT * FROM (SELECT needs_bridge,
                bucket, created,
                LAST_VALUE(lastcreated) OVER endings AS lastcreated,
                nodeid, containerid, first_rx, first_tx, first_read, first_write,
                LAST_VALUE(last_rx) OVER endings AS last_rx,
                LAST_VALUE(last_tx) OVER endings AS last_tx,
                LAST_VALUE(last_read) OVER endings AS last_read,
                LAST_VALUE(last_write) OVER endings AS last_write,
                internal_rx, internal_tx, internal_read, internal_write, duration_rx, duration_tx, duration_read, duration_write,
                blockreadbytes, blockwritebytes, NULL::double precision AS rxbytespersecond,
                NULL::double precision AS txbytespersecond, NULL::double precision AS blockreadbytespersecond,
                NULL::double precision AS blockwritebytespersecond FROM per_container WINDOW endings AS (PARTITION BY nodeid,containerid ORDER BY bucket ROWS BETWEEN CURRENT ROW AND UNBOUNDED FOLLOWING)) edges WHERE needs_bridge
    ) reduced WHERE NOT ranges.summarized
    UNION ALL
    -- Mixed history: compact each chart bucket before the cross-bucket window.
    -- Preserve summary endpoints, internal deltas, durations and gauge weights.
    SELECT TRUE AS needs_bridge,bucket,created,lastcreated,nodeid,containerid,
        first_rx,first_tx,first_read,first_write,
        last_rx,last_tx,last_read,last_write,
        internal_rx,internal_tx,internal_read,internal_write,
        duration_rx,duration_tx,duration_read,duration_write,
        blockreadbytes,blockwritebytes,NULL::double precision AS rxbytespersecond,NULL::double precision AS txbytespersecond,
        NULL::double precision AS blockreadbytespersecond,NULL::double precision AS blockwritebytespersecond
    FROM chunk_ranges ranges CROSS JOIN LATERAL generate_series(ranges.since/$3,ranges.until/$3) b CROSS JOIN LATERAL (
        WITH captures AS NOT MATERIALIZED (
            SELECT created,''::text COLLATE "C" AS nodeid,containerio FROM platformstats
            WHERE platformid=$1 AND created>=GREATEST($2,b*$3) AND created<=LEAST($4,(b+1)*$3-1)
            UNION ALL
            SELECT created,nodeid,containerio FROM platformnodeiostats
            WHERE platformid=$1 AND created>=GREATEST($2,b*$3) AND created<=LEAST($4,(b+1)*$3-1)
        ), summaries AS NOT MATERIALIZED (
            SELECT r.created,r.created/$3 AS bucket,r.lastcreated,r.nodeid,r.containerid,
                r.firstcounters[1],r.firstcounters[2],r.firstcounters[3],r.firstcounters[4],
                r.lastcounters[1],r.lastcounters[2],r.lastcounters[3],r.lastcounters[4],
                r.counterdeltas[1],r.counterdeltas[2],r.counterdeltas[3],r.counterdeltas[4],
                r.counterdurations[1],r.counterdurations[2],r.counterdurations[3],r.counterdurations[4],
                r.blockreadbytes,r.blockwritebytes,r.readcount,r.writecount
            FROM (
                SELECT f.nodeid,r.* FROM fine_ranges f CROSS JOIN LATERAL (
                    SELECT * FROM platformiorollups r WHERE r.sourceid=f.sourceid
                      AND r.created BETWEEN GREATEST(f.lo,b*$3) AND LEAST(f.hi,(b+1)*$3-1)
                      AND r.lastcreated<=$4 OFFSET 0
                ) r WHERE f.lo<(b+1)*$3 AND f.hi>=b*$3
                UNION ALL
                SELECT src.nodeid,r.* FROM sources src CROSS JOIN LATERAL (
                    SELECT * FROM platformiocoarserollups r WHERE r.sourceid=src.id
                      AND r.created>=GREATEST((($2+899)/900)*900,b*$3)
                      AND r.created<LEAST((($4+1)/900)*900,(b+1)*$3)
                      AND r.lastcreated<=$4 OFFSET 0
                ) r WHERE $3>=900 AND $3%900=0
            ) r
        ), mixed_raw AS NOT MATERIALIZED (
            SELECT h.created,h.nodeid,c."containerId" COLLATE "C" AS containerid,
                c."rxBytes" AS rx,c."txBytes" AS tx,c."blockReadBytes" AS read,c."blockWriteBytes" AS write
            FROM captures h CROSS JOIN LATERAL jsonb_to_recordset(h.containerio) AS c(
                    "containerId" text,"rxBytes" double precision,"txBytes" double precision,
                    "blockReadBytes" double precision,"blockWriteBytes" double precision)
        ), counters AS NOT MATERIALIZED (
            SELECT h.created,h.created/$3 AS bucket,NULL::bigint AS lastcreated,h.nodeid,h.containerid,
                h.rx AS first_rx, h.tx AS first_tx, h.read AS first_read, h.write AS first_write, NULL::double precision AS last_rx,
                NULL::double precision AS last_tx, NULL::double precision AS last_read, NULL::double precision AS last_write,
                NULL::double precision AS internal_rx, NULL::double precision AS internal_tx,
                NULL::double precision AS internal_read, NULL::double precision AS internal_write, NULL::bigint AS duration_rx,
                NULL::bigint AS duration_tx, NULL::bigint AS duration_read, NULL::bigint AS duration_write,
                NULL::double precision AS blockreadbytes,NULL::double precision AS blockwritebytes,
                NULL::integer AS readcount,NULL::integer AS writecount
            FROM mixed_raw h
            UNION ALL
            SELECT * FROM summaries
        ), samples AS (
            SELECT *,LEAD(created) OVER observations AS next_created,LAG(COALESCE(lastcreated,created)) OVER observations AS previous_created,
                LAG(CASE WHEN lastcreated IS NULL THEN first_rx ELSE last_rx END) OVER observations AS previous_rx,
                LAG(CASE WHEN lastcreated IS NULL THEN first_tx ELSE last_tx END) OVER observations AS previous_tx,
                LAG(CASE WHEN lastcreated IS NULL THEN first_read ELSE last_read END) OVER observations AS previous_read,
                LAG(CASE WHEN lastcreated IS NULL THEN first_write ELSE last_write END) OVER observations AS previous_write
            FROM counters WINDOW observations AS (PARTITION BY nodeid,containerid ORDER BY bucket,created ROWS BETWEEN 1 PRECEDING AND CURRENT ROW)
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
            FROM samples ORDER BY nodeid,containerid,bucket
        ) SELECT MIN(created) AS created,
            bucket, MAX(COALESCE(lastcreated,created)) AS lastcreated, nodeid, containerid,
            MIN(first_rx) FILTER (WHERE previous_created IS NULL) AS first_rx,
            MIN(first_tx) FILTER (WHERE previous_created IS NULL) AS first_tx,
            MIN(first_read) FILTER (WHERE previous_created IS NULL) AS first_read,
            MIN(first_write) FILTER (WHERE previous_created IS NULL) AS first_write,
            MIN(CASE WHEN lastcreated IS NULL THEN first_rx ELSE last_rx END) FILTER (WHERE next_created IS NULL) AS last_rx,
            MIN(CASE WHEN lastcreated IS NULL THEN first_tx ELSE last_tx END) FILTER (WHERE next_created IS NULL) AS last_tx,
            MIN(CASE WHEN lastcreated IS NULL THEN first_read ELSE last_read END) FILTER (WHERE next_created IS NULL) AS last_read,
            MIN(CASE WHEN lastcreated IS NULL THEN first_write ELSE last_write END) FILTER (WHERE next_created IS NULL) AS last_write,
            statistics_sum(SUM(internal_rx),SUM(rxbytesdelta)) AS internal_rx,
            statistics_sum(SUM(internal_tx),SUM(txbytesdelta)) AS internal_tx,
            statistics_sum(SUM(internal_read),SUM(blockreadbytesdelta)) AS internal_read,
            statistics_sum(SUM(internal_write),SUM(blockwritebytesdelta)) AS internal_write,
            SUM(rxbytesduration)::bigint AS duration_rx, SUM(txbytesduration)::bigint AS duration_tx,
            SUM(blockreadbytesduration)::bigint AS duration_read, SUM(blockwritebytesduration)::bigint AS duration_write,
            SUM((CASE WHEN lastcreated IS NULL THEN first_read ELSE blockreadbytes END)*COALESCE(readcount,(first_read IS NOT NULL)::integer))/NULLIF(SUM(COALESCE(readcount,(first_read IS NOT NULL)::integer)),0) AS blockreadbytes,
            SUM((CASE WHEN lastcreated IS NULL THEN first_write ELSE blockwritebytes END)*COALESCE(writecount,(first_write IS NOT NULL)::integer))/NULLIF(SUM(COALESCE(writecount,(first_write IS NOT NULL)::integer)),0) AS blockwritebytes,
            SUM(COALESCE(readcount,(first_read IS NOT NULL)::integer)) AS readcount,
            SUM(COALESCE(writecount,(first_write IS NOT NULL)::integer)) AS writecount
        FROM deltas GROUP BY nodeid,containerid,bucket
    ) reduced WHERE ranges.summarized
), samples AS (
    -- Exact historical node/container keys preserve identity without a dictionary.
    SELECT *,LAG(COALESCE(lastcreated,created)) OVER observations AS previous_created,
        LAG(CASE WHEN lastcreated IS NULL THEN first_rx ELSE last_rx END) OVER observations AS previous_rx,
        LAG(CASE WHEN lastcreated IS NULL THEN first_tx ELSE last_tx END) OVER observations AS previous_tx,
        LAG(CASE WHEN lastcreated IS NULL THEN first_read ELSE last_read END) OVER observations AS previous_read,
        LAG(CASE WHEN lastcreated IS NULL THEN first_write ELSE last_write END) OVER observations AS previous_write
    FROM chunks WHERE needs_bridge WINDOW observations AS (PARTITION BY nodeid,containerid ORDER BY bucket,created ROWS BETWEEN 1 PRECEDING AND CURRENT ROW)
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
    FROM samples ORDER BY nodeid,containerid,bucket
), boundary_io AS (
    SELECT bucket,
        MIN(created) AS created, SUM(blockreadbytes) AS blockreadbytes, SUM(blockwritebytes) AS blockwritebytes,
        SUM(statistics_sum(internal_rx,rxbytesdelta)) AS rxbytesdelta,
        SUM(statistics_sum(internal_tx,txbytesdelta)) AS txbytesdelta,
        SUM(statistics_sum(internal_read,blockreadbytesdelta)) AS blockreadbytesdelta,
        SUM(statistics_sum(internal_write,blockwritebytesdelta)) AS blockwritebytesdelta,
        SUM(statistics_sum(internal_rx,rxbytesdelta)/NULLIF(rxbytesduration,0)) AS rxbytespersecond,
        SUM(statistics_sum(internal_tx,txbytesdelta)/NULLIF(txbytesduration,0)) AS txbytespersecond,
        SUM(statistics_sum(internal_read,blockreadbytesdelta)/NULLIF(blockreadbytesduration,0)) AS blockreadbytespersecond,
        SUM(statistics_sum(internal_write,blockwritebytesdelta)/NULLIF(blockwritebytesduration,0)) AS blockwritebytespersecond FROM deltas GROUP BY bucket
), io AS (
    SELECT bucket,
        MIN(created) AS created, SUM(blockreadbytes) AS blockreadbytes, SUM(blockwritebytes) AS blockwritebytes,
        SUM(rxbytesdelta) AS rxbytesdelta, SUM(txbytesdelta) AS txbytesdelta,
        SUM(blockreadbytesdelta) AS blockreadbytesdelta, SUM(blockwritebytesdelta) AS blockwritebytesdelta,
        SUM(rxbytespersecond) AS rxbytespersecond, SUM(txbytespersecond) AS txbytespersecond,
        SUM(blockreadbytespersecond) AS blockreadbytespersecond,
        SUM(blockwritebytespersecond) AS blockwritebytespersecond FROM (SELECT * FROM boundary_io UNION ALL SELECT * FROM raw_per_container UNION ALL SELECT bucket,created,blockreadbytes,blockwritebytes,internal_rx,internal_tx,internal_read,internal_write,rxbytespersecond,txbytespersecond,blockreadbytespersecond,blockwritebytespersecond FROM chunks WHERE NOT needs_bridge) contributions GROUP BY bucket
)
SELECT COALESCE(gauges.created,io.created) AS created,
    gauges.cpuusage,gauges.memoryusage,gauges.rxbytes,gauges.txbytes,
    gauges.diskusedbytes,gauges.disktotalbytes,gauges.diskusage,io.blockreadbytes,io.blockwritebytes,io.rxbytesdelta,io.txbytesdelta,
    io.blockreadbytesdelta,io.blockwritebytesdelta,io.rxbytespersecond,io.txbytespersecond,
    io.blockreadbytespersecond,io.blockwritebytespersecond
FROM gauges FULL OUTER JOIN io USING(bucket) ORDER BY created
