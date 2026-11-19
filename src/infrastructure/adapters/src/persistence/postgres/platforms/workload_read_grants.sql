WITH actor_scope AS (
    SELECT id actorid FROM actors WHERE id=$1 AND isenabled
    UNION SELECT team.actorid FROM actorteammemberships membership
    JOIN teams team ON team.id=membership.teamid
    JOIN actors a ON a.id=team.actorid AND a.isenabled WHERE membership.memberactorid=$1
), readable_types AS (
    SELECT p.resourcetype kind
    FROM actor_scope scope JOIN actorroles ar ON ar.actorid=scope.actorid
    JOIN permissions p ON p.roleid=ar.roleid
    WHERE p.permissionlevel IN (1,2,4)
), readable_resources AS (
    SELECT a.resourcetype kind,a.resourceid
    FROM actor_scope scope JOIN resourceaccesses a ON a.actorid=scope.actorid
    WHERE a.permissionlevel IN (1,2,4)
),
