pub(crate) const READ_MASK: i32 = 1 | 2 | 4;

pub(crate) const AUTHORIZED_CTE: &str = r#"
WITH actor_scope AS (
    SELECT actor.id AS actorid
    FROM actors actor
    WHERE actor.id = $1 AND actor.isenabled
    UNION
    SELECT team.actorid
    FROM actorteammemberships membership
    JOIN teams team ON team.id = membership.teamid
    JOIN actors team_actor ON team_actor.id = team.actorid AND team_actor.isenabled
    WHERE membership.memberactorid = $1
), global_access AS (
    SELECT EXISTS (
        SELECT 1 FROM actor_scope scope
        JOIN actorroles assignment ON assignment.actorid = scope.actorid
        JOIN permissions permission ON permission.roleid = assignment.roleid
        WHERE permission.resourcetype = $2
          AND (permission.permissionlevel & $3) <> 0
    ) AS allowed
)
"#;

pub(crate) fn authorized_catalog_query(
    table: &str,
    alias: &str,
    resource_type: citadel_primitives::ResourceType,
) -> String {
    let extra_columns = if resource_type == citadel_primitives::ResourceType::GitRepository {
        ", NULL::jsonb AS latest_activity"
    } else {
        ""
    };
    format!(
        r#"{AUTHORIZED_CTE}
SELECT resource.*, COALESCE((SELECT jsonb_agg(jsonb_build_object('id',tag.id,'name',tag.name,'color',tag.color) ORDER BY tag.name,tag.id)
 FROM resourcetags link JOIN tags tag ON tag.id=link.tagid WHERE link.resourcetype=$5 AND link.resourceid=resource.id),'[]'::jsonb) AS tags{extra_columns}
FROM {table} resource WHERE ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
 SELECT 1 FROM actor_scope scope JOIN resourceaccesses access ON access.actorid=scope.actorid
 WHERE access.resourcetype=$2 AND access.resourceid=resource.id AND (access.permissionlevel & $3)<>0))
ORDER BY resource.name,resource.id -- {alias}"#
    )
}
