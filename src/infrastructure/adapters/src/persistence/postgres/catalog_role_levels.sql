WITH enabled_actor AS (
    SELECT id FROM actors WHERE id = $1 AND isenabled
), actor_scope AS (
    SELECT id AS actorid FROM enabled_actor
    UNION
    SELECT team.actorid
    FROM enabled_actor member
    JOIN actorteammemberships membership ON membership.memberactorid = member.id
    JOIN teams team ON team.id = membership.teamid
    JOIN actors team_actor ON team_actor.id = team.actorid AND team_actor.isenabled
), role_levels AS (
    SELECT permission.permissionlevel
    FROM actor_scope scope
    JOIN actorroles assignment ON assignment.actorid = scope.actorid
    JOIN permissions permission ON permission.roleid = assignment.roleid
    WHERE permission.resourcetype = $2
)
