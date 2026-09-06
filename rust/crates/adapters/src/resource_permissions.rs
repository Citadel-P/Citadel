use citadel_domain::{ActorId, ResourceType};
use sqlx::PgPool;
use std::collections::BTreeMap;
use uuid::Uuid;

/// Grants for a list response, without a database round trip for every row.
pub(crate) async fn for_resources(
    pool: &PgPool,
    actor: ActorId,
    kind: ResourceType,
    ids: &[Uuid],
) -> Result<BTreeMap<Uuid, i32>, sqlx::Error> {
    if ids.is_empty() {
        return Ok(BTreeMap::new());
    }
    let rows: Vec<(Uuid, i32)> = sqlx::query_as(
        r#"
WITH actor_scope AS (
    SELECT id AS actorid FROM actors WHERE id=$1 AND isenabled
    UNION
    SELECT team.actorid FROM actorteammemberships membership
    JOIN teams team ON team.id=membership.teamid
    JOIN actors team_actor ON team_actor.id=team.actorid AND team_actor.isenabled
    JOIN actors member ON member.id=membership.memberactorid AND member.isenabled
    WHERE membership.memberactorid=$1
)
SELECT ids.id,COALESCE(bit_or(grants.level),0)::int4
FROM unnest($3::uuid[]) ids(id)
LEFT JOIN LATERAL (
    SELECT p.permissionlevel AS level FROM actor_scope scope
    JOIN actorroles assignment ON assignment.actorid=scope.actorid
    JOIN permissions p ON p.roleid=assignment.roleid AND p.resourcetype=$2
    UNION ALL
    SELECT access.permissionlevel FROM actor_scope scope
    JOIN resourceaccesses access ON access.actorid=scope.actorid
    WHERE access.resourcetype=$2 AND access.resourceid=ids.id
) grants ON true GROUP BY ids.id
"#,
    )
    .bind(actor.value())
    .bind(kind as i32)
    .bind(ids)
    .fetch_all(pool)
    .await?;
    Ok(rows.into_iter().collect())
}
