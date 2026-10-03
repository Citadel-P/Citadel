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
    let mut spec = match object.get("spec").filter(|value| !value.is_null()) {
        Some(value) => serde_json::from_value(value.clone())
            .map_err(|e| BackupError::Validation(format!("Invalid Backup Repository spec: {e}")))?,
        None => current.spec.clone(),
    };
    spec.normalize();
    let mut input = BackupRepositoryConfiguration {
        name: current.name.clone(),
        description,
        spec,
        password_secret_id: current.password_secret_id,
    };
    input.validate()?;
    let mut previous = current.spec.clone();
    previous.normalize();
    if current.status == crate::BackupRepositoryStatus::Ready && input.spec != previous {
        return Err(BackupError::Validation(
            "Ready backup repository location cannot be changed.".into(),
        ));
    }
    Ok(input)
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
            spec: serde_json::from_value(spec).unwrap(),
            password_secret_id: Uuid::now_v7(),
            status: crate::BackupRepositoryStatus::Unknown,
            control_state: citadel_primitives::ResourceControlState::Idle,
            current_run_id: None,
            control_started_at: None,
            last_pruned_at: None,
            last_checked_at: None,

            updated_at: Utc::now(),
            archived_at: None,
            row_version: 0,
            audit: citadel_primitives::AuditMetadata {
                created_at: Utc::now(),
                created_by_actor_id: citadel_primitives::ActorId::new(Uuid::now_v7()),
            },
        }
    }

    #[test]
    fn ready_repository_rejects_changed_location_but_accepts_normalized_original() {
        let mut current = repository(
            json!({"$type":"FileSystem","location":"Core","path":"/backups","platformId":null}),
        );
        current.status = crate::BackupRepositoryStatus::Ready;
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
                "$type":"S3Compatible","endpoint":" https://s3.example.test ","bucket":" citadel ",
                "prefix":" /prod/backups/ ","region":" eu-west-1 ",
                "accessKeySecretId":Uuid::now_v7(),"secretKeySecretId":Uuid::now_v7()
            }}),
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(&changed.spec).unwrap()["endpoint"],
            "https://s3.example.test"
        );
        assert_eq!(
            serde_json::to_value(&changed.spec).unwrap()["bucket"],
            "citadel"
        );
        assert_eq!(
            serde_json::to_value(&changed.spec).unwrap()["prefix"],
            "prod/backups"
        );
        assert_eq!(
            serde_json::to_value(&changed.spec).unwrap()["region"],
            "eu-west-1"
        );
        assert_eq!(changed.description.as_deref(), Some("backups"));
        assert_eq!(changed.password_secret_id, current.password_secret_id);
    }
    #[test]
    fn ready_location_comparison_uses_configuration_fields_and_defaults() {
        let mut current = repository(
            json!({"$type":"FileSystem","location":"Core","path":"/backups","type":"FileSystem"}),
        );
        current.status = crate::BackupRepositoryStatus::Ready;
        assert!(apply(&current, &json!({"spec":{"$type":"FileSystem","location":"Core","platformId":null,"path":"/backups"}})).is_ok());
        assert!(
            apply(
                &current,
                &json!({"spec":{"$type":"FileSystem","location":"Core","path":"/other"}})
            )
            .is_err()
        );
        let key = Uuid::now_v7();
        current.spec = serde_json::from_value(json!({"$type":"S3Compatible","endpoint":"https://s3.example.test","bucket":"backups","accessKeySecretId":key,"secretKeySecretId":key})).unwrap();
        let spec = current.spec.clone();
        let mut spec = serde_json::to_value(spec).unwrap();
        assert!(apply(&current, &json!({"spec":spec})).is_ok());
        spec["bucket"] = "another-bucket".into();
        assert!(apply(&current, &json!({"spec":spec})).is_err());
    }
}
