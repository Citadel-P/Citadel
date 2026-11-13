-- Resolve historical owners first; filter physical scans by owner and time.
-- One owner-array InitPlan avoids a generic plan repeating a date scan per owner.
-- Keep only scalar endpoints in the counter window; combine physical deltas
-- before logical replica aggregation. Deleted projections are not consulted.
WITH owners AS MATERIALIZED (
    SELECT o.id,o.sourceid,o.servicename,o.replicakey FROM statisticsowners o
    JOIN statisticssources src ON src.id=o.sourceid WHERE o.kind='Service' AND (
        ($3::uuid IS NOT NULL AND (o.swarmserviceid=$3 OR (o.swarmserviceid IS NULL AND o.servicestackid IS NULL AND src.platformid=$1 AND o.dockerserviceid=$2))) OR
        ($3 IS NULL AND $4::uuid IS NOT NULL AND ((o.servicestackid=$4 AND o.servicename=$5) OR (o.swarmserviceid IS NULL AND o.servicestackid IS NULL AND src.platformid=$1 AND o.dockerserviceid=$2))) OR
        ($3 IS NULL AND $4 IS NULL AND src.platformid=$1 AND o.dockerserviceid=$2))
), raw AS NOT MATERIALIZED (
    SELECT * FROM statisticssamples WHERE ownerid IS NOT NULL AND ownerid=ANY(ARRAY(SELECT id FROM owners))
        AND created>=$6 AND created<=$8
), summaries AS MATERIALIZED (
    SELECT sourceid,ownerid,created,lastcreated,cpuusage,memoryactive,memoryactivecache,memorycache,memorylimit,
        rxbytes,txbytes,blockreadbytes,blockwritebytes,samplecount,readcount,writecount,
        firstcounters,lastcounters,counterdeltas,counterdurations
    FROM statisticsrollups WHERE ownerid IS NOT NULL AND ownerid=ANY(ARRAY(SELECT id FROM owners))
        AND created>=$6 AND created<=$8 AND lastcreated<=$8
), gauge_parts AS (
    SELECT o.replicakey,r.created/$7 AS bucket,MIN(r.created) AS created,COUNT(*) AS samplecount,
        SUM(r.cpuusage) AS cpuusage,SUM(r.memoryactive) AS memoryactive,
        SUM(r.memoryactivecache) AS memoryactivecache,COUNT(r.memoryactivecache) AS cachecount,SUM(r.memorycache) AS memorycache,
        SUM(r.memorylimit) AS memorylimit,SUM(r.rxbytes) AS rxbytes,SUM(r.txbytes) AS txbytes,
        SUM(r.blockreadbytes) AS blockreadbytes,SUM(r.blockwritebytes) AS blockwritebytes,
        COUNT(r.blockreadbytes) AS readcount,COUNT(r.blockwritebytes) AS writecount
    FROM raw r JOIN owners o ON o.id=r.ownerid GROUP BY o.replicakey,r.created/$7
    UNION ALL
    SELECT o.replicakey,r.created/$7,MIN(r.created),SUM(r.samplecount),
        SUM(r.cpuusage*r.samplecount),SUM(r.memoryactive*r.samplecount),SUM(r.memoryactivecache*r.samplecount),SUM(CASE WHEN r.memoryactivecache IS NOT NULL THEN r.samplecount ELSE 0 END),SUM(r.memorycache*r.samplecount),
        SUM(r.memorylimit*r.samplecount),SUM(r.rxbytes*r.samplecount),SUM(r.txbytes*r.samplecount),
        SUM(r.blockreadbytes*r.readcount),SUM(r.blockwritebytes*r.writecount),SUM(r.readcount),SUM(r.writecount)
    FROM summaries r JOIN owners o ON o.id=r.ownerid GROUP BY o.replicakey,r.created/$7
), counters AS (
    -- Summaries contribute two narrow boundaries. The end supplies the next
    -- observation's baseline; internal resets/durations are added separately.
    SELECT sourceid,ownerid,created,0::smallint AS kind,rxbytes AS rx,txbytes AS tx,
        blockreadbytes AS read,blockwritebytes AS write FROM raw
    UNION ALL
    SELECT r.sourceid,r.ownerid,e.created,e.kind,e.counters[1],e.counters[2],e.counters[3],e.counters[4]
    FROM summaries r CROSS JOIN LATERAL (VALUES
        (r.created,1::smallint,r.firstcounters),(r.lastcreated,2::smallint,r.lastcounters)
    ) e(created,kind,counters)
), samples AS (
    SELECT counters.*,LAG(created) OVER observations AS previous_created,
        LAG(rx) OVER observations AS previous_rx,LAG(tx) OVER observations AS previous_tx,
        LAG(read) OVER observations AS previous_read,LAG(write) OVER observations AS previous_write
    FROM counters JOIN owners o ON o.id=counters.ownerid WINDOW observations AS (PARTITION BY counters.sourceid ORDER BY created,kind)
), contributions AS (
    SELECT ownerid,created/$7 AS bucket,
        statistics_delta(rx,previous_rx) AS rxdelta,statistics_delta(tx,previous_tx) AS txdelta,
        statistics_delta(read,previous_read) AS readdelta,statistics_delta(write,previous_write) AS writedelta,
        CASE WHEN rx IS NOT NULL AND previous_rx IS NOT NULL THEN created-previous_created ELSE 0 END AS rxduration,
        CASE WHEN tx IS NOT NULL AND previous_tx IS NOT NULL THEN created-previous_created ELSE 0 END AS txduration,
        CASE WHEN read IS NOT NULL AND previous_read IS NOT NULL THEN created-previous_created ELSE 0 END AS readduration,
        CASE WHEN write IS NOT NULL AND previous_write IS NOT NULL THEN created-previous_created ELSE 0 END AS writeduration
    FROM samples WHERE kind<>2
    UNION ALL
    SELECT ownerid,created/$7,counterdeltas[1],counterdeltas[2],counterdeltas[3],counterdeltas[4],
        counterdurations[1],counterdurations[2],counterdurations[3],counterdurations[4]
    FROM summaries
), io AS (
    -- Only now combine physical-source deltas into a logical replica.
    SELECT o.replicakey,c.bucket,SUM(rxdelta) AS rxbytesdelta,SUM(txdelta) AS txbytesdelta,
        SUM(readdelta) AS blockreadbytesdelta,SUM(writedelta) AS blockwritebytesdelta,
        SUM(rxdelta)/NULLIF(SUM(rxduration),0) AS rxbytespersecond,
        SUM(txdelta)/NULLIF(SUM(txduration),0) AS txbytespersecond,
        SUM(readdelta)/NULLIF(SUM(readduration),0) AS blockreadbytespersecond,
        SUM(writedelta)/NULLIF(SUM(writeduration),0) AS blockwritebytespersecond
    FROM contributions c JOIN owners o ON o.id=c.ownerid GROUP BY o.replicakey,c.bucket
), gauges AS (
    SELECT replicakey,bucket,MIN(created) AS created,
        SUM(cpuusage)/SUM(samplecount) AS cpuusage,SUM(memoryactive)/SUM(samplecount) AS memoryactive,
        CASE WHEN SUM(cachecount)=SUM(samplecount) THEN SUM(memoryactivecache)/SUM(samplecount) END AS memoryactivecache,
        SUM(memorycache)/SUM(samplecount) AS memorycache,SUM(memorylimit)/SUM(samplecount) AS memorylimit,
        SUM(rxbytes)/SUM(samplecount) AS rxbytes,SUM(txbytes)/SUM(samplecount) AS txbytes,
        SUM(blockreadbytes)/NULLIF(SUM(readcount),0) AS blockreadbytes,
        SUM(blockwritebytes)/NULLIF(SUM(writecount),0) AS blockwritebytes
    FROM gauge_parts GROUP BY replicakey,bucket
), replica_buckets AS (
    SELECT g.*,io.rxbytesdelta,io.txbytesdelta,io.blockreadbytesdelta,io.blockwritebytesdelta,
        io.rxbytespersecond,io.txbytespersecond,io.blockreadbytespersecond,io.blockwritebytespersecond
    FROM gauges g JOIN io USING(replicakey,bucket)
)

SELECT '00000000-0000-0000-0000-000000000000'::uuid AS containerid,MIN(created) AS created,
    SUM(cpuusage) AS cpuusage,SUM(memoryactive) AS memoryactive,CASE WHEN COUNT(memoryactivecache)=COUNT(*) THEN SUM(memoryactivecache) END AS memoryactivecache,SUM(memorycache) AS memorycache,
    SUM(memorylimit) AS memorylimit,SUM(rxbytes) AS rxbytes,SUM(txbytes) AS txbytes,
    SUM(rxbytesdelta) AS rxbytesdelta,SUM(txbytesdelta) AS txbytesdelta,
    SUM(blockreadbytes) AS blockreadbytes,
    SUM(blockwritebytes) AS blockwritebytes,
    SUM(blockreadbytesdelta) AS blockreadbytesdelta,
    SUM(blockwritebytesdelta) AS blockwritebytesdelta,
    SUM(rxbytespersecond) AS rxbytespersecond,
    SUM(txbytespersecond) AS txbytespersecond,
    SUM(blockreadbytespersecond) AS blockreadbytespersecond,
    SUM(blockwritebytespersecond) AS blockwritebytespersecond
FROM replica_buckets GROUP BY bucket ORDER BY bucket LIMIT 65537
