SELECT s.id, COALESCE(r.gitcheckrevision,r.id) AS release_revision,
       COALESCE(ref.gitcheckrevision,ref.id) AS ref_revision,
       CASE WHEN s.checkedgitinputs=ARRAY[COALESCE(r.gitcheckrevision,r.id),COALESCE(ref.gitcheckrevision,ref.id)]
            THEN s.rowversion END AS pending_version
FROM stacks s
JOIN stackreleases r ON r.id=s.currentstackreleaseid
JOIN platforms p ON p.id=r.platformid
JOIN gitrepositoryrefs ref ON ref.gitrepositoryid=CASE WHEN pg_input_is_valid(r.spec->>'GitRepoId','uuid')
                                THEN (r.spec->>'GitRepoId')::uuid END
                         AND ref.branch=r.spec->>'Branch'
                         -- Indexed dependency key. The typed joins above retain exact UUID and branch semantics.
                         AND r.spec ? 'GitRepoId'
                         AND (translate(lower(r.spec->>'GitRepoId'), '{}-', '') || ':' || (r.spec->>'Branch'))
                             = (replace(ref.gitrepositoryid::text,'-','') || ':' || ref.branch)
WHERE ($3::uuid IS NULL OR ref.id=$3) AND s.id>$1 AND s.controlstate='Idle' AND p.status='Online'
  AND r.status IN ('Healthy','Degraded')
  AND r.spec->>'UpdateBehavior' IN ('Notify','StackAutoDeploy','ServiceAutoDeploy')
  AND s.stacksource='Git' AND COALESCE(r.spec->>'CommitSha','')=''
  AND ref.status='Healthy'
  AND (s.checkedgitinputs IS DISTINCT FROM ARRAY[COALESCE(r.gitcheckrevision,r.id),COALESCE(ref.gitcheckrevision,ref.id)]
       OR (s.stackupdatestate->'RecreateStackOnNewCommitState'->>'RemoteCommitSha' IS NOT NULL
           AND (r.spec->>'UpdateBehavior' <> 'StackAutoDeploy' OR
                r.lastgitapplysyncat IS NULL OR ref.lastsyncedat > r.lastgitapplysyncat)
           AND ref.lastsyncedat > CASE
               WHEN s.stackupdatestate->'RecreateStackOnNewCommitState'->>'LastCheckedAt' ~ '^[0-9]{4}-'
               THEN (s.stackupdatestate->'RecreateStackOnNewCommitState'->>'LastCheckedAt')::timestamptz
               ELSE '-infinity'::timestamptz END))
ORDER BY s.id LIMIT $2
