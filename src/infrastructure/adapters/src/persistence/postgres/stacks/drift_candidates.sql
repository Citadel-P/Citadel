SELECT s.id,s.name,s.controlstate,s.rowversion,s.currentstackreleaseid,s.driftpolicy,
       COALESCE(r.gitcheckrevision,r.id) AS input_revision,r.spec,r.platformid,r.status,COALESCE(r.source->>'ResolvedCommitSha',r.source->>'resolvedCommitSha') AS deployed_commit,
       p.status AS platform_status,COALESCE(p.platformdescriptor->>'$type',p.platformdescriptor->>'type','Docker') AS platform_kind,
       CASE WHEN last.info->>'$type'='StackDriftDetected'
            THEN COALESCE(last.info->>'Fingerprint',last.info->>'fingerprint') END AS previous_fingerprint
FROM stacks s
JOIN stackreleases r ON r.id=s.currentstackreleaseid
JOIN platforms p ON p.id=r.platformid
LEFT JOIN LATERAL (
    SELECT a.info::jsonb AS info FROM activityevents a WHERE a.resourceid=s.id AND a.resourcetype='Stack'
      AND a.eventtype NOT IN ('StackUpdated','StackRenamed')
    ORDER BY a.createdat DESC,a.id DESC LIMIT 1
) last ON true
WHERE s.controlstate='Idle' AND r.status IN ('Healthy','Degraded')
  AND COALESCE(s.driftpolicy->>'Mode','Disabled')<>'Disabled'
  AND ($1::uuid IS NULL OR s.id>$1)
ORDER BY s.id LIMIT $2
