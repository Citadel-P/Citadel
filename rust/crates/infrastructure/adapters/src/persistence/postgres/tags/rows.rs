use super::*;
pub(super) fn map_tag(row: PgRow) -> Result<Tag, TagError> {
    Ok(Tag {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        normalized_name: row.try_get("normalizedname").map_err(storage)?,
        color: row.try_get("color").map_err(storage)?,
        created_by_actor_id: row.try_get("createdbyactorid").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
        updated_at: row.try_get("updatedat").map_err(storage)?,
        usage_count: row.try_get("usage_count").map_err(storage)?,
    })
}

pub(super) fn map_tag_summary(row: PgRow) -> Result<TagSummary, TagError> {
    Ok(TagSummary {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        color: row.try_get("color").map_err(storage)?,
    })
}
