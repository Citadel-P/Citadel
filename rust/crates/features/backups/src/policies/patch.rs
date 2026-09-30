use citadel_primitives::ActorId;
use serde_json::{Value, json};

use crate::BackupError;
use crate::BackupPolicy;
use crate::BackupPolicyConfiguration;

/// Apply the public merge-patch contract without allowing metadata or execution state
/// to be overwritten. The caller must compare row_version again when committing.
pub fn merge(
    current: &BackupPolicy,
    patch: &Value,
) -> Result<BackupPolicyConfiguration, BackupError> {
    let patch = patch
        .as_object()
        .ok_or_else(|| BackupError::Validation("Policy patch must be an object.".into()))?;
    let mut value = json!({
        "name":current.name,"description":current.description,"source":current.source,
        "backupRepositoryId":current.backup_repository_id,"enabled":current.enabled,
        "cron":current.cron,"timeZone":current.time_zone,"webhook":current.webhook,
        "keepLastSuccessful":current.keep_last_successful,"timeoutSeconds":current.timeout_seconds,
        "alertOnFailure":current.alert_on_failure,"runAsActorId":current.run_as_actor_id
    });
    for field in [
        "description",
        "source",
        "backupRepositoryId",
        "enabled",
        "cron",
        "timeZone",
        "webhook",
        "keepLastSuccessful",
        "timeoutSeconds",
        "alertOnFailure",
        "runAsActorId",
    ] {
        if let Some(proposed) = patch.get(field) {
            // Null scalar update fields retain their current values.
            if proposed.is_null()
                && !matches!(field, "description" | "cron" | "timeZone" | "webhook")
            {
                continue;
            }
            value[field] = proposed.clone();
        }
    }
    let mut input: BackupPolicyConfiguration =
        serde_path_to_error::deserialize(value).map_err(|error| {
            BackupError::Validation(format!("Invalid Backup Policy patch: {error}"))
        })?;
    input.description = input
        .description
        .take()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    if input
        .description
        .as_ref()
        .is_some_and(|s| s.chars().count() > 600)
    {
        return Err(BackupError::Validation(
            "Description cannot exceed 600 characters.".into(),
        ));
    }
    input.cron = normalize(input.cron);
    input.time_zone = normalize(input.time_zone);
    if ((patch.contains_key("cron") || patch.contains_key("timeZone"))
        && input.cron.is_some() != input.time_zone.is_some())
        || input.cron.as_ref().is_some_and(|s| s.len() > 128)
        || input.time_zone.as_ref().is_some_and(|s| s.len() > 128)
    {
        return Err(BackupError::Validation(
            "Cron and time zone must both be set or both be empty.".into(),
        ));
    }
    if let Some(webhook) = &mut input.webhook {
        let object = webhook
            .as_object_mut()
            .ok_or_else(|| BackupError::Validation("Webhook must be an object or null.".into()))?;
        for field in ["secret", "branchFilter"] {
            if let Some(value) = object.get_mut(field) {
                if !value.is_null() && !value.is_string() {
                    return Err(BackupError::Validation(
                        "Webhook settings are invalid.".into(),
                    ));
                }
                if let Some(text) = value.as_str() {
                    if text.chars().count() > 256 {
                        return Err(BackupError::Validation(
                            "Webhook settings cannot exceed 256 characters.".into(),
                        ));
                    }
                    *value = normalize(Some(text.to_owned()))
                        .map(Value::String)
                        .unwrap_or(Value::Null);
                }
            }
        }
        if object.get("enabled").is_some_and(|v| !v.is_boolean()) {
            return Err(BackupError::Validation(
                "Webhook enabled must be a boolean.".into(),
            ));
        }
        if object.get("enabled").and_then(Value::as_bool) == Some(true)
            && object.get("secret").and_then(Value::as_str).is_none()
        {
            return Err(BackupError::Validation(
                "An enabled Webhook requires a secret.".into(),
            ));
        }
    }
    input.validate(ActorId::new(current.run_as_actor_id))?;
    // Creation's legacy default must not turn an explicitly cleared schedule back on.
    if input.cron.is_none() {
        input.time_zone = None;
    }
    if input.run_as_actor_id.is_none_or(|id| id.is_nil()) {
        return Err(BackupError::Validation("Run-as Actor is required.".into()));
    }
    guard(current, &input)?;
    Ok(input)
}

