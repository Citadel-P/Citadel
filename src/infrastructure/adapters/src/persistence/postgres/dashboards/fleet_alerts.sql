, events AS (
    SELECT o.kind,o.id,e.id AS target,e.createdat,e.type,e.severity FROM alertevents e
    JOIN owners o ON o.id=e.resourceid AND o.kind=CASE WHEN e.type='WebhookGitRepoSyncFailed' AND e.resourcetype='Webhook' THEN 'GitRepository' ELSE e.resourcetype END
    WHERE e.resolvedat IS NULL AND ($2 OR 6 IN (SELECT permission FROM global_grants) OR (6,e.id) IN (SELECT permission,id FROM grants))
)
