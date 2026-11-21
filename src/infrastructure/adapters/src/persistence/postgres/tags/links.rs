use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub(crate) async fn insert(
    transaction: &mut Transaction<'_, Postgres>,
    resource_type: &str,
    resource_id: Uuid,
    tag_ids: &[Uuid],
    actor_id: Uuid,
) -> Result<(), ResourceTagError> {
    validate(transaction, tag_ids).await?;
    insert_validated(transaction, resource_type, resource_id, tag_ids, actor_id)
        .await
        .map_err(ResourceTagError::Database)
}

pub(super) async fn validate(
    transaction: &mut Transaction<'_, Postgres>,
    tag_ids: &[Uuid],
) -> Result<(), ResourceTagError> {
    if tag_ids.is_empty() {
        return Ok(());
    }
    let ids = super::unique_ids(tag_ids);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM tags WHERE id=ANY($1::uuid[])")
        .bind(&ids)
        .fetch_one(&mut **transaction)
        .await
        .map_err(ResourceTagError::Database)?;
    if usize::try_from(count).ok() == Some(ids.len()) {
        Ok(())
    } else {
        Err(ResourceTagError::Missing)
    }
}

pub(super) async fn insert_validated(
    transaction: &mut Transaction<'_, Postgres>,
    resource_type: &str,
    resource_id: Uuid,
    tag_ids: &[Uuid],
    actor_id: Uuid,
) -> Result<(), sqlx::Error> {
    if tag_ids.is_empty() {
        return Ok(());
    }
    sqlx::query("INSERT INTO resourcetags(resourcetype,resourceid,tagid,createdbyactorid) SELECT $1,$2,id,$4 FROM unnest($3::uuid[]) id")
        .bind(resource_type).bind(resource_id).bind(super::unique_ids(tag_ids)).bind(actor_id)
        .execute(&mut **transaction).await?;
    Ok(())
}

pub(crate) enum ResourceTagError {
    Missing,
    Database(sqlx::Error),
}

pub(crate) async fn load(
    connection: &mut sqlx::PgConnection,
    kind: &str,
    ids: &[Uuid],
) -> Result<std::collections::HashMap<Uuid, Vec<citadel_tags::TagSummary>>, sqlx::Error> {
    use sqlx::Row;
    let mut tags = std::collections::HashMap::<Uuid, Vec<_>>::new();
    if ids.is_empty() {
        return Ok(tags);
    }
    let rows = sqlx::query("SELECT link.resourceid,tag.id,tag.name,tag.color FROM resourcetags link JOIN tags tag ON tag.id=link.tagid WHERE link.resourcetype=$1 AND link.resourceid=ANY($2) ORDER BY tag.name,tag.id").bind(kind).bind(ids).fetch_all(connection).await?;
    for row in rows {
        tags.entry(row.try_get("resourceid")?)
            .or_default()
            .push(citadel_tags::TagSummary {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                color: row.try_get("color")?,
            });
    }
    Ok(tags)
}
