-- Let each consumer project its columns instead of spooling every JSON capture
-- to a shared temporary relation just to read the small host gauges again.
WITH history AS NOT MATERIALIZED (
    SELECT created, cpuusage, memoryusage, rxbytes, txbytes,
        diskusedbytes, disktotalbytes, diskusage, containerio
    FROM platformstats WHERE platformid=$1 AND created>=$2 AND created<=$4
), gauges AS (
    SELECT created/$3 AS bucket, MIN(created) AS created, AVG(cpuusage) AS cpuusage,
        AVG(memoryusage) AS memoryusage, AVG(rxbytes) AS rxbytes, AVG(txbytes) AS txbytes,
        ROUND(AVG(diskusedbytes))::bigint AS diskusedbytes,
        ROUND(AVG(disktotalbytes))::bigint AS disktotalbytes, AVG(diskusage) AS diskusage
    FROM history GROUP BY created/$3
), captures AS (
    SELECT created, ''::text COLLATE "C" AS nodeid, containerio FROM history
    UNION ALL
    SELECT created, nodeid, containerio FROM platformnodeiostats
    WHERE platformid=$1 AND created>=$2 AND created<=$4
), counters AS (
    -- Docker identities need byte ordering, not locale-aware text comparisons.
    SELECT h.created, h.nodeid, c."containerId" COLLATE "C" AS "containerId",
        c."rxBytes", c."txBytes", c."blockReadBytes", c."blockWriteBytes"
    FROM captures h
    CROSS JOIN LATERAL jsonb_to_recordset(h.containerio) AS c(
        "containerId" text, "rxBytes" double precision, "txBytes" double precision,
        "blockReadBytes" double precision, "blockWriteBytes" double precision)
), samples AS (
    SELECT *, LAG("rxBytes") OVER observations AS previous_rx,
        LAG("txBytes") OVER observations AS previous_tx,
        LAG("blockReadBytes") OVER observations AS previous_read,
        LAG("blockWriteBytes") OVER observations AS previous_write,
        LAG(created) OVER observations AS previous_created
    FROM counters WINDOW observations AS (PARTITION BY nodeid,"containerId" ORDER BY created)
), deltas AS (
    -- Each physical container has its own baseline. Removing a container is not a reset.
    SELECT *,
        CASE WHEN previous_rx IS NULL THEN NULL WHEN "rxBytes">=previous_rx THEN "rxBytes"-previous_rx ELSE "rxBytes" END AS rxbytesdelta,
        CASE WHEN previous_tx IS NULL THEN NULL WHEN "txBytes">=previous_tx THEN "txBytes"-previous_tx ELSE "txBytes" END AS txbytesdelta,
        CASE WHEN previous_read IS NULL THEN NULL WHEN "blockReadBytes">=previous_read THEN "blockReadBytes"-previous_read ELSE "blockReadBytes" END AS blockreadbytesdelta,
        CASE WHEN previous_write IS NULL THEN NULL WHEN "blockWriteBytes">=previous_write THEN "blockWriteBytes"-previous_write ELSE "blockWriteBytes" END AS blockwritebytesdelta
    FROM samples
), per_container AS (
    SELECT created/$3 AS bucket, nodeid, "containerId", MIN(created) AS created,
        AVG("blockReadBytes") AS blockreadbytes, AVG("blockWriteBytes") AS blockwritebytes,
        SUM(rxbytesdelta) AS rxbytesdelta, SUM(txbytesdelta) AS txbytesdelta,
        SUM(blockreadbytesdelta) AS blockreadbytesdelta, SUM(blockwritebytesdelta) AS blockwritebytesdelta,
        SUM(rxbytesdelta)/NULLIF(SUM(created-previous_created) FILTER (WHERE rxbytesdelta IS NOT NULL),0) AS rxbytespersecond,
        SUM(txbytesdelta)/NULLIF(SUM(created-previous_created) FILTER (WHERE txbytesdelta IS NOT NULL),0) AS txbytespersecond,
        SUM(blockreadbytesdelta)/NULLIF(SUM(created-previous_created) FILTER (WHERE blockreadbytesdelta IS NOT NULL),0) AS blockreadbytespersecond,
        SUM(blockwritebytesdelta)/NULLIF(SUM(created-previous_created) FILTER (WHERE blockwritebytesdelta IS NOT NULL),0) AS blockwritebytespersecond
    FROM deltas GROUP BY created/$3,nodeid,"containerId"
), io AS (
    SELECT bucket, MIN(created) AS created, SUM(blockreadbytes) AS blockreadbytes, SUM(blockwritebytes) AS blockwritebytes,
        SUM(rxbytesdelta) AS rxbytesdelta, SUM(txbytesdelta) AS txbytesdelta,
        SUM(blockreadbytesdelta) AS blockreadbytesdelta, SUM(blockwritebytesdelta) AS blockwritebytesdelta,
        SUM(rxbytespersecond) AS rxbytespersecond, SUM(txbytespersecond) AS txbytespersecond,
        SUM(blockreadbytespersecond) AS blockreadbytespersecond, SUM(blockwritebytespersecond) AS blockwritebytespersecond
    FROM per_container GROUP BY bucket
)
SELECT COALESCE(gauges.created,io.created) AS created,
    gauges.cpuusage, gauges.memoryusage, gauges.rxbytes, gauges.txbytes,
    gauges.diskusedbytes, gauges.disktotalbytes, gauges.diskusage, io.blockreadbytes, io.blockwritebytes, io.rxbytesdelta, io.txbytesdelta,
    io.blockreadbytesdelta, io.blockwritebytesdelta, io.rxbytespersecond, io.txbytespersecond,
    io.blockreadbytespersecond, io.blockwritebytespersecond
FROM gauges FULL OUTER JOIN io USING(bucket) ORDER BY created
