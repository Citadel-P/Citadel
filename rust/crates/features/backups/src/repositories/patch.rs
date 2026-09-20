use serde_json::Value;

use crate::BackupError;
use crate::BackupRepository;
use crate::BackupRepositoryConfiguration;

pub fn apply(
    current: &BackupRepository,
    patch: &Value,
) -> Result<BackupRepositoryConfiguration, BackupError> {
    let object = patch
        .as_object()
        .ok_or_else(|| BackupError::Validation("Repository patch must be an object.".into()))?;
    let description = if object.contains_key("description") {
        crate::policies::metadata::description_patch(patch)?.map(str::to_owned)
    } else {
        current.description.clone()
    };
    let mut spec = object
        .get("spec")
        .filter(|v| !v.is_null())
        .unwrap_or(&current.spec)
        .clone();
    normalize_spec(&mut spec);
    let mut input = BackupRepositoryConfiguration {
        name: current.name.clone(),
        description,
        spec,
        password_secret_id: current.password_secret_id,
    };
    input.validate()?;
    let mut previous = current.spec.clone();
    normalize_spec(&mut previous);
    if current.status == "Ready" && input.spec != previous {
        return Err(BackupError::Validation(
            "Ready backup repository location cannot be changed.".into(),
        ));
    }
    Ok(input)
}

fn normalize_spec(spec: &mut Value) {
    let kind = spec
        .get("$type")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    if let Some(fields) = spec.as_object_mut() {
        for key in match kind.as_str() {
            "FileSystem" => &["path"][..],
            "S3Compatible" => &["bucket", "prefix", "region"][..],
            _ => &[],
        } {
            if let Some(Value::String(value)) = fields.get_mut(*key) {
                *value = value.trim().to_owned();
                if *key == "prefix" {
                    *value = value.trim_matches('/').to_owned();
                }
            }
        }
        if kind == "S3Compatible" {
            for key in ["region", "prefix"] {
                if fields
                    .get(key)
                    .is_none_or(|v| v.as_str().is_some_and(str::is_empty))
                {
                    fields.insert(key.into(), Value::Null);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use serde_json::json;
    use uuid::Uuid;

    fn repository(spec: Value) -> BackupRepository {
        BackupRepository {
            id: Uuid::now_v7(),
            name: "repo".into(),
            normalized_name: "REPO".into(),
            description: Some("old".into()),
            repository_type: spec["$type"].as_str().unwrap().into(),
            spec,
            password_secret_id: Uuid::now_v7(),
            status: "Unknown".into(),
            control_state: "Idle".into(),
            current_run_id: None,
            control_started_at: None,
            last_pruned_at: None,
            last_checked_at: None,
            created_by_actor_id: Uuid::now_v7(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            archived_at: None,
            row_version: 0,
        }
    }

    #[test]
    fn ready_repository_rejects_changed_location_but_accepts_normalized_original() {
        let mut current = repository(
            json!({"$type":"FileSystem","location":"Core","path":"/backups","platformId":null}),
        );
        current.status = "Ready".into();
        assert!(apply(&current, &json!({"spec":{"$type":"FileSystem","location":"Core","path":"/other","platformId":null}})).is_err());
        let same = apply(&current, &json!({"spec":{"$type":"FileSystem","location":"Core","path":" /backups ","platformId":null}})).unwrap();
        assert_eq!(same.spec, current.spec);
        assert_eq!(same.description, current.description);
        let cleared = apply(&current, &json!({"description":null,"spec":null})).unwrap();
        assert_eq!(cleared.spec, current.spec);
        assert!(cleared.description.is_none());
    }

    #[test]
    fn s3_patch_normalizes_bucket_prefix_and_region_without_changing_password() {
        let current = repository(json!({"$type":"FileSystem","location":"Core","path":"/backups"}));
        let changed = apply(
            &current,
            &json!({"description":" backups ","passwordSecretId":Uuid::now_v7(),"spec":{
                "$type":"S3Compatible","endpoint":"https://s3.example.test","bucket":" citadel ",
                "prefix":" /prod/backups/ ","region":" eu-west-1 ",
                "accessKeySecretId":Uuid::now_v7(),"secretKeySecretId":Uuid::now_v7()
            }}),
        )
        .unwrap();
        assert_eq!(changed.spec["bucket"], "citadel");
        assert_eq!(changed.spec["prefix"], "prod/backups");
        assert_eq!(changed.spec["region"], "eu-west-1");
        assert_eq!(changed.description.as_deref(), Some("backups"));
        assert_eq!(changed.password_secret_id, current.password_secret_id);
    }
}
