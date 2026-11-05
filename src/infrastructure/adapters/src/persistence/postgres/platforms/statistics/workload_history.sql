WITH raw_samples AS (
    SELECT servicename,replicakey,containerid::text AS instance,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes
    FROM workloadstats
    WHERE (($2 AND deploymentid=$1) OR (NOT $2 AND stackid=$1)) AND created>=$3 AND created<=$4
    UNION ALL
    SELECT servicename,taskkey,platformid::text||':'||dockertaskid AS instance,created,cpuusage,memoryactive,memorycache,memorylimit,rxbytes,txbytes
    FROM swarmservicestats
    WHERE NOT $2 AND stackid=$1 AND created>=$3 AND created<=$4
) , samples AS (
    SELECT *,LAG(rxbytes) OVER observations AS previous_rx,LAG(txbytes) OVER observations AS previous_tx
    FROM raw_samples WINDOW observations AS (PARTITION BY servicename,instance ORDER BY created)
), replica_buckets AS (
    -- Replacements in the same replica/slot share a bucket, rather than
    -- counting the retired and replacement instances as two replicas.
    SELECT servicename,replicakey,(created/$5) AS bucket,MIN(created) AS created,
        AVG(cpuusage) AS cpuusage,AVG(memoryactive) AS memoryactive,AVG(memorycache) AS memorycache,
        AVG(memorylimit) AS memorylimit,AVG(rxbytes) AS rxbytes,AVG(txbytes) AS txbytes,
        SUM(CASE WHEN previous_rx IS NULL THEN NULL WHEN rxbytes>=previous_rx THEN rxbytes-previous_rx ELSE rxbytes END) AS rxbytesdelta,
        SUM(CASE WHEN previous_tx IS NULL THEN NULL WHEN txbytes>=previous_tx THEN txbytes-previous_tx ELSE txbytes END) AS txbytesdelta
    FROM samples GROUP BY servicename,replicakey,(created/$5)
)
SELECT servicename,'00000000-0000-0000-0000-000000000000'::uuid AS containerid,MIN(created) AS created,
    SUM(cpuusage) AS cpuusage,SUM(memoryactive) AS memoryactive,SUM(memorycache) AS memorycache,
    SUM(memorylimit) AS memorylimit,SUM(rxbytes) AS rxbytes,SUM(txbytes) AS txbytes,
    SUM(rxbytesdelta) AS rxbytesdelta,SUM(txbytesdelta) AS txbytesdelta
FROM replica_buckets GROUP BY servicename,bucket ORDER BY bucket,servicename LIMIT 65537
