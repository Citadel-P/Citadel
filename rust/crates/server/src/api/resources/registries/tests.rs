use super::{
    examples,
    requests::{NewRegistry, RegistryPatch},
    spec::{RegistryKind, RegistrySpec},
    views::RegistryView,
};
use serde_json::{Value, json};

#[test]
fn all_provider_examples_round_trip_native_settings() {
    for payload in [
        examples::custom(),
        examples::dockerhub(),
        examples::azure(),
        examples::aws(),
        examples::gitlab(),
        examples::github(),
    ] {
        let request: NewRegistry = serde_json::from_value(payload.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(&request.configuration).unwrap(),
            payload["configuration"]
        );
        let mut input: citadel_registries::NewRegistry = request.into();
        input.validate().unwrap();
    }
    // The built-in Docker Hub registry has no private credentials.
    let default: RegistrySpec = serde_json::from_value(json!({"$type":"DockerHub"})).unwrap();
    assert_eq!(
        serde_json::to_value(default).unwrap(),
        json!({"$type":"DockerHub"})
    );
    for invalid in [
        json!({"$type":"Unknown"}),
        json!({"$type":"Custom","authEnabled":"true"}),
    ] {
        assert!(serde_json::from_value::<RegistrySpec>(invalid).is_err());
    }
}

#[test]
fn credential_patch_preserves_missing_null_and_partial_values() {
    let patch: RegistryPatch =
        serde_json::from_value(json!({"configuration":{"password":"new-secret"}})).unwrap();
    let patch: citadel_registries::RegistryPatch = patch.into();
    let citadel_primitives::PatchField::Value(config) = patch.configuration else {
        panic!("expected partial configuration")
    };
    assert_eq!(config, json!({"password":"new-secret"}));
    let patch: RegistryPatch =
        serde_json::from_value(json!({"configuration":{"pat":null}})).unwrap();
    let patch: citadel_registries::RegistryPatch = patch.into();
    let citadel_primitives::PatchField::Value(config) = patch.configuration else {
        panic!("expected explicit null")
    };
    assert_eq!(config, json!({"pat":null}));
    let missing: citadel_registries::RegistryPatch =
        serde_json::from_value::<RegistryPatch>(json!({}))
            .unwrap()
            .into();
    assert!(matches!(
        missing.configuration,
        citadel_primitives::PatchField::Missing
    ));
    let null: citadel_registries::RegistryPatch =
        serde_json::from_value::<RegistryPatch>(json!({"configuration":null}))
            .unwrap()
            .into();
    assert!(matches!(
        null.configuration,
        citadel_primitives::PatchField::Null
    ));
    for invalid in [
        json!({"configuration":{"authEnabled":"true"}}),
        json!({"configuration":{"$type":"Unknown"}}),
    ] {
        assert!(serde_json::from_value::<RegistryPatch>(invalid).is_err());
    }
}

#[test]
fn public_registry_view_excludes_credentials_and_checks_kind() {
    let mut registry = citadel_registries::RegistryDetails {
        id: uuid::Uuid::now_v7(),
        audit: citadel_primitives::AuditMetadata {
            created_by_actor_id: citadel_primitives::ActorId::new(uuid::Uuid::now_v7()),
            created_at: chrono::Utc::now(),
        },
        name: "registry".into(),
        status: citadel_registries::RegistryStatus::Active,
        description: None,
        registry_host: "registry.example".into(),
        registry_type: "Custom".into(),
        configuration: json!({"$type":"Custom","password":"secret"}),
        tags: vec![],
    };
    let native = serde_json::to_value(&registry).unwrap();
    assert_eq!(
        native["createdByActorId"],
        json!(registry.audit.created_by_actor_id.value())
    );
    assert_eq!(native["createdAt"], json!(registry.audit.created_at));
    assert!(native.get("audit").is_none());
    assert!(native.get("configuration").is_none());
    let view = RegistryView::try_from(registry.clone()).unwrap();
    assert_eq!(view.registry_type, RegistryKind::Custom);
    let wire = serde_json::to_value(view).unwrap();
    assert_eq!(wire["type"], "Custom");
    assert_eq!(wire["createdByActorId"], native["createdByActorId"]);
    assert_eq!(wire["createdAt"], native["createdAt"]);
    assert!(wire.get("audit").is_none());
    assert!(wire.get("configuration").is_none());
    registry.registry_type = "Unknown".into();
    assert!(RegistryView::try_from(registry).is_err());
}

#[test]
fn registry_schema_uses_native_configuration_and_partial_updates() {
    let doc = serde_json::to_value(crate::openapi::document(false)).unwrap();
    let schemas = &doc["components"]["schemas"];
    assert_eq!(
        schemas["NewRegistry"]["properties"]["configuration"]["$ref"],
        "#/components/schemas/RegistrySpec"
    );
    assert_eq!(
        schemas["RegistryConfigResponse"]["properties"]["configuration"]["$ref"],
        "#/components/schemas/RegistrySpec"
    );
    assert_eq!(
        schemas["RegistryView"]["properties"]["type"]["$ref"],
        "#/components/schemas/RegistryKind"
    );
    assert!(
        !schemas["RegistryConfigurationPatch"]["required"]
            .as_array()
            .is_some_and(|v| v.contains(&Value::from("$type")))
    );
    assert!(
        schemas["RegistryPatch"]["properties"]["configuration"]
            .to_string()
            .contains("#/components/schemas/RegistryConfigurationPatch")
    );
}
