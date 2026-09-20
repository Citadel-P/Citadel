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
        TaggableResourceType::AutomationAction => "actions",
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
    let ids = unique_ids(ids);
    let count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tags WHERE id=ANY($1::uuid[])")
        .bind(&ids)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?;
    if count != ids.len() as i64 {
        return Err(TagError::Validation(
            "One or more Tags do not exist.".into(),
        ));
    }
    Ok(())
}

pub(crate) async fn insert_resource_tags(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    t: TaggableResourceType,
    id: Uuid,
    ids: &[Uuid],
) -> Result<(), TagError> {
    let ids = unique_ids(ids);
    if ids.is_empty() {
        return Ok(());
    }
    sqlx::query("INSERT INTO resourcetags (resourcetype,resourceid,tagid,createdbyactorid,createdat) SELECT $1,$2,tagid,$4,$5 FROM unnest($3::uuid[]) tagid")
    .bind(t.as_database_str()).bind(id).bind(ids).bind(actor.value()).bind(Utc::now()).execute(&mut **tx).await.map_err(storage)?;
    Ok(())
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

pub(crate) fn exactly_one(value: u64) -> Result<(), TagError> {
    match value {
        1 => Ok(()),
        0 => Err(TagError::NotFound),
        _ => Err(TagError::Storage(
            "Mutation affected an unexpected number of rows.".into(),
        )),
    }
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
