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
    fn comparison_keys<'a>(
        &'a self,
        names: &'a [String],
    ) -> BoxFuture<'a, Result<Vec<String>, TagError>> {
        Box::pin(async move {
            sqlx::query_scalar("SELECT lower(name) FROM unnest($1::text[]) WITH ORDINALITY AS input(name, position) ORDER BY position")
                .bind(names).fetch_all(&self.pool).await.map_err(storage)
        })
    }

    fn list_tags<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
    ) -> BoxFuture<'a, Result<Vec<Tag>, TagError>> {
        Box::pin(async move {
            let query = queries::visible_tags(false);
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor_id.value())
                .bind(ResourceType::Tag as i32)
                .bind(citadel_primitives::PermissionLevel::Read.accepted_database_levels())
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
        actor: ActorId,
        administrator: bool,
        id: Uuid,
        patch: &'a TagPatch,
    ) -> BoxFuture<'a, Result<(Tag, Vec<TaggableResourceType>), TagError>> {
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
            sqlx::query("UPDATE tags SET name=$2, normalizedname=lower($2), color=$3, updatedat=$4 WHERE id=$1")
                .bind(id)
                .bind(&tag.name)
                .bind(&tag.color)
                .bind(Utc::now())
                .execute(&mut *transaction)
                .await
                .map_err(database_error)?;
            let query = queries::visible_tags(true);
            let row = sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::Tag as i32)
                .bind(citadel_primitives::PermissionLevel::Read.accepted_database_levels())
                .bind(administrator)
                .bind(id)
                .fetch_optional(&mut *transaction)
                .await
                .map_err(storage)?
                .ok_or(TagError::NotFound)?;
            let tag = map_tag(row)?;
            let affected = queries::linked_types(&mut transaction, id).await?;
            transaction.commit().await.map_err(storage)?;
            Ok((tag, affected))
        })
    }

    fn delete_tag<'a>(
        &'a self,
        id: Uuid,
    ) -> BoxFuture<'a, Result<Vec<TaggableResourceType>, TagError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            sqlx::query("SELECT id FROM tags WHERE id=$1 FOR UPDATE")
                .bind(id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?
                .ok_or(TagError::NotFound)?;
            let affected = queries::linked_types(&mut tx, id).await?;
            sqlx::query("DELETE FROM tags WHERE id=$1")
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(database_error)?;
            tx.commit().await.map_err(storage)?;
            Ok(affected)
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
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            ensure_resource_exists_tx(&mut transaction, resource_type, resource_id).await?;
            validate_tag_ids(&mut transaction, tag_ids).await?;
            replace_resource_tags_tx(
                &mut transaction,
                actor_id,
                resource_type,
                resource_id,
                tag_ids,
            )
            .await?;
            let tags = load_resource_tags(&mut transaction, resource_type, resource_id).await?;
            transaction.commit().await.map_err(storage)?;
            Ok(tags)
        })
    }
}
