WITH samples AS (
    -- The first sample inside the requested range is a baseline, not traffic.
    -- Calculate before averaging so resets inside a bucket are preserved.
    SELECT *,LAG(rxbytes) OVER observations AS previous_rx,LAG(txbytes) OVER observations AS previous_tx
    FROM containerstats WHERE containerid=ANY($1) AND created>=$2 AND created<=$4
    WINDOW observations AS (PARTITION BY containerid ORDER BY created)
)
SELECT containerid,MIN(created) AS created,AVG(cpuusage) AS cpuusage,
    AVG(memoryactive) AS memoryactive,AVG(memorycache) AS memorycache,AVG(memorylimit) AS memorylimit,
    AVG(rxbytes) AS rxbytes,AVG(txbytes) AS txbytes,
    SUM(CASE WHEN previous_rx IS NULL THEN NULL WHEN rxbytes>=previous_rx THEN rxbytes-previous_rx ELSE rxbytes END) AS rxbytesdelta,
    SUM(CASE WHEN previous_tx IS NULL THEN NULL WHEN txbytes>=previous_tx THEN txbytes-previous_tx ELSE txbytes END) AS txbytesdelta
FROM samples GROUP BY containerid,(created/$3) ORDER BY MIN(created),containerid LIMIT 65537
