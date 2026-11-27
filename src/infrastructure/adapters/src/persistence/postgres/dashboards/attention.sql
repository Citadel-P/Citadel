, reasons(kind,id,priority,rank,code,target,at,evidence) AS (
    SELECT kind,id,CASE WHEN state='Degraded' THEN 1 ELSE 0 END,
           CASE state WHEN 'Offline' THEN 0 WHEN 'Failed' THEN 1 ELSE 4 END,state,NULL::uuid,NULL::timestamptz,NULL::text
    FROM owners WHERE (kind='Platform' AND state='Offline') OR (kind IN ('Deployment','Stack','SwarmService') AND state IN ('Failed','Degraded'))
    UNION ALL
    SELECT kind,id,CASE severity WHEN 'Critical' THEN 0 ELSE 1 END,2,severity||' alert',target,createdat,type
    FROM events WHERE severity IN ('Critical','Warning') AND type NOT IN ('DeploymentImageUpdateAvailable','StackImageUpdateAvailable','StackGitUpdateAvailable')
    UNION ALL
    SELECT o.kind,o.id,1,3,'Backup '||r.status,r.id,r.completedat,NULL FROM owners o JOIN backuppolicies p ON p.id=o.id AND o.kind='BackupPolicy' AND p.enabled
    CROSS JOIN LATERAL (SELECT id,status,completedat FROM backupruns WHERE backuppolicyid=p.id AND completedat IS NOT NULL AND status<>'Cancelled' ORDER BY completedat DESC,id DESC LIMIT 1) r
    WHERE r.status IN ('Failed','TimedOut','Rejected','Interrupted','SucceededWithWarnings')
    UNION ALL
    SELECT kind,id,2,5,'Image update',NULL,checked,digest
    FROM owners WHERE digest IS NOT NULL
    UNION ALL
    SELECT o.kind,o.id,2,5,'New Git commit',NULL,CASE WHEN pg_input_is_valid(g->>'LastCheckedAt','timestamptz') THEN (g->>'LastCheckedAt')::timestamptz END,g->>'RemoteCommitSha'
    FROM owners o JOIN stacks s ON s.id=o.id AND o.kind='Stack' JOIN stackreleases r ON r.id=s.currentstackreleaseid
    CROSS JOIN LATERAL (SELECT s.stackupdatestate->'RecreateStackOnNewCommitState' AS g) u
    WHERE s.stacksource='Git' AND r.source->>'SourceType'='Git' AND pg_input_is_valid(r.spec->>'GitRepoId','uuid') AND translate(lower(r.source->>'GitRepositoryId'),'{}-','')=translate(lower(r.spec->>'GitRepoId'),'{}-','') AND r.source->>'Branch'=r.spec->>'Branch'
    AND nullif(g->>'CurrentCommitSha','')=r.source->>'ResolvedCommitSha' AND nullif(g->>'RemoteCommitSha','')<>g->>'CurrentCommitSha'
    UNION ALL
    SELECT o.kind,o.id,2,5,'Image update',NULL,(i->>'LastCheckedAt')::timestamptz,i->>'RemoteDigest'
    FROM owners o JOIN stacks s ON s.id=o.id AND o.kind='Stack' JOIN stackreleases r ON r.id=s.currentstackreleaseid
    CROSS JOIN LATERAL json_array_elements(CASE WHEN json_typeof(s.stackupdatestate->'RecreateStackOnNewImageState'->'AutoUpdateStates')='array' THEN s.stackupdatestate->'RecreateStackOnNewImageState'->'AutoUpdateStates' ELSE '[]'::json END) i
    WHERE CASE WHEN pg_input_is_valid(i->>'LastCheckedAt','timestamptz') THEN (i->>'LastCheckedAt')::timestamptz >= r.createdat ELSE false END AND i->>'UpdateAvailable'='true' AND nullif(i->>'CurrentDigest','')<>nullif(i->>'RemoteDigest','')
), first_reason AS (
    SELECT DISTINCT ON (kind,id) kind,id,priority,rank,at FROM reasons ORDER BY kind,id,priority,rank,at NULLS LAST,target
), classified AS (
    SELECT kind,id,bool_or(priority<2) AS problem,bool_or(priority=2) AS update FROM reasons GROUP BY kind,id
), page AS (
    SELECT f.* FROM first_reason f JOIN classified c USING(kind,id)
    WHERE $4='All' OR ($4='Problems' AND c.problem) OR ($4='Updates' AND c.update)
    ORDER BY priority,rank,at NULLS LAST,kind,id LIMIT $5 OFFSET $6
), preview_reasons AS (
    SELECT DISTINCT ON (r.kind,r.id,r.code) r.*,count(*) OVER (PARTITION BY r.kind,r.id,r.code) AS occurrences
    FROM page JOIN reasons r USING(kind,id)
    ORDER BY r.kind,r.id,r.code,r.at NULLS LAST,r.target,r.evidence
), detail AS (
    SELECT p.kind,p.id,o.platform_id AS "platformId",o.name,o.state,p.priority,p.rank,p.at,
        (SELECT jsonb_agg(jsonb_build_object('code',r.code,'target',coalesce(r.target,(SELECT target FROM events e WHERE e.kind=r.kind AND e.id=r.id AND
              e.type=CASE WHEN r.code='New Git commit' THEN 'StackGitUpdateAvailable' WHEN r.code='Image update' AND r.kind='Deployment' THEN 'DeploymentImageUpdateAvailable' WHEN r.code='Image update' AND r.kind='Stack' THEN 'StackImageUpdateAvailable' END ORDER BY createdat,target LIMIT 1)),'occurrences',r.occurrences,'observedAt',r.at,'evidence',r.evidence) ORDER BY r.priority,r.rank,r.at NULLS LAST,r.code,r.target,r.evidence)
         FROM preview_reasons r WHERE r.kind=p.kind AND r.id=p.id) AS reasons
    FROM page p JOIN owners o USING(kind,id)
)
SELECT jsonb_build_object('states','[]'::jsonb,'alerts',0,
    'total',count(*),'problems',count(*) FILTER(WHERE problem),'updates',count(*) FILTER(WHERE update),
    'rows',coalesce((SELECT jsonb_agg(to_jsonb(d)-'priority'-'rank'-'at' ORDER BY priority,rank,at NULLS LAST,kind,id) FROM detail d),'[]'))
FROM classified
