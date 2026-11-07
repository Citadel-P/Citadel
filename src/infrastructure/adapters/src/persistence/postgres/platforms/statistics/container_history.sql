WITH samples AS (
    -- The first sample inside the requested range is a baseline, not traffic.
    -- Calculate before averaging so resets inside a bucket are preserved.
    SELECT *,LAG(lastcounters[1]) OVER observations AS previous_rx,LAG(lastcounters[2]) OVER observations AS previous_tx,
        LAG(lastcounters[3]) OVER observations AS previous_read,LAG(lastcounters[4]) OVER observations AS previous_write,
        LAG(lastcreated) OVER observations AS previous_created
    FROM containerstats WHERE containerid=ANY($1) AND created>=$2 AND created<=$4 AND lastcreated<=$4
    WINDOW observations AS (PARTITION BY containerid ORDER BY created)
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
)
SELECT containerid,MIN(created) AS created,SUM(cpuusage*samplecount)/NULLIF(SUM(samplecount),0) AS cpuusage,
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
FROM deltas GROUP BY containerid,(created/$3) ORDER BY MIN(created),containerid LIMIT 65537
