-- Same projection summary as .NET SwarmProjectionRepository.GetSummaryAsync.
-- Aggregate in PostgreSQL; do not materialize all Services/Tasks in the server.
WITH service_summary AS (
    SELECT COUNT(*) AS total,
        COUNT(*) FILTER (WHERE NOT isstale AND desiredtaskcount > 0 AND runningtaskcount >= desiredtaskcount) AS healthy,
        COUNT(*) FILTER (WHERE NOT isstale AND desiredtaskcount > 0 AND runningtaskcount > 0 AND runningtaskcount < desiredtaskcount) AS degraded,
        COUNT(*) FILTER (WHERE NOT isstale AND desiredtaskcount > 0 AND runningtaskcount <= 0) AS failed,
        COUNT(*) FILTER (WHERE NOT isstale AND desiredtaskcount <= 0) AS stopped,
        COUNT(*) FILTER (WHERE isstale) AS unknown,
        COALESCE(SUM(runningtaskcount), 0)::bigint AS runningtasks,
        COALESCE(SUM(desiredtaskcount), 0)::bigint AS desiredtasks
    FROM swarmserviceprojections WHERE platformid = $1 AND ownership <> 'System'
), node_summary AS (
    SELECT COUNT(*) AS nodecount,
        COUNT(*) FILTER (WHERE lower(role) = 'manager') AS managercount,
        COUNT(*) FILTER (WHERE lower(role) = 'manager' AND NOT isstale AND lower(reachability) = 'reachable') AS reachablemanagers,
        COALESCE(bool_or(lower(role) = 'manager' AND NOT isstale AND isleader), false) AS hasleader,
        COALESCE(bool_or(lower(role) = 'manager' AND isstale), false) AS managersstale
    FROM swarmnodeprojections WHERE platformid = $1
)
SELECT service_summary.*, node_summary.*,
    EXISTS (
        SELECT 1 FROM swarmnodeprojections WHERE platformid = $1 AND isstale
        UNION ALL SELECT 1 FROM swarmserviceprojections WHERE platformid = $1 AND isstale
        UNION ALL SELECT 1 FROM swarmtaskprojections WHERE platformid = $1 AND isstale
        UNION ALL SELECT 1 FROM swarmnetworkprojections WHERE platformid = $1 AND isstale
        UNION ALL SELECT 1 FROM swarmsecretprojections WHERE platformid = $1 AND isstale
        UNION ALL SELECT 1 FROM swarmconfigprojections WHERE platformid = $1 AND isstale
        UNION ALL SELECT 1 FROM swarmnodeimageprojections WHERE platformid = $1 AND isstale
        UNION ALL SELECT 1 FROM swarmnodevolumeprojections WHERE platformid = $1 AND isstale
        UNION ALL SELECT 1 FROM swarmnodenetworkprojections WHERE platformid = $1 AND isstale
    ) AS isstale,
    ((SELECT COUNT(*) FROM swarmnetworkprojections WHERE platformid = $1)
        + (SELECT COUNT(*) FROM swarmnodenetworkprojections WHERE platformid = $1)) AS networkcount,
    (SELECT COUNT(*) FROM swarmnodenetworkprojections WHERE platformid = $1) AS localnetworkcount,
    (SELECT COUNT(*) FROM swarmnodevolumeprojections WHERE platformid = $1) AS volumecount,
    (SELECT COUNT(*) FROM swarmnodeimageprojections WHERE platformid = $1) AS imagecount
FROM service_summary CROSS JOIN node_summary
