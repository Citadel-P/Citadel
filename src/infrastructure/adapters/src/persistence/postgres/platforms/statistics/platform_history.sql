WITH history AS MATERIALIZED (
    SELECT * FROM platformstats WHERE platformid=$1 AND created>=$2 AND created<=$4
), gauges AS (
    SELECT created/$3 AS bucket, MIN(created) AS created, AVG(cpuusage) AS cpuusage,
        AVG(memoryusage) AS memoryusage, AVG(rxbytes) AS rxbytes, AVG(txbytes) AS txbytes,
        ROUND(AVG(diskusedbytes))::bigint AS diskusedbytes,
        ROUND(AVG(disktotalbytes))::bigint AS disktotalbytes, AVG(diskusage) AS diskusage
    FROM history GROUP BY created/$3
), counters AS (
    SELECT h.created, c.* FROM history h
    CROSS JOIN LATERAL jsonb_to_recordset(h.containerio) AS c(
        "containerId" text, "rxBytes" double precision, "txBytes" double precision,
        "blockReadBytes" double precision, "blockWriteBytes" double precision)
), samples AS (
    SELECT *, LAG("rxBytes") OVER observations AS previous_rx,
        LAG("txBytes") OVER observations AS previous_tx,
        LAG("blockReadBytes") OVER observations AS previous_read,
        LAG("blockWriteBytes") OVER observations AS previous_write,
        LAG(created) OVER observations AS previous_created
    FROM counters WINDOW observations AS (PARTITION BY "containerId" ORDER BY created)
), deltas AS (
    -- Each physical container has its own baseline. Removing a container is not a reset.
    SELECT *,
        CASE WHEN previous_rx IS NULL THEN NULL WHEN "rxBytes">=previous_rx THEN "rxBytes"-previous_rx ELSE "rxBytes" END AS rxbytesdelta,
        CASE WHEN previous_tx IS NULL THEN NULL WHEN "txBytes">=previous_tx THEN "txBytes"-previous_tx ELSE "txBytes" END AS txbytesdelta,
        CASE WHEN previous_read IS NULL THEN NULL WHEN "blockReadBytes">=previous_read THEN "blockReadBytes"-previous_read ELSE "blockReadBytes" END AS blockreadbytesdelta,
        CASE WHEN previous_write IS NULL THEN NULL WHEN "blockWriteBytes">=previous_write THEN "blockWriteBytes"-previous_write ELSE "blockWriteBytes" END AS blockwritebytesdelta
    FROM samples
), per_container AS (
    SELECT created/$3 AS bucket, "containerId",
        AVG("blockReadBytes") AS blockreadbytes, AVG("blockWriteBytes") AS blockwritebytes,
        SUM(rxbytesdelta) AS rxbytesdelta, SUM(txbytesdelta) AS txbytesdelta,
        SUM(blockreadbytesdelta) AS blockreadbytesdelta, SUM(blockwritebytesdelta) AS blockwritebytesdelta,
        SUM(rxbytesdelta)/NULLIF(SUM(created-previous_created) FILTER (WHERE rxbytesdelta IS NOT NULL),0) AS rxbytespersecond,
        SUM(txbytesdelta)/NULLIF(SUM(created-previous_created) FILTER (WHERE txbytesdelta IS NOT NULL),0) AS txbytespersecond,
        SUM(blockreadbytesdelta)/NULLIF(SUM(created-previous_created) FILTER (WHERE blockreadbytesdelta IS NOT NULL),0) AS blockreadbytespersecond,
        SUM(blockwritebytesdelta)/NULLIF(SUM(created-previous_created) FILTER (WHERE blockwritebytesdelta IS NOT NULL),0) AS blockwritebytespersecond
    FROM deltas GROUP BY created/$3,"containerId"
), io AS (
    SELECT bucket, SUM(blockreadbytes) AS blockreadbytes, SUM(blockwritebytes) AS blockwritebytes,
        SUM(rxbytesdelta) AS rxbytesdelta, SUM(txbytesdelta) AS txbytesdelta,
        SUM(blockreadbytesdelta) AS blockreadbytesdelta, SUM(blockwritebytesdelta) AS blockwritebytesdelta,
        SUM(rxbytespersecond) AS rxbytespersecond, SUM(txbytespersecond) AS txbytespersecond,
        SUM(blockreadbytespersecond) AS blockreadbytespersecond, SUM(blockwritebytespersecond) AS blockwritebytespersecond
    FROM per_container GROUP BY bucket
)
SELECT gauges.*, io.blockreadbytes, io.blockwritebytes, io.rxbytesdelta, io.txbytesdelta,
    io.blockreadbytesdelta, io.blockwritebytesdelta, io.rxbytespersecond, io.txbytespersecond,
    io.blockreadbytespersecond, io.blockwritebytespersecond
FROM gauges LEFT JOIN io USING(bucket) ORDER BY created
