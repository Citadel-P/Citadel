, readable_types AS (
    SELECT p.resourcetype kind
    FROM actor_scope scope JOIN actorroles ar ON ar.actorid=scope.actorid
    JOIN permissions p ON p.roleid=ar.roleid
    WHERE p.permissionlevel IN (1,2,4)
), readable_resources AS (
    SELECT a.resourcetype kind,a.resourceid
    FROM actor_scope scope JOIN resourceaccesses a ON a.actorid=scope.actorid
    WHERE a.permissionlevel IN (1,2,4)
),
