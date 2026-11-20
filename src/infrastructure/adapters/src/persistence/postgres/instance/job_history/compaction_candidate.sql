-- Limit the index walk before grouping. Lock and compact only one session/hour
-- per transaction, with at most 32 jobs and twelve fine windows per job.
WITH candidates AS (
    SELECT b.bootid, b.jobid, b.bucket / 3600000 * 3600000 AS hour
    FROM jobdiagnosticbuckets b
    JOIN jobdiagnosticsessions s ON s.bootid = b.bootid
    WHERE b.resolutionms = 300000 AND b.bucket >= $1 AND b.bucket < $2
      AND (s.stopped IS NOT NULL OR b.bucket / 3600000 < (s.snapshot->>'observedAt')::bigint / 3600000)
    ORDER BY b.bucket
    LIMIT 32
)
SELECT bootid, hour, array_agg(DISTINCT jobid)
FROM candidates
GROUP BY bootid, hour
ORDER BY hour, bootid
LIMIT 1
