, global_reads AS (
    SELECT DISTINCT p.resourcetype FROM actor_scope s
    JOIN actorroles ar ON ar.actorid=s.actorid
    JOIN permissions p ON p.roleid=ar.roleid WHERE p.permissionlevel=ANY($3)
), usage AS (
    SELECT link.tagid, count(*)::integer AS usage_count
    FROM visible_tags tag
    JOIN resourcetags link ON link.tagid=tag.id
    JOIN targets t ON t.kind=link.resourcetype AND t.id=link.resourceid
    WHERE $4 OR t.permission_type IN (SELECT resourcetype FROM global_reads) OR EXISTS (
        SELECT FROM actor_scope s JOIN resourceaccesses a ON a.actorid=s.actorid
        WHERE a.resourcetype=t.permission_type AND a.resourceid=t.id AND a.permissionlevel=ANY($3)
    )
    GROUP BY link.tagid
)
SELECT tag.*, coalesce(usage.usage_count, 0) AS usage_count
FROM visible_tags tag LEFT JOIN usage ON usage.tagid=tag.id ORDER BY tag.name, tag.id
