use super::*;
use std::collections::BTreeMap;
use uuid::Uuid;

fn external_spec() -> DeploymentSpec {
    DeploymentSpec {
        image: DeploymentImageInfo::External {
            registry_id: Uuid::from_u128(0x100),
            image_tag: "nginx:latest".to_owned(),
            resolved_digest: Some("sha256:old".to_owned()),
        },
        update_behavior: UpdateBehavior::Notify,
        life_cycle_spec: None,
        resource_spec: None,
        labels: Some(BTreeMap::from([(
            "Mixed.Key".to_owned(),
            "value".to_owned(),
        )])),
        ports: Some(vec!["8080:80".to_owned()]),
        volumes: None,
        networks: None,
        command: None,
        environment_variables: None,
    }
}

#[test]
fn storage_contract_keeps_pascal_case_without_rewriting_map_keys() {
    let value = external_spec().to_storage_value().unwrap();
    assert_eq!(value["Image"]["$type"], "External");
    assert_eq!(value["Image"]["ImageTag"], "nginx:latest");
    assert_eq!(value["Labels"]["Mixed.Key"], "value");
    assert!(value.get("image").is_none());
    assert_eq!(
        DeploymentSpec::from_storage_value(value).unwrap(),
        external_spec()
    );
}

#[test]
fn create_removes_resolved_provenance() {
    let created = external_spec().for_create();
    assert!(matches!(
        created.image,
        DeploymentImageInfo::External {
            resolved_digest: None,
            ..
        }
    ));
}

#[test]
fn digest_pinned_external_image_rejects_auto_update() {
    let mut spec = external_spec();
    if let DeploymentImageInfo::External { image_tag, .. } = &mut spec.image {
        *image_tag = "nginx@sha256:abc".to_owned();
    }
    assert!(spec.validate().is_err());
}

#[test]
fn update_behavior_preserves_explicit_values_and_rejects_invalid_values() {
    for behavior in ["Disabled", "Notify", "AutoDeploy"] {
        let spec: DeploymentSpec = serde_json::from_value(serde_json::json!({
            "image":{"$type":"External","registryId":Uuid::from_u128(0x100),"imageTag":"nginx"},
            "updateBehavior":behavior
        }))
        .unwrap();
        assert_eq!(
            serde_json::to_value(spec).unwrap()["updateBehavior"],
            behavior
        );
    }
    for invalid in [
        serde_json::Value::Null,
        serde_json::json!("invalid"),
        serde_json::json!(123),
    ] {
        assert!(
            serde_json::from_value::<DeploymentSpec>(serde_json::json!({
                "image":{"$type":"Local","imageId":"sha256:test"},"updateBehavior":invalid
            }))
            .is_err()
        );
    }
    assert!(serde_json::from_value::<DeploymentSpec>(serde_json::json!({})).is_err());
}

#[test]
fn omitted_stop_timeout_is_valid() {
    let mut spec = external_spec();
    spec.update_behavior = UpdateBehavior::Disabled;

    assert!(spec.validate().is_ok());
}

#[test]
fn lifecycle_contract_defaults_an_omitted_restart_policy_to_no() {
    let value = serde_json::json!({
        "image": { "$type": "Local", "imageId": "image" },
        "updateBehavior": "Disabled",
        "lifeCycleSpec": { "stopSignal": "SIGKILL", "stopTimeout": 15 }
    });

    let spec: DeploymentSpec = serde_json::from_value(value).unwrap();

    assert_eq!(
        spec.life_cycle_spec.unwrap().restart_policy,
        ContainerRestartPolicy::No
    );
}
