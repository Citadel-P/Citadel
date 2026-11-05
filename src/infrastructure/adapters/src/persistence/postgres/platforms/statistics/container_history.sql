WITH samples AS (
    -- The first sample inside the requested range is a baseline, not traffic.
    -- Calculate before averaging so resets inside a bucket are preserved.
    SELECT *,LAG(rxbytes) OVER observations AS previous_rx,LAG(txbytes) OVER observations AS previous_tx,
        LAG(blockreadbytes) OVER observations AS previous_read,LAG(blockwritebytes) OVER observations AS previous_write,
        LAG(created) OVER observations AS previous_created
    FROM containerstats WHERE containerid=ANY($1) AND created>=$2 AND created<=$4
    WINDOW observations AS (PARTITION BY containerid ORDER BY created)
), deltas AS (
    SELECT *,
        CASE WHEN previous_rx IS NULL THEN NULL WHEN rxbytes>=previous_rx THEN rxbytes-previous_rx ELSE rxbytes END AS rxbytesdelta,
        CASE WHEN previous_tx IS NULL THEN NULL WHEN txbytes>=previous_tx THEN txbytes-previous_tx ELSE txbytes END AS txbytesdelta,
        CASE WHEN previous_read IS NULL THEN NULL WHEN blockreadbytes>=previous_read THEN blockreadbytes-previous_read ELSE blockreadbytes END AS blockreadbytesdelta,
        CASE WHEN previous_write IS NULL THEN NULL WHEN blockwritebytes>=previous_write THEN blockwritebytes-previous_write ELSE blockwritebytes END AS blockwritebytesdelta
    FROM samples
)
SELECT containerid,MIN(created) AS created,AVG(cpuusage) AS cpuusage,
    AVG(memoryactive) AS memoryactive,AVG(memorycache) AS memorycache,AVG(memorylimit) AS memorylimit,
    AVG(rxbytes) AS rxbytes,AVG(txbytes) AS txbytes,
    SUM(rxbytesdelta) AS rxbytesdelta,
    SUM(txbytesdelta) AS txbytesdelta,
    AVG(blockreadbytes) AS blockreadbytes,AVG(blockwritebytes) AS blockwritebytes,
    SUM(blockreadbytesdelta) AS blockreadbytesdelta,SUM(blockwritebytesdelta) AS blockwritebytesdelta,
    SUM(rxbytesdelta) / NULLIF(SUM(created-previous_created) FILTER (WHERE rxbytesdelta IS NOT NULL),0) AS rxbytespersecond,
    SUM(txbytesdelta) / NULLIF(SUM(created-previous_created) FILTER (WHERE txbytesdelta IS NOT NULL),0) AS txbytespersecond,
    SUM(blockreadbytesdelta) / NULLIF(SUM(created-previous_created) FILTER (WHERE blockreadbytesdelta IS NOT NULL),0) AS blockreadbytespersecond,
    SUM(blockwritebytesdelta) / NULLIF(SUM(created-previous_created) FILTER (WHERE blockwritebytesdelta IS NOT NULL),0) AS blockwritebytespersecond
FROM deltas GROUP BY containerid,(created/$3) ORDER BY MIN(created),containerid LIMIT 65537
