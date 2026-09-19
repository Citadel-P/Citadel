use citadel_domain::BackupPolicyActivitySnapshot;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::{BackupError, BackupPolicy};

#[derive(Debug, Deserialize)]
pub struct RenameBackupPolicyInput {
    pub id: Uuid,
    pub name: String,
}

impl RenameBackupPolicyInput {
    pub fn validate(&mut self) -> Result<(), BackupError> {
        self.name = self.name.trim().to_owned();
        if self.id.is_nil() || self.name.is_empty() || self.name.chars().count() > 128 {
            return Err(BackupError::Validation(
                "A valid Policy ID and a name of 1 to 128 characters are required.".into(),
            ));
        }
        Ok(())
    }
}

pub fn description_patch(patch: &Value) -> Result<Option<&str>, BackupError> {
    let object = patch
        .as_object()
        .ok_or_else(|| BackupError::Validation("Metadata must be an object.".into()))?;
    match object.get("description") {
        // The .NET metadata document is applied to an empty description.
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) if value.chars().count() <= 600 => {
            Ok((!value.trim().is_empty()).then_some(value.trim()))
        }
        _ => Err(BackupError::Validation(
            "Description must be a string of at most 600 characters or null.".into(),
        )),
    }
}

pub fn activity_snapshot(
    policy: &BackupPolicy,
) -> Result<BackupPolicyActivitySnapshot, BackupError> {
    let source = &policy.source;
    let field = |name: &str| {
        source
            .get(name)
            .and_then(Value::as_str)
            .ok_or_else(|| BackupError::Storage(format!("Backup source is missing {name}.")))
    };
    let source_type = field("$type")?;
    let source_key = match source_type {
        "CitadelSystem" => "citadel-system".into(),
        "Stack" => format!("stack:{}", field("stackId")?),
        "Deployment" => format!("deployment:{}", field("deploymentId")?),
        "SwarmService" => format!("swarm-service:{}", field("swarmServiceId")?),
        "DockerVolume" => {
            let platform = field("platformId")?;
            let volume = field("volumeName")?;
            match source
                .get("dockerNodeId")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|id| !id.is_empty())
            {
                Some(node) => format!("{platform}:{node}:{volume}"),
                None => format!("{platform}:{volume}"),
            }
        }
        _ => return Err(BackupError::Storage("Unknown Backup source type.".into())),
    };
    Ok(BackupPolicyActivitySnapshot {
        id: policy.id,
        name: policy.name.clone(),
        description: policy.description.clone(),
        source_type: source_type.into(),
        source_key,
        backup_repository_id: policy.backup_repository_id,
        enabled: policy.enabled,
        cron: policy.cron.clone(),
        time_zone: policy.time_zone.clone(),
        webhook_enabled: policy
            .webhook
            .as_ref()
            .and_then(|value| value.get("enabled"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        keep_last_successful: policy.keep_last_successful,
        timeout_seconds: policy.timeout_seconds,
        alert_on_failure: policy.alert_on_failure,
        run_as_actor_id: policy.run_as_actor_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn metadata_matches_empty_null_and_trimmed_description_contract() {
        for value in [
            json!({}),
            json!({"description":null}),
            json!({"description":"  "}),
        ] {
            assert_eq!(description_patch(&value).unwrap(), None);
        }
        assert_eq!(
            description_patch(&json!({"description":" ops "})).unwrap(),
            Some("ops")
        );
        for value in [
            json!([]),
            json!({"description":1}),
            json!({"description":"a".repeat(601)}),
        ] {
            assert!(description_patch(&value).is_err());
        }
    }

    #[test]
    fn rename_validates_id_and_name_before_storage() {
        for (id, name) in [
            (Uuid::nil(), "name".into()),
            (Uuid::now_v7(), " ".into()),
            (Uuid::now_v7(), "a".repeat(129)),
        ] {
            assert!(RenameBackupPolicyInput { id, name }.validate().is_err());
        }
        let mut input = RenameBackupPolicyInput {
            id: Uuid::now_v7(),
            name: " name ".into(),
        };
        input.validate().unwrap();
        assert_eq!(input.name, "name");
    }
}
