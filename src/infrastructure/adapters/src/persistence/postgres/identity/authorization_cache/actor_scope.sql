WITH actor_scope AS MATERIALIZED (
    SELECT COALESCE((
        SELECT array_prepend(actor.id, ARRAY(
            SELECT team.actorid
            FROM actorteammemberships membership
            JOIN teams team ON team.id = membership.teamid
            JOIN actors team_actor ON team_actor.id = team.actorid AND team_actor.isenabled
            WHERE membership.memberactorid = actor.id
        ))
        FROM actors actor WHERE actor.id = $1 AND actor.isenabled
    ), '{}'::uuid[]) AS actors
)
SELECT actors,
       ARRAY(
           SELECT ROW(p.resourcetype, p.permissionlevel, p.specificpermissions)
           FROM actorroles ar
           JOIN permissions p ON p.roleid = ar.roleid
           WHERE ar.actorid = ANY(s.actors)
       ) AS role_grants,
       -- Stop at the first matching entry in the (actorid, resourcetype) index.
       COALESCE((
           SELECT true FROM resourceaccesses a
           WHERE a.actorid = ANY(s.actors)
           ORDER BY a.actorid, a.resourcetype
           LIMIT 1
       ), false) AS has_resource_grants
FROM actor_scope s
