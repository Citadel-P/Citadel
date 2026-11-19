pub(crate) const READ_MASK: i32 = 1 | 2 | 4;

pub(crate) const AUTHORIZED_CTE: &str = concat!(
    include_str!("catalog_role_levels.sql"),
    ", global_access AS (SELECT EXISTS (SELECT 1 FROM role_levels WHERE (permissionlevel & $3) <> 0) AS allowed)\n"
);

pub(crate) const AUTHORIZED_BY_LEVELS_CTE: &str = concat!(
    include_str!("catalog_role_levels.sql"),
    ", global_access AS (SELECT EXISTS (SELECT 1 FROM role_levels WHERE permissionlevel = ANY($3)) AS allowed)\n"
);

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
