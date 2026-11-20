, role_levels AS (
    SELECT permission.permissionlevel
    FROM actor_scope scope
    JOIN actorroles assignment ON assignment.actorid = scope.actorid
    JOIN permissions permission ON permission.roleid = assignment.roleid
    WHERE permission.resourcetype = $2
)
