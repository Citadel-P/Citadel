, targets(kind, permission_type, id) AS (
    SELECT 'Platform', 0, id FROM platforms
    UNION ALL
    SELECT 'Deployment', 1, id FROM deployments
    UNION ALL
    SELECT 'Stack', 2, s.id FROM stacks s
    JOIN stackreleases r ON r.id=s.currentstackreleaseid
    JOIN platforms p ON p.id=r.platformid
    UNION ALL
    SELECT 'Registry', 3, id FROM registries
    UNION ALL
    SELECT 'GitRepository', 4, id FROM gitrepositories
    UNION ALL
    SELECT 'Script', 13, id FROM scripts
    UNION ALL
    SELECT 'BackupPolicy', 16, id FROM backuppolicies WHERE archivedat IS NULL
    UNION ALL
    SELECT 'Build', 18, id FROM buildprojects WHERE archivedat IS NULL
    UNION ALL
    SELECT 'BuildAgentPool', 19, id FROM buildagentpools WHERE archivedat IS NULL
    UNION ALL
    SELECT 'SwarmService', 20, id FROM swarmservices
)
