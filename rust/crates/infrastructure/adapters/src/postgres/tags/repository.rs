use super::*;
#[derive(Clone)]
pub struct PostgresTagRepository {
    pub(crate) pool: PgPool,
}
impl PostgresTagRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
impl TagRepository for PostgresTagRepository {
    fn list_tags<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
    ) -> BoxFuture<'a, Result<Vec<Tag>, TagError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT tag.*,
       (SELECT COUNT(*)::integer FROM resourcetags link WHERE link.tagid = tag.id) AS usage_count
FROM tags tag
WHERE ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
    SELECT 1 FROM actor_scope scope
    JOIN resourceaccesses access ON access.actorid = scope.actorid
    WHERE access.resourcetype = $2 AND access.resourceid = tag.id
      AND (access.permissionlevel & $3) <> 0
))
ORDER BY tag.name, tag.id"#
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor_id.value())
                .bind(ResourceType::Tag as i32)
                .bind(READ_MASK)
                .bind(administrator)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_tag)
                .collect()
        })
    }

    fn create_tag<'a>(
        &'a self,
        actor_id: ActorId,
        tag: &'a NewTag,
    ) -> BoxFuture<'a, Result<Tag, TagError>> {
        Box::pin(async move {
            let now = Utc::now();
            let row = sqlx::query(
                r#"
INSERT INTO tags (id, name, normalizedname, color, createdbyactorid, createdat, updatedat)
VALUES ($1, $2, lower($2), $3, $4, $5, $5)
RETURNING *, 0::integer AS usage_count
"#,
            )
            .bind(Uuid::now_v7())
            .bind(&tag.name)
            .bind(&tag.color)
            .bind(actor_id.value())
            .bind(now)
            .fetch_one(&self.pool)
            .await
            .map_err(database_error)?;
            map_tag(row)
        })
    }

    fn update_tag<'a>(
        &'a self,
        id: Uuid,
        patch: &'a TagPatch,
    ) -> BoxFuture<'a, Result<Tag, TagError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let current = sqlx::query("SELECT name,color FROM tags WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(storage)?
                .ok_or(TagError::NotFound)?;
            let mut tag = NewTag {
                name: patch
                    .name
                    .clone()
                    .unwrap_or(current.try_get("name").map_err(storage)?),
                color: patch
                    .color
                    .clone()
                    .unwrap_or(current.try_get("color").map_err(storage)?),
            };
            tag.validate()?;
            let row = sqlx::query(
                r#"
UPDATE tags
SET name = $2, normalizedname = lower($2), color = $3, updatedat = $4
WHERE id = $1
RETURNING *, (SELECT COUNT(*)::integer FROM resourcetags link WHERE link.tagid = tags.id) AS usage_count
"#,
            )
            .bind(id)
            .bind(&tag.name)
            .bind(&tag.color)
            .bind(Utc::now())
            .fetch_optional(&mut *transaction)
            .await
            .map_err(database_error)?
            .ok_or(TagError::NotFound)?;
            let tag = map_tag(row)?;
            transaction.commit().await.map_err(storage)?;
            Ok(tag)
        })
    }

    fn delete_tag<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<(), TagError>> {
        Box::pin(async move {
            let affected = sqlx::query("DELETE FROM tags WHERE id = $1")
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(database_error)?
                .rows_affected();
            exactly_one(affected)
        })
    }

    fn get_resource_tags<'a>(
        &'a self,
        resource_type: TaggableResourceType,
        resource_id: Uuid,
    ) -> BoxFuture<'a, Result<Vec<TagSummary>, TagError>> {
        Box::pin(async move {
            ensure_resource_exists(&self.pool, resource_type, resource_id).await?;
            sqlx::query(
                r#"
SELECT tag.id, tag.name, tag.color
FROM resourcetags link
JOIN tags tag ON tag.id = link.tagid
WHERE link.resourcetype = $1 AND link.resourceid = $2
ORDER BY tag.name, tag.id
"#,
            )
            .bind(resource_type.as_database_str())
            .bind(resource_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(map_tag_summary)
            .collect()
        })
    }

    fn replace_resource_tags<'a>(
        &'a self,
        actor_id: ActorId,
        resource_type: TaggableResourceType,
        resource_id: Uuid,
        tag_ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<TagSummary>, TagError>> {
        Box::pin(async move {
            let tag_ids = unique_ids(tag_ids);
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            ensure_resource_exists_tx(&mut transaction, resource_type, resource_id).await?;
            let count = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM tags WHERE id = ANY($1::uuid[])",
            )
            .bind(&tag_ids)
            .fetch_one(&mut *transaction)
            .await
            .map_err(storage)?;
            if count != tag_ids.len() as i64 {
                return Err(TagError::Validation(
                    "One or more Tags do not exist.".to_owned(),
                ));
            }
            sqlx::query("DELETE FROM resourcetags WHERE resourcetype = $1 AND resourceid = $2")
                .bind(resource_type.as_database_str())
                .bind(resource_id)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            if !tag_ids.is_empty() {
                sqlx::query(
                    r#"
INSERT INTO resourcetags (resourcetype, resourceid, tagid, createdbyactorid, createdat)
SELECT $1, $2, tagid, $4, $5 FROM unnest($3::uuid[]) AS tagid
"#,
                )
                .bind(resource_type.as_database_str())
                .bind(resource_id)
                .bind(&tag_ids)
                .bind(actor_id.value())
                .bind(Utc::now())
                .execute(&mut *transaction)
                .await
                .map_err(database_error)?;
            }
            let tags = load_resource_tags(&mut transaction, resource_type, resource_id).await?;
            transaction.commit().await.map_err(storage)?;
            Ok(tags)
        })
    }
}
