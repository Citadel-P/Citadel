use super::*;
pub(crate) const TAG_SUMMARIES: &str = r#"
COALESCE((
    SELECT jsonb_agg(jsonb_build_object('id', tag.id, 'name', tag.name, 'color', tag.color)
                     ORDER BY tag.name, tag.id)
    FROM resourcetags link
    JOIN tags tag ON tag.id = link.tagid
    WHERE link.resourcetype = $1 AND link.resourceid = resource.id
), '[]'::jsonb) AS tags
"#;

pub(super) async fn ensure_resource_exists(
    pool: &PgPool,
    t: TaggableResourceType,
    id: Uuid,
) -> Result<(), TagError> {
    let mut tx = pool.begin().await.map_err(storage)?;
    ensure_resource_exists_tx(&mut tx, t, id).await?;
    tx.rollback().await.map_err(storage)
}

pub(super) async fn ensure_resource_exists_tx(
    tx: &mut Transaction<'_, Postgres>,
    t: TaggableResourceType,
    id: Uuid,
) -> Result<(), TagError> {
    let table = match t {
        TaggableResourceType::Deployment => "deployments",
        TaggableResourceType::Stack => "stacks",
        TaggableResourceType::Platform => "platforms",
        TaggableResourceType::GitRepository => "gitrepositories",
        TaggableResourceType::Registry => "registries",
        TaggableResourceType::Script => "scripts",
        TaggableResourceType::BackupPolicy => "backuppolicies",
        TaggableResourceType::Build => "buildprojects",
        TaggableResourceType::BuildAgentPool => "buildagentpools",
        TaggableResourceType::SwarmService => "swarmservices",
    };
    let active = if matches!(
        t,
        TaggableResourceType::Build | TaggableResourceType::BuildAgentPool
    ) {
        " AND archivedat IS NULL"
    } else {
        ""
    };
    let q = format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE id=$1{active})");
    if !sqlx::query_scalar::<_, bool>(AssertSqlSafe(q.as_str()))
        .bind(id)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?
    {
        return Err(TagError::NotFound);
    }
    Ok(())
}

pub(super) async fn load_resource_tags(
    tx: &mut Transaction<'_, Postgres>,
    t: TaggableResourceType,
    id: Uuid,
) -> Result<Vec<TagSummary>, TagError> {
    sqlx::query("SELECT tag.id,tag.name,tag.color FROM resourcetags link JOIN tags tag ON tag.id=link.tagid WHERE link.resourcetype=$1 AND link.resourceid=$2 ORDER BY tag.name,tag.id")
    .bind(t.as_database_str()).bind(id).fetch_all(&mut **tx).await.map_err(storage)?.into_iter().map(map_tag_summary).collect()
}

pub(crate) async fn validate_tag_ids(
    tx: &mut Transaction<'_, Postgres>,
    ids: &[Uuid],
) -> Result<(), TagError> {
    super::links::validate(tx, ids)
        .await
        .map_err(|error| match error {
            super::links::ResourceTagError::Missing => {
                TagError::Validation("One or more Tags do not exist.".into())
            }
            super::links::ResourceTagError::Database(db) => storage(db),
        })
}

pub(crate) async fn insert_resource_tags(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    t: TaggableResourceType,
    id: Uuid,
    ids: &[Uuid],
) -> Result<(), TagError> {
    super::links::insert_validated(tx, t.as_database_str(), id, ids, actor.value())
        .await
        .map_err(database_error)
}

pub(crate) async fn replace_resource_tags_tx(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    resource_type: TaggableResourceType,
    resource_id: Uuid,
    ids: &[Uuid],
) -> Result<(), TagError> {
    sqlx::query("DELETE FROM resourcetags WHERE resourcetype=$1 AND resourceid=$2")
        .bind(resource_type.as_database_str())
        .bind(resource_id)
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
    insert_resource_tags(tx, actor, resource_type, resource_id, ids).await
}

pub(crate) fn unique_ids(ids: &[Uuid]) -> Vec<Uuid> {
    let mut ids = ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    ids
}

pub(crate) fn database_error(error: sqlx::Error) -> TagError {
    if let sqlx::Error::Database(db) = &error {
        if db.code().as_deref() == Some("23505") {
            return TagError::Conflict(
                "A resource with the same unique value already exists.".into(),
            );
        }
        if db.code().as_deref() == Some("23503") {
            return TagError::Conflict("The resource is still in use.".into());
        }
    }
    storage(error)
}

pub(crate) fn storage(error: impl std::fmt::Display) -> TagError {
    TagError::Storage(error.to_string())
}

pub(super) fn visible_tags(single: bool) -> String {
    let filter = if single { "AND tag.id=$5" } else { "" };
    format!("{AUTHORIZED_BY_LEVELS_CTE} {}, visible_tags AS MATERIALIZED (
        SELECT tag.* FROM tags tag WHERE ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
            SELECT FROM actor_scope scope JOIN resourceaccesses access ON access.actorid=scope.actorid
            WHERE access.resourcetype=$2 AND access.resourceid=tag.id AND access.permissionlevel=ANY($3)))
        {filter}
    ) {}",
        include_str!("usage_targets.sql"),
        include_str!("usage_counts.sql"))
}

pub(super) async fn linked_types(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<Vec<TaggableResourceType>, TagError> {
    let kinds = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT resourcetype FROM resourcetags WHERE tagid=$1",
    )
    .bind(id)
    .fetch_all(&mut **tx)
    .await
    .map_err(storage)?;
    kinds
        .into_iter()
        .map(|kind| serde_json::from_value(serde_json::Value::String(kind)).map_err(storage))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn usage_targets_match_each_taggable_resources_permission() {
        let expected: BTreeMap<_, _> = ResourceType::ALL
            .into_iter()
            .filter_map(|permission| {
                let kind: TaggableResourceType =
                    serde_json::from_value(serde_json::to_value(permission).unwrap()).ok()?;
                assert_eq!(kind.permission_type(), permission);
                Some((kind.as_database_str(), permission as i32))
            })
            .collect();
        let actual: BTreeMap<_, _> = include_str!("usage_targets.sql")
            .lines()
            .filter_map(|line| {
                let columns = line.trim().strip_prefix("SELECT ")?;
                let mut fields = columns.split(',');
                Some((
                    fields.next().unwrap().trim_matches('\''),
                    fields.next().unwrap().trim().parse::<i32>().unwrap(),
                ))
            })
            .collect();
        assert_eq!(actual, expected);
    }
}
