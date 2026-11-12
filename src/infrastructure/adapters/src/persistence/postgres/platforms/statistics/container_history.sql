-- Resolve live container projections once and keep source bounds on the physical
-- tables. A generic cached plan must not scan other containers' history.
WITH sources AS MATERIALIZED (
    SELECT src.id,src.containerid FROM statisticssources src
    JOIN containers c ON c.id=src.containerid WHERE c.id=ANY($1)
), raw AS NOT MATERIALIZED (
    -- OFFSET 0 keeps this as a parameterized per-source scan. Pulling the join
    -- above the scan can read unrelated sources or overestimate enough to JIT
    -- compile an otherwise small interactive query.
    SELECT s.* FROM sources src CROSS JOIN LATERAL (
        SELECT * FROM statisticssamples WHERE sourceid=src.id
            AND created>=$2 AND created<=$4 OFFSET 0
    ) s
), summaries AS MATERIALIZED (
    SELECT s.* FROM sources src CROSS JOIN LATERAL (
        SELECT * FROM statisticsrollups WHERE sourceid=src.id
            AND created>=$2 AND created<=$4 AND lastcreated<=$4 OFFSET 0
    ) s
), gauge_parts AS (
    SELECT sourceid,created/$3 AS bucket,MIN(created) AS created,COUNT(*) AS samplecount,
        SUM(cpuusage) AS cpuusage,SUM(memoryactive) AS memoryactive,SUM(memorycache) AS memorycache,
        SUM(memorylimit) AS memorylimit,SUM(rxbytes) AS rxbytes,SUM(txbytes) AS txbytes,
        SUM(blockreadbytes) AS blockreadbytes,SUM(blockwritebytes) AS blockwritebytes,
        COUNT(blockreadbytes) AS readcount,COUNT(blockwritebytes) AS writecount
    FROM raw GROUP BY sourceid,created/$3
    UNION ALL
    SELECT sourceid,created/$3,MIN(created),SUM(samplecount),
        SUM(cpuusage*samplecount),SUM(memoryactive*samplecount),SUM(memorycache*samplecount),
        SUM(memorylimit*samplecount),SUM(rxbytes*samplecount),SUM(txbytes*samplecount),
        SUM(blockreadbytes*readcount),SUM(blockwritebytes*writecount),SUM(readcount),SUM(writecount)
    FROM summaries GROUP BY sourceid,created/$3
), gauges AS (
    SELECT sourceid,bucket,MIN(created) AS created,
        SUM(cpuusage)/SUM(samplecount) AS cpuusage,SUM(memoryactive)/SUM(samplecount) AS memoryactive,
        SUM(memorycache)/SUM(samplecount) AS memorycache,SUM(memorylimit)/SUM(samplecount) AS memorylimit,
        SUM(rxbytes)/SUM(samplecount) AS rxbytes,SUM(txbytes)/SUM(samplecount) AS txbytes,
        SUM(blockreadbytes)/NULLIF(SUM(readcount),0) AS blockreadbytes,
        SUM(blockwritebytes)/NULLIF(SUM(writecount),0) AS blockwritebytes
    FROM gauge_parts GROUP BY sourceid,bucket
), counters AS (
    -- Summaries contribute two narrow boundaries. The end supplies the next
    -- observation's baseline; internal resets/durations are added separately.
    SELECT sourceid,created,0::smallint AS kind,rxbytes AS rx,txbytes AS tx,
        blockreadbytes AS read,blockwritebytes AS write FROM raw
    UNION ALL
    SELECT r.sourceid,e.created,e.kind,e.counters[1],e.counters[2],e.counters[3],e.counters[4]
    FROM summaries r CROSS JOIN LATERAL (VALUES
        (r.created,1::smallint,r.firstcounters),(r.lastcreated,2::smallint,r.lastcounters)
    ) e(created,kind,counters)
), samples AS (
    SELECT *,LAG(created) OVER observations AS previous_created,
        LAG(rx) OVER observations AS previous_rx,LAG(tx) OVER observations AS previous_tx,
        LAG(read) OVER observations AS previous_read,LAG(write) OVER observations AS previous_write
    FROM counters WINDOW observations AS (PARTITION BY sourceid ORDER BY created,kind)
), contributions AS (
    SELECT sourceid,created/$3 AS bucket,
        statistics_delta(rx,previous_rx) AS rxdelta,statistics_delta(tx,previous_tx) AS txdelta,
        statistics_delta(read,previous_read) AS readdelta,statistics_delta(write,previous_write) AS writedelta,
        CASE WHEN rx IS NOT NULL AND previous_rx IS NOT NULL THEN created-previous_created ELSE 0 END AS rxduration,
        CASE WHEN tx IS NOT NULL AND previous_tx IS NOT NULL THEN created-previous_created ELSE 0 END AS txduration,
        CASE WHEN read IS NOT NULL AND previous_read IS NOT NULL THEN created-previous_created ELSE 0 END AS readduration,
        CASE WHEN write IS NOT NULL AND previous_write IS NOT NULL THEN created-previous_created ELSE 0 END AS writeduration
    FROM samples WHERE kind<>2
    UNION ALL
    SELECT sourceid,created/$3,counterdeltas[1],counterdeltas[2],counterdeltas[3],counterdeltas[4],
        counterdurations[1],counterdurations[2],counterdurations[3],counterdurations[4]
    FROM summaries
), io AS (
    SELECT sourceid,bucket,SUM(rxdelta) AS rxbytesdelta,SUM(txdelta) AS txbytesdelta,
        SUM(readdelta) AS blockreadbytesdelta,SUM(writedelta) AS blockwritebytesdelta,
        SUM(rxdelta)/NULLIF(SUM(rxduration),0) AS rxbytespersecond,
        SUM(txdelta)/NULLIF(SUM(txduration),0) AS txbytespersecond,
        SUM(readdelta)/NULLIF(SUM(readduration),0) AS blockreadbytespersecond,
        SUM(writedelta)/NULLIF(SUM(writeduration),0) AS blockwritebytespersecond
    FROM contributions GROUP BY sourceid,bucket
)
SELECT src.containerid,g.created,g.cpuusage,g.memoryactive,g.memorycache,g.memorylimit,g.rxbytes,g.txbytes,
    g.blockreadbytes,g.blockwritebytes,io.rxbytesdelta,io.txbytesdelta,io.blockreadbytesdelta,io.blockwritebytesdelta,
    io.rxbytespersecond,io.txbytespersecond,io.blockreadbytespersecond,io.blockwritebytespersecond
FROM gauges g JOIN io USING(sourceid,bucket) JOIN sources src ON src.id=g.sourceid
ORDER BY g.created,src.containerid LIMIT 65537
