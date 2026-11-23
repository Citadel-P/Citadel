/// The caller supplies the persisted workload type as a compile-time SQL literal.
macro_rules! workload_grants {
    ($kind:ident, $value:literal) => {{
        const _: () = assert!(citadel_primitives::ResourceType::$kind as i32 == $value);
        concat!(
            include_str!("../enabled_actor_scope.sql"),
            r#", global_permission AS (
            SELECT COALESCE(MAX(p.permissionlevel),0)::integer AS level_mask,
                   COALESCE(bit_or(p.specificpermissions),0)::integer AS specific_mask
            FROM actor_scope scope JOIN actorroles assignment ON assignment.actorid=scope.actorid
            JOIN permissions p ON p.roleid=assignment.roleid WHERE p.resourcetype="#,
            stringify!($value),
            r#"), resource_permissions AS (
            SELECT a.resourceid, COALESCE(MAX(a.permissionlevel),0)::integer AS level_mask,
                   COALESCE(bit_or(a.specificpermissions),0)::integer AS specific_mask
            FROM actor_scope scope JOIN resourceaccesses a ON a.actorid=scope.actorid
            WHERE a.resourcetype="#,
            stringify!($value),
            " GROUP BY a.resourceid)\n"
        )
    }};
}
pub(super) use workload_grants;

pub(super) fn workload_permission(
    administrator: bool,
    level: i32,
    specific: i32,
) -> Result<citadel_primitives::EffectivePermission, String> {
    use citadel_primitives::{EffectivePermission, PermissionLevel, SpecificPermissions};
    if administrator {
        return Ok(EffectivePermission::Administrator);
    }
    Ok(EffectivePermission::Granted {
        level: PermissionLevel::from_i32(level)
            .ok_or_else(|| format!("unknown persisted PermissionLevel value {level}"))?,
        specifics: SpecificPermissions::from_bits_retain(specific as u32),
    })
}

pub(super) const EFFECTIVE_RESOURCE_PERMISSION: &str = concat!(
    include_str!("enabled_actor_scope.sql"),
    r#", grants AS (
    SELECT p.permissionlevel, p.specificpermissions FROM actor_scope s
    JOIN actorroles ar ON ar.actorid=s.actorid JOIN permissions p ON p.roleid=ar.roleid
    WHERE p.resourcetype=$2
    UNION ALL
    SELECT a.permissionlevel, a.specificpermissions FROM actor_scope s
    JOIN resourceaccesses a ON a.actorid=s.actorid
    WHERE a.resourcetype=$2 AND a.resourceid=$3
)
SELECT COALESCE(MAX(permissionlevel),0)::integer, COALESCE(bit_or(specificpermissions),0)::integer
FROM grants"#
);

pub(crate) const AUTHORIZED_CTE: &str = concat!(
    include_str!("enabled_actor_scope.sql"),
    include_str!("catalog_role_levels.sql"),
    ", global_access AS (SELECT EXISTS (SELECT 1 FROM role_levels WHERE (permissionlevel & $3) <> 0) AS allowed)\n"
);

pub(crate) const AUTHORIZED_BY_LEVELS_CTE: &str = concat!(
    include_str!("enabled_actor_scope.sql"),
    include_str!("catalog_role_levels.sql"),
    ", global_access AS (SELECT EXISTS (SELECT 1 FROM role_levels WHERE permissionlevel = ANY($3)) AS allowed)\n"
);

/// Match every requested name on the single owner selected by the outer query ($1).
pub(super) const OWNER_TAG_FILTER: &str = r#"
 AND NOT EXISTS (
    SELECT FROM unnest($3::text[]) requested(name)
    WHERE NOT EXISTS (
        SELECT FROM resourcetags link JOIN tags t ON t.id=link.tagid
        WHERE link.resourceid=$1 AND link.resourcetype=$2 AND t.normalizedname=lower(requested.name)
    )
 )"#;

