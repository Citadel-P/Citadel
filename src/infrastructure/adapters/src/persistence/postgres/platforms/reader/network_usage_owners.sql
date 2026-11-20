workloads AS (
    SELECT 1 kind,d.id,d.name,'[]'::jsonb refs,NULL::text compose,NULL::text projectname,d.status,d.controlstate
    FROM deployments d WHERE d.platformid=$3
    UNION ALL
    SELECT 2,s.id,s.name,'[]'::jsonb,
        NULL::text,NULL::text,r.status,s.controlstate
    FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE r.platformid=$3
    UNION ALL
    SELECT 20,s.id,s.name,'[]'::jsonb,NULL,NULL,
