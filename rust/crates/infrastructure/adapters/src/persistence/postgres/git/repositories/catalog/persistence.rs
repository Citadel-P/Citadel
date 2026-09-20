use super::*;
use crate::persistence::postgres::activities::store::insert_activity as insert_typed_activity;
use citadel_activities::ActivityEvent;
use serde::Serialize;
pub(crate) fn apply_json_merge_patch(target: &mut Value, patch: &Value) {
    let Value::Object(patch) = patch else {
        *target = patch.clone();
        return;
    };
    if !target.is_object() {
        *target = Value::Object(serde_json::Map::new());
    }
    let target = target
        .as_object_mut()
        .expect("target was initialized as an object");
    for (key, value) in patch {
        if value.is_null() {
            target.remove(key);
        } else {
            apply_json_merge_patch(target.entry(key.clone()).or_insert(Value::Null), value);
        }
    }
}
pub(crate) fn unique_ids(ids: &[Uuid]) -> Vec<Uuid> {
    let mut ids = ids.to_vec();
    ids.sort_unstable();
    ids.dedup();
    ids
}
pub(crate) fn exactly_one(value: u64) -> Result<(), GitRepositoryError> {
    match value {
        1 => Ok(()),
        0 => Err(GitRepositoryError::NotFound),
        _ => Err(GitRepositoryError::Storage(
            "Mutation affected an unexpected number of rows.".into(),
        )),
    }
}
pub(crate) fn database_error(error: sqlx::Error) -> GitRepositoryError {
    if let sqlx::Error::Database(db) = &error {
        if db.code().as_deref() == Some("23505") {
            return GitRepositoryError::Conflict(
                "A resource with the same unique value already exists.".into(),
            );
        }
        if db.code().as_deref() == Some("23503") {
            return GitRepositoryError::Conflict("The resource is still in use.".into());
        }
    }
    storage(error)
}
pub(crate) fn storage(error: impl std::fmt::Display) -> GitRepositoryError {
    GitRepositoryError::Storage(error.to_string())
}
pub(crate) fn serialize_optional<T: Serialize>(
    value: &Option<T>,
) -> Result<Option<String>, GitRepositoryError> {
    value
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(storage)
}
pub(crate) fn deserialize_optional<T: serde::de::DeserializeOwned>(
    value: Option<String>,
) -> Result<Option<T>, GitRepositoryError> {
    value
        .map(|value| serde_json::from_str(&value))
        .transpose()
        .map_err(storage)
}
pub(crate) async fn insert_git_activity(
    tx: &mut Transaction<'_, Postgres>,
    actor: ActorId,
    id: Uuid,
    name: &str,
    info: ActivityEventInfo,
) -> Result<(), GitRepositoryError> {
    let event =
        ActivityEvent::new_git_repository_event(id, name.to_owned(), actor, info, Utc::now())
            .map_err(|error| GitRepositoryError::Storage(error.to_string()))?;
    insert_typed_activity(tx, &event)
        .await
        .map_err(|error| GitRepositoryError::Storage(error.to_string()))
}
pub(crate) fn mask_webhook(value: &Value) -> Value {
    let mut masked = value.clone();
    if let Some(object) = masked.as_object_mut() {
        mask_property(object, "Secret");
    }
    masked
}
pub(super) fn mask_property(object: &mut serde_json::Map<String, Value>, pascal_name: &str) {
    let camel_name = format!(
        "{}{}",
        pascal_name[..1].to_ascii_lowercase(),
        &pascal_name[1..]
    );
    for key in [pascal_name, camel_name.as_str()] {
        if let Some(value) = object.get_mut(key) {
            *value = if value.as_str().is_some_and(|value| !value.is_empty()) {
                Value::String("****************".to_owned())
            } else {
                Value::Null
            };
        }
    }
}
pub(crate) fn serialize_activity_value<T: Serialize>(
    value: Option<&T>,
) -> Result<Option<Value>, GitRepositoryError> {
    value.map(serde_json::to_value).transpose().map_err(storage)
}
