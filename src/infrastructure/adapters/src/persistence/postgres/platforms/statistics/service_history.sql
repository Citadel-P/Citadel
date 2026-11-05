WITH samples AS (
    SELECT *,LAG(rxbytes) OVER observations AS previous_rx,LAG(txbytes) OVER observations AS previous_tx
    FROM swarmservicestats
    WHERE created>=$6 AND created<=$8 AND (
        ($3::uuid IS NOT NULL AND (swarmserviceid=$3 OR (swarmserviceid IS NULL AND stackid IS NULL AND platformid=$1 AND dockerserviceid=$2))) OR
        ($3 IS NULL AND $4::uuid IS NOT NULL AND ((stackid=$4 AND servicename=$5) OR (swarmserviceid IS NULL AND stackid IS NULL AND platformid=$1 AND dockerserviceid=$2))) OR
        ($3 IS NULL AND $4 IS NULL AND platformid=$1 AND dockerserviceid=$2))
    WINDOW observations AS (PARTITION BY platformid,dockertaskid ORDER BY created)
), task_buckets AS (
    -- Gauges average by logical slot; traffic is calculated per physical Task
    -- before summing, so replacement and independent resets cannot inflate it.
    SELECT taskkey,(created/$7) AS bucket,MIN(created) AS created,AVG(cpuusage) AS cpuusage,
        AVG(memoryactive) AS memoryactive,AVG(memorycache) AS memorycache,AVG(memorylimit) AS memorylimit,
        AVG(rxbytes) AS rxbytes,AVG(txbytes) AS txbytes,
        SUM(CASE WHEN previous_rx IS NULL THEN NULL WHEN rxbytes>=previous_rx THEN rxbytes-previous_rx ELSE rxbytes END) AS rxbytesdelta,
        SUM(CASE WHEN previous_tx IS NULL THEN NULL WHEN txbytes>=previous_tx THEN txbytes-previous_tx ELSE txbytes END) AS txbytesdelta
    FROM samples GROUP BY taskkey,(created/$7)
)
SELECT '00000000-0000-0000-0000-000000000000'::uuid AS containerid,MIN(created) AS created,
    SUM(cpuusage) AS cpuusage,SUM(memoryactive) AS memoryactive,SUM(memorycache) AS memorycache,
    SUM(memorylimit) AS memorylimit,SUM(rxbytes) AS rxbytes,SUM(txbytes) AS txbytes,
    SUM(rxbytesdelta) AS rxbytesdelta,SUM(txbytesdelta) AS txbytesdelta
FROM task_buckets GROUP BY bucket ORDER BY bucket
