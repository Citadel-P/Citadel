WITH raw_samples AS (
    SELECT servicename,replicakey,containerid::text AS instance,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes
    FROM workloadstats
    WHERE (($2 AND deploymentid=$1) OR (NOT $2 AND stackid=$1)) AND created>=$3 AND created<=$4
    UNION ALL
    SELECT servicename,taskkey,platformid::text||':'||dockertaskid AS instance,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes,blockreadbytes,blockwritebytes
    FROM swarmservicestats
    WHERE NOT $2 AND stackid=$1 AND created>=$3 AND created<=$4
) , samples AS (
    SELECT *,LAG(rxbytes) OVER observations AS previous_rx,LAG(txbytes) OVER observations AS previous_tx,
        LAG(blockreadbytes) OVER observations AS previous_read,LAG(blockwritebytes) OVER observations AS previous_write,
        LAG(created) OVER observations AS previous_created
    FROM raw_samples WINDOW observations AS (PARTITION BY servicename,instance ORDER BY created)
), deltas AS (
    SELECT *,
        CASE WHEN previous_rx IS NULL THEN NULL WHEN rxbytes>=previous_rx THEN rxbytes-previous_rx ELSE rxbytes END AS rxbytesdelta,
        CASE WHEN previous_tx IS NULL THEN NULL WHEN txbytes>=previous_tx THEN txbytes-previous_tx ELSE txbytes END AS txbytesdelta,
        CASE WHEN previous_read IS NULL THEN NULL WHEN blockreadbytes>=previous_read THEN blockreadbytes-previous_read ELSE blockreadbytes END AS blockreadbytesdelta,
        CASE WHEN previous_write IS NULL THEN NULL WHEN blockwritebytes>=previous_write THEN blockwritebytes-previous_write ELSE blockwritebytes END AS blockwritebytesdelta
    FROM samples
), replica_buckets AS (
    -- Replacements in the same replica/slot share a bucket, rather than
    -- counting the retired and replacement instances as two replicas.
    SELECT servicename,replicakey,(created/$5) AS bucket,MIN(created) AS created,
        AVG(cpuusage) AS cpuusage,AVG(memoryactive) AS memoryactive,AVG(memorycache) AS memorycache,
        AVG(memorylimit) AS memorylimit,AVG(rxbytes) AS rxbytes,AVG(txbytes) AS txbytes,
        SUM(rxbytesdelta) AS rxbytesdelta,
        SUM(txbytesdelta) AS txbytesdelta,
        AVG(blockreadbytes) AS blockreadbytes,AVG(blockwritebytes) AS blockwritebytes,
        SUM(blockreadbytesdelta) AS blockreadbytesdelta,SUM(blockwritebytesdelta) AS blockwritebytesdelta,
        SUM(rxbytesdelta) / NULLIF(SUM(created-previous_created) FILTER (WHERE rxbytesdelta IS NOT NULL),0) AS rxbytespersecond,
        SUM(txbytesdelta) / NULLIF(SUM(created-previous_created) FILTER (WHERE txbytesdelta IS NOT NULL),0) AS txbytespersecond,
        SUM(blockreadbytesdelta) / NULLIF(SUM(created-previous_created) FILTER (WHERE blockreadbytesdelta IS NOT NULL),0) AS blockreadbytespersecond,
        SUM(blockwritebytesdelta) / NULLIF(SUM(created-previous_created) FILTER (WHERE blockwritebytesdelta IS NOT NULL),0) AS blockwritebytespersecond
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