/// Compare against the complete requested set so an unknown name never broadens the filter.
pub(super) const ALL_REQUESTED_TAGS: &str = r#"
 AND resource.id IN (
     SELECT link.resourceid FROM unnest($6::text[]) WITH ORDINALITY requested(name,n)
     JOIN tags tag ON tag.normalizedname=lower(requested.name)
     JOIN resourcetags link ON link.tagid=tag.id AND link.resourcetype=$5
     GROUP BY link.resourceid HAVING count(*)=cardinality($6::text[])
 )"#;

pub(super) fn authorized_catalog_query(table: &str, extra_columns: &str, filter: &str) -> String {
    format!(
        r#"{AUTHORIZED_BY_LEVELS_CTE}
SELECT resource.*, COALESCE((SELECT jsonb_agg(jsonb_build_object('id',tag.id,'name',tag.name,'color',tag.color) ORDER BY tag.name,tag.id)
 FROM resourcetags link JOIN tags tag ON tag.id=link.tagid WHERE link.resourcetype=$5 AND link.resourceid=resource.id),'[]'::jsonb) AS tags{extra_columns}
FROM {table} resource WHERE ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
 SELECT 1 FROM actor_scope scope JOIN resourceaccesses access ON access.actorid=scope.actorid
 WHERE access.resourcetype=$2 AND access.resourceid=resource.id AND access.permissionlevel=ANY($3)))
{filter}
ORDER BY resource.name,resource.id"#
    )
}

pub(super) async fn tagged_workload_rows(
    pool: &sqlx::PgPool,
    actor: citadel_primitives::ActorId,
    administrator: bool,
    kind: citadel_tags::TaggableResourceType,
    source: &'static str,
    tags: &[String],
) -> Result<Vec<sqlx::postgres::PgRow>, sqlx::Error> {
    // PostgreSQL text cannot contain NUL, so no stored tag can match it.
    if tags.iter().any(|tag| tag.contains('\0')) {
        return Ok(Vec::new());
    }
    let filter = if tags.is_empty() {
        ""
    } else {
        ALL_REQUESTED_TAGS
    };
    let sql = format!(
        r#"{AUTHORIZED_BY_LEVELS_CTE}
SELECT resource.* FROM {source}
 AND ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
     SELECT 1 FROM actor_scope scope JOIN resourceaccesses access ON access.actorid=scope.actorid
     WHERE access.resourcetype=$2 AND access.resourceid=resource.id AND access.permissionlevel=ANY($3)))
{filter} ORDER BY resource.name,resource.id"#
    );
    let mut query = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(actor.value())
        .bind(kind.permission_type() as i32)
        .bind(citadel_primitives::PermissionLevel::Read.accepted_database_levels())
        .bind(administrator);
    if !tags.is_empty() {
        query = query.bind(kind.as_database_str()).bind(tags);
    }
    query.fetch_all(pool).await
}

/// Build and backup history inherit access from the project or policy that owns the run.
pub(super) fn authorized_run_history_query(
    table: &'static str,
    parent_column: &'static str,
    columns: &'static str,
    single_parent: bool,
) -> String {
    let parent = format!("run.{parent_column}");
    let access_target = if single_parent { "$4" } else { &parent };
    let can_read = format!(
        r#"$5 OR (SELECT allowed FROM global_access) OR EXISTS (
    SELECT 1 FROM actor_scope scope JOIN resourceaccesses access ON access.actorid=scope.actorid
    WHERE access.resourcetype=$2 AND access.resourceid={access_target}
      AND access.permissionlevel=ANY($3)
)"#
    );
    if single_parent {
        // An empty or singleton index key lets a generic prepared plan skip
        // denied history and read allowed runs in parent-index order.
        format!(
            r#"{AUTHORIZED_BY_LEVELS_CTE}
SELECT {columns} FROM {table} run
WHERE {parent}=ANY(ARRAY(SELECT $4::uuid WHERE {can_read}))
ORDER BY {parent},run.queuedat DESC,run.id DESC LIMIT $6"#
        )
    } else {
        format!(
            r#"{AUTHORIZED_BY_LEVELS_CTE}
SELECT {columns} FROM {table} run
WHERE $4::uuid IS NULL AND ({can_read})
ORDER BY run.queuedat DESC,run.id DESC LIMIT $6"#
        )
    }
}
