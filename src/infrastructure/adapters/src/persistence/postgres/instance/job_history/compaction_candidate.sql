-- The partial index visits only retained fine windows. One job/hour is at most
-- twelve rows. Never seal the hour of a reporter's latest committed checkpoint:
-- it can still hold a cumulative window for that hour in memory.
SELECT b.bootid, b.jobid, b.bucket/3600000*3600000
FROM jobdiagnosticbuckets b
JOIN jobdiagnosticsessions s ON s.bootid=b.bootid
WHERE b.resolutionms=300000 AND b.bucket >= $1 AND b.bucket < $2
  AND (s.stopped IS NOT NULL OR b.bucket/3600000 < (s.snapshot->>'observedAt')::bigint/3600000)
ORDER BY b.bucket
LIMIT 1
