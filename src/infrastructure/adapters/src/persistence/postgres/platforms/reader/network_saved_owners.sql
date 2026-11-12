WITH actor_scope AS (
    SELECT id actorid FROM actors WHERE id=$1 AND isenabled
    UNION SELECT team.actorid FROM actorteammemberships membership
    JOIN teams team ON team.id=membership.teamid
    JOIN actors a ON a.id=team.actorid AND a.isenabled WHERE membership.memberactorid=$1
), grants AS (
    SELECT p.resourcetype kind,NULL::uuid resourceid,p.permissionlevel
    FROM actor_scope scope JOIN actorroles ar ON ar.actorid=scope.actorid
    JOIN permissions p ON p.roleid=ar.roleid
    UNION ALL SELECT a.resourcetype,a.resourceid,a.permissionlevel
    FROM actor_scope scope JOIN resourceaccesses a ON a.actorid=scope.actorid
), workloads AS (
    SELECT 1 kind,d.id,d.name,COALESCE(d.spec->'Networks',d.spec->'networks','[]'::jsonb) refs,NULL::text compose,NULL::text projectname,d.status,d.controlstate
    FROM deployments d WHERE d.platformid=$3
    UNION ALL
    SELECT 2,s.id,s.name,'[]'::jsonb,
        CASE WHEN octet_length(r.spec->>'composeFile')<=4194304 THEN r.spec->>'composeFile' END,r.spec->>'projectName',r.status,s.controlstate
    FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE r.platformid=$3
    UNION ALL
    SELECT 20,s.id,s.name,COALESCE(s.spec->'networkIds','[]'::jsonb),NULL,NULL,
