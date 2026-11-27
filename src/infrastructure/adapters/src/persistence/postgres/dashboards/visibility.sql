, dashboard_access AS (
    SELECT d.id, d.name, d.visibility, d.revision,
        CASE WHEN d.visibility='Private' THEN d.owneractorid=$1
             ELSE (SELECT administrator FROM dashboard_viewer) OR d.visibility='InstanceShared' OR grants.level >= 1 END AS can_read,
        CASE WHEN d.visibility='Private' THEN d.owneractorid=$1
             ELSE (SELECT administrator FROM dashboard_viewer) OR grants.level >= 2 END AS can_edit,
        CASE WHEN d.visibility='Private' THEN d.owneractorid=$1 ELSE (SELECT administrator FROM dashboard_viewer) END AS can_delete
    FROM dashboards d
    LEFT JOIN LATERAL (
        SELECT greatest(
            (SELECT max(p.permissionlevel) FROM actor_scope s JOIN actorroles ar ON ar.actorid=s.actorid
             JOIN permissions p ON p.roleid=ar.roleid WHERE p.resourcetype=22),
            (SELECT max(a.permissionlevel) FROM actor_scope s JOIN resourceaccesses a ON a.actorid=s.actorid
             WHERE a.resourcetype=22 AND a.resourceid=d.id), 0
        ) AS level
    ) grants ON true
    WHERE EXISTS (SELECT FROM enabled_actor)
)
