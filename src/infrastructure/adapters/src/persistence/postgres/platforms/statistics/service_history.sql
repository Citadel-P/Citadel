WITH samples AS (
    SELECT *,LAG(lastcounters[1]) OVER observations AS previous_rx,LAG(lastcounters[2]) OVER observations AS previous_tx,
        LAG(lastcounters[3]) OVER observations AS previous_read,LAG(lastcounters[4]) OVER observations AS previous_write,
        LAG(lastcreated) OVER observations AS previous_created
    FROM swarmservicestats
    WHERE created>=$6 AND created<=$8 AND lastcreated<=$8 AND (
        ($3::uuid IS NOT NULL AND (swarmserviceid=$3 OR (swarmserviceid IS NULL AND stackid IS NULL AND platformid=$1 AND dockerserviceid=$2))) OR
        ($3 IS NULL AND $4::uuid IS NOT NULL AND ((stackid=$4 AND servicename=$5) OR (swarmserviceid IS NULL AND stackid IS NULL AND platformid=$1 AND dockerserviceid=$2))) OR
        ($3 IS NULL AND $4 IS NULL AND platformid=$1 AND dockerserviceid=$2))
    WINDOW observations AS (PARTITION BY platformid,dockertaskid ORDER BY created)
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
), task_buckets AS (
    -- Gauges average by logical slot; traffic is calculated per physical Task
    -- before summing, so replacement and independent resets cannot inflate it.
    SELECT taskkey,(created/$7) AS bucket,MIN(created) AS created,SUM(cpuusage*samplecount)/NULLIF(SUM(samplecount),0) AS cpuusage,
        SUM(memoryactive*samplecount)/NULLIF(SUM(samplecount),0) AS memoryactive,SUM(memorycache*samplecount)/NULLIF(SUM(samplecount),0) AS memorycache,SUM(memorylimit*samplecount)/NULLIF(SUM(samplecount),0) AS memorylimit,
        SUM(rxbytes*samplecount)/NULLIF(SUM(samplecount),0) AS rxbytes,SUM(txbytes*samplecount)/NULLIF(SUM(samplecount),0) AS txbytes,
        SUM(rxbytesdelta) AS rxbytesdelta,
        SUM(txbytesdelta) AS txbytesdelta,
        SUM(blockreadbytes*readcount)/NULLIF(SUM(readcount),0) AS blockreadbytes,SUM(blockwritebytes*writecount)/NULLIF(SUM(writecount),0) AS blockwritebytes,
        SUM(blockreadbytesdelta) AS blockreadbytesdelta,SUM(blockwritebytesdelta) AS blockwritebytesdelta,
        SUM(rxbytesdelta) / NULLIF(SUM(rxbytesduration),0) AS rxbytespersecond,
        SUM(txbytesdelta) / NULLIF(SUM(txbytesduration),0) AS txbytespersecond,
        SUM(blockreadbytesdelta) / NULLIF(SUM(blockreadbytesduration),0) AS blockreadbytespersecond,
        SUM(blockwritebytesdelta) / NULLIF(SUM(blockwritebytesduration),0) AS blockwritebytespersecond
    FROM deltas GROUP BY taskkey,(created/$7)
)
SELECT '00000000-0000-0000-0000-000000000000'::uuid AS containerid,MIN(created) AS created,
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
FROM task_buckets GROUP BY bucket ORDER BY bucket
