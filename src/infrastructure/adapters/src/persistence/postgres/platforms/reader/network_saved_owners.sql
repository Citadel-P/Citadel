workloads AS (
    SELECT 1 kind,d.id,d.name,COALESCE(d.spec->'Networks',d.spec->'networks','[]'::jsonb) refs,NULL::uuid releaseid,d.status,d.controlstate
    FROM deployments d WHERE d.platformid=$3
    UNION ALL
    SELECT 2,s.id,s.name,'[]'::jsonb,r.id,r.status,s.controlstate
    FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE r.platformid=$3
    UNION ALL
    SELECT 20,s.id,s.name,COALESCE(s.spec->'networkIds','[]'::jsonb),NULL,