pub fn guard(current: &BackupPolicy, input: &BackupPolicyConfiguration) -> Result<(), BackupError> {
    if current.control_state != "Idle" || current.current_run_id.is_some() {
        return Err(BackupError::Conflict(
            "Backup Policy cannot be changed while a run is active.".into(),
        ));
    }
    if current.first_successful_run_at.is_some()
        && (current.source != input.source
            || current.backup_repository_id != input.backup_repository_id)
    {
        return Err(BackupError::Validation(
            "Backup Policy source and repository cannot change after a successful run.".into(),
        ));
    }
    Ok(())
}

pub fn changes_paid_trigger(current: &BackupPolicy, input: &BackupPolicyConfiguration) -> bool {
    let webhook = |v: &Option<Value>| {
        v.as_ref()
            .and_then(|v| v.get("enabled"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    };
    input.enabled
        && ((!current.enabled && (input.cron.is_some() || webhook(&input.webhook)))
            || (input.cron.is_some()
                && (input.cron != current.cron || input.time_zone != current.time_zone))
            || (webhook(&input.webhook) && input.webhook != current.webhook))
}

pub fn changes_execution(patch: &Value) -> bool {
    [
        "source",
        "backupRepositoryId",
        "enabled",
        "cron",
        "timeZone",
        "webhook",
        "keepLastSuccessful",
        "timeoutSeconds",
        "alertOnFailure",
        "runAsActorId",
    ]
    .iter()
    .any(|field| patch.get(field).is_some())
}

fn normalize(value: Option<String>) -> Option<String> {
    value.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;
    fn policy() -> BackupPolicy {
        let now = chrono::Utc::now();
        BackupPolicy {
            tags: Vec::new(),
            latest_run: None,
            id: Uuid::now_v7(),
            name: "Backup".into(),
            normalized_name: "BACKUP".into(),
            description: Some("original".into()),
            source: json!({"$type":"CitadelSystem"}),
            backup_repository_id: Uuid::now_v7(),
            enabled: true,
            cron: None,
            time_zone: None,
            webhook: None,
            keep_last_successful: 2,
            timeout_seconds: 60,
            alert_on_failure: true,
            run_as_actor_id: Uuid::now_v7(),
            control_state: "Idle".into(),
            current_run_id: None,
            last_scheduled_run_at: None,
            first_successful_run_at: None,
            created_by_actor_id: Uuid::now_v7(),
            created_at: now,
            updated_at: now,
            archived_at: None,
            row_version: 1,
        }
    }
    #[test]
    fn patch_preserves_omitted_fields_and_ignores_state_and_name() {
        let current = policy();
        let input = merge(
            &current,
            &json!({"enabled":false,"name":"attack","controlState":"Processing"}),
        )
        .unwrap();
        assert_eq!(input.name, current.name);
        assert_eq!(input.source, current.source);
        assert_eq!(input.description, current.description);
        assert!(!input.enabled);
        assert_eq!(
            merge(&current, &json!({"description":null}))
                .unwrap()
                .description,
            None
        );
        assert_eq!(
            merge(&current, &json!({"keepLastSuccessful":null}))
                .unwrap()
                .keep_last_successful,
            Some(2)
        );
    }
    #[test]
    fn patch_blocks_active_runs_and_changes_to_successful_source() {
        let mut current = policy();
        current.control_state = "Processing".into();
        assert!(matches!(
            merge(&current, &json!({"enabled":false})),
            Err(BackupError::Conflict(_))
        ));
        current.control_state = "Idle".into();
        current.first_successful_run_at = Some(chrono::Utc::now());
        assert!(merge(&current, &json!({"backupRepositoryId":Uuid::now_v7()})).is_err());
        assert!(
            merge(
                &current,
                &json!({"source":{"$type":"Deployment","deploymentId":Uuid::now_v7()}})
            )
            .is_err()
        );
        assert!(merge(&current, &json!({"description":"ok"})).is_ok());
    }
    #[test]
    fn paid_trigger_gating_allows_disabled_and_unchanged_schedules() {
        let current = policy();
        let scheduled = merge(&current, &json!({"cron":"0 2 * * *","timeZone":"UTC"})).unwrap();
        assert!(changes_paid_trigger(&current, &scheduled));
        let disabled = merge(
            &current,
            &json!({"enabled":false,"cron":"0 2 * * *","timeZone":"UTC"}),
        )
        .unwrap();
        assert!(!changes_paid_trigger(&current, &disabled));
        assert!(!changes_paid_trigger(
            &current,
            &merge(&current, &json!({"description":"metadata"})).unwrap()
        ));
        assert!(merge(&current, &json!({"cron":"0 2 * * *"})).is_err());
        assert!(merge(&current, &json!({"webhook":{"enabled":true}})).is_err());
    }
}
