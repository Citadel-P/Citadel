, global_grants(permission) AS (
    SELECT DISTINCT p.resourcetype FROM actor_scope a JOIN actorroles ar USING(actorid)
    JOIN permissions p ON p.roleid=ar.roleid WHERE p.permissionlevel IN (1,2,4)
), grants(permission,id) AS (
    SELECT DISTINCT p.resourcetype,p.resourceid FROM actor_scope a JOIN resourceaccesses p USING(actorid)
    WHERE p.permissionlevel IN (1,2,4)
), tagged(kind,id) AS (
    SELECT link.resourcetype,link.resourceid FROM unnest($3::text[]) requested(name)
    JOIN tags t ON t.normalizedname=lower(requested.name) JOIN resourcetags link ON link.tagid=t.id
    GROUP BY link.resourcetype,link.resourceid HAVING count(*)=cardinality($3::text[])
), population(kind,permission,id,owner_id,platform_id,name,state,checked,digest) AS NOT MATERIALIZED (
