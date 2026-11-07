WITH raw_samples AS (
    SELECT servicename,replicakey,containerid::text AS instance,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes,
        samplecount,readcount,writecount,lastcreated,firstcounters,lastcounters,counterdeltas,counterdurations
    FROM workloadstats
    WHERE (($2 AND deploymentid=$1) OR (NOT $2 AND stackid=$1)) AND created>=$3 AND created<=$4 AND lastcreated<=$4
    UNION ALL
    SELECT servicename,taskkey,platformid::text||':'||dockertaskid AS instance,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes,
        samplecount,readcount,writecount,lastcreated,firstcounters,lastcounters,counterdeltas,counterdurations
    FROM swarmservicestats
    WHERE NOT $2 AND stackid=$1 AND created>=$3 AND created<=$4 AND lastcreated<=$4
) , samples AS (
    SELECT *,LAG(lastcounters[1]) OVER observations AS previous_rx,LAG(lastcounters[2]) OVER observations AS previous_tx,
        LAG(lastcounters[3]) OVER observations AS previous_read,LAG(lastcounters[4]) OVER observations AS previous_write,
        LAG(lastcreated) OVER observations AS previous_created
    FROM raw_samples WINDOW observations AS (PARTITION BY servicename,instance ORDER BY created)
), deltas AS (
    SELECT *,
        statistics_sum(counterdeltas[1],statistics_delta(firstcounters[1],previous_rx)) AS rxbytesdelta,
        counterdurations[1]+CASE WHEN firstcounters[1] IS NOT NULL AND previous_rx IS NOT NULL THEN created-previous_created ELSE 0 END AS rxbytesduration,
        statistics_sum(counterdeltas[2],statistics_delta(firstcounters[2],previous_tx)) AS txbytesdelta,
        counterdurations[2]+CASE WHEN firstcounters[2] IS NOT NULL AND previous_tx IS NOT NULL THEN created-previous_created ELSE 0 END AS txbytesduration,
        statistics_sum(counterdeltas[3],statistics_delta(firstcounters[3],previous_read)) AS blockreadbytesdelta,
        counterdurations[3]+CASE WHEN firstcounters[3] IS NOT NULL AND previous_read IS NOT NULL THEN created-previous_created ELSE 0 END AS blockreadbytesduration,
        statistics_sum(counterdeltas[4],statistics_delta(firstcounters[4],previous_write)) AS blockwritebytesdelta,
        counterdurations[4]+CASE WHEN firstcounters[4] IS NOT NULL AND previous_write IS NOT NULL THEN created-previous_created ELSE 0 END AS blockwritebytesduration
    FROM samples
), replica_buckets AS (
    -- Replacements in the same replica/slot share a bucket, rather than
    -- counting the retired and replacement instances as two replicas.
    SELECT servicename,replicakey,(created/$5) AS bucket,MIN(created) AS created,
        SUM(cpuusage*samplecount)/NULLIF(SUM(samplecount),0) AS cpuusage,SUM(memoryactive*samplecount)/NULLIF(SUM(samplecount),0) AS memoryactive,SUM(memorycache*samplecount)/NULLIF(SUM(samplecount),0) AS memorycache,
        SUM(memorylimit*samplecount)/NULLIF(SUM(samplecount),0) AS memorylimit,SUM(rxbytes*samplecount)/NULLIF(SUM(samplecount),0) AS rxbytes,SUM(txbytes*samplecount)/NULLIF(SUM(samplecount),0) AS txbytes,
        SUM(rxbytesdelta) AS rxbytesdelta,
        SUM(txbytesdelta) AS txbytesdelta,
        SUM(blockreadbytes*readcount)/NULLIF(SUM(readcount),0) AS blockreadbytes,SUM(blockwritebytes*writecount)/NULLIF(SUM(writecount),0) AS blockwritebytes,
        SUM(blockreadbytesdelta) AS blockreadbytesdelta,SUM(blockwritebytesdelta) AS blockwritebytesdelta,
        SUM(rxbytesdelta) / NULLIF(SUM(rxbytesduration),0) AS rxbytespersecond,
        SUM(txbytesdelta) / NULLIF(SUM(txbytesduration),0) AS txbytespersecond,
        SUM(blockreadbytesdelta) / NULLIF(SUM(blockreadbytesduration),0) AS blockreadbytespersecond,
        SUM(blockwritebytesdelta) / NULLIF(SUM(blockwritebytesduration),0) AS blockwritebytespersecond
    FROM deltas GROUP BY servicename,replicakey,(created/$5)
)
SELECT servicename,'00000000-0000-0000-0000-000000000000'::uuid AS containerid,MIN(created) AS created,
    SUM(cpuusage) AS cpuusage,SUM(memoryactive) AS memoryactive,SUM(memorycache) AS memorycache,
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
FROM replica_buckets GROUP BY servicename,bucket ORDER BY bucket,servicename LIMIT 65537
