use super::{requests::*, spec::*};
use crate::api::resources::{bindings::spec::*, common::DuplicateSourceInput};
use serde_json::json;
use uuid::Uuid;

fn create_input() -> serde_json::Value {
    json!({"name": "copy", "platformId": Uuid::now_v7(), "stackSource": "WebEditor",
           "spec": {"$type": "WebEditor", "composeFile": "services: {}"},
           "duplicateSource": {"resourceType": "Stack", "resourceId": Uuid::now_v7(), "resourceName": "original"}})
}

#[test]
fn duplicate_source_uses_shared_typed_contract_and_rejects_other_resources() {
    let input = create_input();
    let wire: CreateStackInput = serde_json::from_value(input.clone()).unwrap();
    let domain = citadel_stacks::CreateStack::try_from(wire).unwrap();
    assert_eq!(domain.duplicate_source.as_ref().unwrap().name, "original");
    let roundtrip = serde_json::to_value(CreateStackInput::from(domain)).unwrap();
    assert_eq!(roundtrip["duplicateSource"], input["duplicateSource"]);
    let mut invalid = input.clone();
    invalid["duplicateSource"]["resourceType"] = json!("Deployment");
    let parsed: CreateStackInput = serde_json::from_value(invalid).unwrap();
    assert!(citadel_stacks::CreateStack::try_from(parsed).is_err());
    for id in [json!("invalid"), json!(42)] {
        let mut invalid = input.clone();
        invalid["duplicateSource"]["resourceId"] = id;
        assert!(serde_json::from_value::<CreateStackInput>(invalid).is_err());
    }
    assert!(
        serde_json::from_value::<DuplicateSourceInput>(json!({"resourceType": "Stack"})).is_err()
    );
}

#[test]
fn metadata_patch_distinguishes_omitted_null_and_value() {
    for (input, expected) in [
        (json!({}), None),
        (json!({"description": null}), Some(None)),
        (
            json!({"description": "updated"}),
            Some(Some("updated".into())),
        ),
    ] {
        let patch: PatchStackMetadataInput = serde_json::from_value(input).unwrap();
        assert_eq!(patch.description, expected);
    }
    for input in [
        json!({"description": 42}),
        json!(null),
        json!("description"),
    ] {
        assert!(serde_json::from_value::<PatchStackMetadataInput>(input).is_err());
    }
}

#[test]
fn binding_snapshot_uses_native_enums_and_preserves_secret_metadata() {
    let secret = Uuid::now_v7();
    let view = ResourceBindingSnapshot::try_from(citadel_stacks::ResourceBindingSnapshot {
        name: "TOKEN".into(),
        kind: "Secret".into(),
        scope: "Stack".into(),
        value: "[redacted]".into(),
        secret_id: Some(secret),
        secret_delivery_mode: Some("MountedFile".into()),
        target_path: Some("/run/secrets/token".into()),
    })
    .unwrap();
    assert_eq!(view.kind, ResourceBindingKind::Secret);
    assert_eq!(view.scope, ResourceBindingScope::Stack);
    assert_eq!(
        view.secret_delivery_mode,
        Some(SecretDeliveryMode::MountedFile)
    );
    let wire = serde_json::to_value(view).unwrap();
    assert_eq!(wire["secretId"], secret.to_string());
    assert_eq!(wire["targetPath"], "/run/secrets/token");
    assert!(
        ResourceBindingSnapshot::try_from(citadel_stacks::ResourceBindingSnapshot {
            name: "bad".into(),
            kind: "Unknown".into(),
            scope: "Stack".into(),
            value: "".into(),
            secret_id: None,
            secret_delivery_mode: None,
            target_path: None,
        })
        .is_err()
    );
}

#[test]
fn stack_schemas_describe_native_requests_streams_and_shared_types() {
    let doc = serde_json::to_value(crate::openapi::document(false)).unwrap();
    let schemas = &doc["components"]["schemas"];
    let stream = &schemas["StackStreamItem"];
    assert!(stream["properties"].get("progressMessage").is_some());
    assert_eq!(stream["required"], json!(["type"]));
    for operation in ["apply", "rollback"] {
        assert_eq!(
            doc["paths"][format!("/api/v1/stacks/{operation}")]["post"]["responses"]["200"]["content"]
                ["application/json"]["schema"]["items"]["$ref"],
            "#/components/schemas/StackStreamItem"
        );
    }
    for operation in ["start", "stop", "pause", "resume", "restart"] {
        let body = &doc["paths"][format!("/api/v1/stacks/{operation}")]["post"]["requestBody"]["content"]
            ["application/json"]["schema"];
        assert_eq!(body["type"], "array");
        assert_eq!(body["items"]["format"], "uuid");
    }
    for media in ["application/json", "application/merge-patch+json"] {
        assert_eq!(
            doc["paths"]["/api/v1/stacks/{id}/_metadata"]["patch"]["requestBody"]["content"][media]
                ["schema"]["$ref"],
            "#/components/schemas/PatchStackMetadataInput"
        );
    }
    let compatibility = crate::openapi::compatibility::schemas();
    for name in [
        "ActorType",
        "PlatformType",
        "ResourceBindingKind",
        "ResourceBindingScope",
        "SecretDeliveryMode",
        "DuplicateSourceInput",
        "StackIds",
        "StackStreamItems",
        "StackStreamItem",
        "StackApplyEventType",
    ] {
        assert!(!compatibility.contains_key(name), "{name} is still frozen");
    }
}
