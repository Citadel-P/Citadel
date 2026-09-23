use serde::Deserialize;
use serde_json::Value;

use crate::AlertError;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameAlertRuleInput {
    pub id: uuid::Uuid,
    pub name: String,
}

impl RenameAlertRuleInput {
    pub fn validate(&mut self) -> Result<(), AlertError> {
        self.name = self.name.trim().to_owned();
        if self.id.is_nil() || self.name.is_empty() || self.name.chars().count() > 120 {
            return Err(AlertError::Validation(
                "A valid rule ID and a name of 1 to 120 characters are required.".into(),
            ));
        }
        Ok(())
    }
}

// Preserve merge-patch's distinction between an omitted description and null.
// Other metadata (including tags) is not persisted on Alert Rules.
pub fn description_patch(patch: &Value) -> Result<Option<Option<&str>>, AlertError> {
    let object = patch
        .as_object()
        .ok_or_else(|| AlertError::Validation("Metadata must be an object.".into()))?;
    match object.get("description") {
        None => Ok(None),
        Some(Value::Null) => Ok(Some(None)),
        Some(Value::String(value)) => Ok(Some(Some(value))),
        _ => Err(AlertError::Validation(
            "Description must be a string or null.".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn metadata_distinguishes_missing_null_and_value() {
        assert_eq!(description_patch(&json!({})).unwrap(), None);
        assert_eq!(
            description_patch(&json!({"description":null})).unwrap(),
            Some(None)
        );
        assert_eq!(
            description_patch(&json!({"description":"ops"})).unwrap(),
            Some(Some("ops"))
        );
        assert!(description_patch(&json!({"description":123})).is_err());
        assert!(description_patch(&json!([])).is_err());
    }

    #[test]
    fn rename_rejects_empty_id_empty_name_and_overlong_name() {
        for (id, name) in [
            (uuid::Uuid::nil(), "ops".into()),
            (uuid::Uuid::now_v7(), " ".into()),
            (uuid::Uuid::now_v7(), "a".repeat(121)),
        ] {
            assert!(RenameAlertRuleInput { id, name }.validate().is_err());
        }
    }
}
