use super::{patch::*, requests::*, spec::*};
use serde_json::json;

#[test]
fn backup_patch_preserves_omitted_and_null_values() {
    let empty: UpdateBackupPolicyInput = serde_json::from_value(json!({})).unwrap();
    assert_eq!(serde_json::to_value(empty).unwrap(), json!({}));
    let value = json!({"description":null,"source":null,"enabled":null,"cron":null,"timeZone":null,"webhook":null,"runAsActorId":null});
    let patch: UpdateBackupPolicyInput = serde_json::from_value(value.clone()).unwrap();
    assert_eq!(serde_json::to_value(patch).unwrap(), value);
    let patch: UpdateBackupRepositoryInput = serde_json::from_value(
        json!({"spec":null,"description":null,"passwordSecretId":"ignored"}),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(patch).unwrap(),
        json!({"spec":null,"description":null})
    );
}

#[test]
fn backup_patch_rejects_invalid_configuration_vocabulary() {
    for value in [
        json!({"source":{"$type":"Unknown"}}),
        json!({"webhook":{"provider":"Unknown"}}),
        json!({"enabled":"true"}),
    ] {
        assert!(serde_json::from_value::<UpdateBackupPolicyInput>(value).is_err());
    }
    assert!(
        serde_json::from_value::<UpdateBackupRepositoryInput>(
            json!({"spec":{"$type":"FileSystem","location":"Unknown","path":"/data"}})
        )
        .is_err()
    );
}

#[test]
fn backup_configuration_roundtrips_supported_variants() {
    let id = uuid::Uuid::now_v7();
    for value in [
        json!({"$type":"CitadelSystem"}),
        json!({"$type":"Deployment","deploymentId":id}),
        json!({"$type":"Stack","stackId":id}),
        json!({"$type":"SwarmService","swarmServiceId":id}),
        json!({"$type":"DockerVolume","platformId":id,"volumeName":"data","dockerNodeId":"node","consistency":"StopAttachedContainers"}),
    ] {
        let spec: BackupSourceSpec = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(spec).unwrap(), value);
    }
    let spec: BackupRepositorySpec = serde_json::from_value(json!({"$type":"S3Compatible","endpoint":"https://s3.example.test","bucket":"backups","accessKeySecretId":id,"secretKeySecretId":id})).unwrap();
    let wire = serde_json::to_value(spec).unwrap();
    assert_eq!(wire["bucketLookup"], "Auto");
    assert_eq!(wire["allowInsecureHttp"], false);
    assert!(wire.get("type").is_none());
}

#[test]
fn queue_contract_matches_accepted_triggers() {
    for trigger in ["Manual", "Schedule", "Webhook"] {
        let input: QueueInput = serde_json::from_value(json!({"trigger":trigger})).unwrap();
        assert_eq!(input.trigger.unwrap().as_str(), trigger);
    }
    assert!(serde_json::from_value::<QueueInput>(json!({"trigger":"Automation"})).is_err());
    assert!(serde_json::from_value::<BackupRunTrigger>(json!("Automation")).is_ok());
}

#[test]
fn repository_view_checks_persisted_configuration_and_status() {
    let id = uuid::Uuid::now_v7();
    let now = chrono::Utc::now();
    let mut repository = citadel_backups::BackupRepository {
        id,
        name: "backup".into(),
        normalized_name: "BACKUP".into(),
        description: None,
        repository_type: "FileSystem".into(),
        spec: serde_json::from_value(
            json!({"$type":"FileSystem","location":"Core","path":"/backups"}),
        )
        .unwrap(),
        password_secret_id: id,
        status: citadel_backups::BackupRepositoryStatus::Ready,
        control_state: citadel_primitives::ResourceControlState::Idle,
        current_run_id: None,
        control_started_at: None,
        last_pruned_at: None,
        last_checked_at: None,

        updated_at: now,
        archived_at: None,
        row_version: 1,

        audit: citadel_primitives::AuditMetadata {
            created_at: now,
            created_by_actor_id: citadel_primitives::ActorId::new(id),
        },
    };
    let view = super::views::BackupRepositoryView::try_from(repository.clone()).unwrap();
    let wire = serde_json::to_value(view).unwrap();
    assert_eq!(wire["status"], "Ready");
    assert_eq!(wire["spec"]["$type"], "FileSystem");
    assert!(wire["spec"].get("type").is_none());
    repository.status = citadel_backups::BackupRepositoryStatus::Unavailable;
    assert!(super::views::BackupRepositoryView::try_from(repository.clone()).is_ok());
    assert!(serde_json::from_value::<BackupRunItemStatus>(json!("Queued")).is_ok());
    assert!(
        "Unexpected"
            .parse::<citadel_backups::BackupRepositoryStatus>()
            .is_err()
    );
    repository.status = citadel_backups::BackupRepositoryStatus::Ready;
    let mut spec = serde_json::to_value(repository.spec).unwrap();
    spec["location"] = "Unexpected".into();
    assert!(serde_json::from_value::<BackupRepositorySpec>(spec).is_err());
}

#[test]
fn tagged_configuration_schema_uses_the_serialized_field_names() {
    use utoipa::PartialSchema;
    let source = serde_json::to_string(
        &crate::api::resources::schema_models::backups::BackupSourceSpecSchema::schema(),
    )
    .unwrap();
    for field in [
        "platformId",
        "volumeName",
        "dockerNodeId",
        "stackId",
        "deploymentId",
        "swarmServiceId",
    ] {
        assert!(
            source.contains(&format!("\"{field}\"")),
            "missing {field}: {source}"
        );
    }
    assert!(!source.contains("platform_id"));
    let repository = serde_json::to_string(
        &crate::api::resources::schema_models::backups::BackupRepositorySpecSchema::schema(),
    )
    .unwrap();
    for field in [
        "platformId",
        "accessKeySecretId",
        "secretKeySecretId",
        "bucketLookup",
        "allowInsecureHttp",
    ] {
        assert!(
            repository.contains(&format!("\"{field}\"")),
            "missing {field}: {repository}"
        );
    }
}
