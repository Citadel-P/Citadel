use crate::*;

fn input() -> BuildProjectConfiguration {
    BuildProjectConfiguration {
        name: " demo ".into(),
        description: None,
        enabled: true,
        git_repository_id: Uuid::now_v7(),
        branch: None,
        context_path: None,
        dockerfile_path: None,
        target: None,
        build_args: None,
        build_secrets: None,
        platform_id: Some(Uuid::now_v7()),
        push_to_registry: true,
        registry_id: Some(Uuid::now_v7()),
        image_repository: " team/demo ".into(),
        tag_templates: None,
        webhook: None,
        timeout_seconds: None,
        retention_run_count: None,
        tag_ids: vec![],
        builder_kind: "Platform".into(),
        build_agent_pool_id: None,
    }
}

#[test]
fn validates_and_normalizes_build_project() {
    let mut value = input();
    value.validate().unwrap();
    assert_eq!(value.name, "demo");
    assert_eq!(value.context_path.as_deref(), Some("."));
    assert_eq!(value.timeout_seconds, Some(1800));
}

#[test]
fn build_paths_reject_absolute_paths_and_repository_metadata() {
    for path in [
        "/tmp/context",
        r"C:\work\context",
        r"\\server\context",
        "../context",
        ".git/config",
    ] {
        assert!(normalize_path(Some(path.into()), ".").is_err(), "{path}");
    }
    assert_eq!(
        normalize_path(Some("services/api".into()), ".").unwrap(),
        "services/api"
    );
}

#[test]
fn build_webhook_uses_the_shared_authentication_validation() {
    let mut value = input();
    assert!(
        serde_json::from_value::<citadel_primitives::WebhookConfig>(
            serde_json::json!({"enabled":true,"provider":"Generic","authScheme":"None"})
        )
        .is_err()
    );
    value.webhook = Some(
        serde_json::from_value(serde_json::json!({"enabled":true,"provider":"Generic","authScheme":"BearerToken","secret":"test-shared-secret"})).unwrap(),
    );
    value.validate().unwrap();
}

#[test]
fn rejects_invalid_duplicate_or_unbound_build_secrets() {
    let mut invalid = input();
    invalid.build_secrets = Some(vec![BuildSecretSpec {
        id: "npm token".into(),
        secret_id: Uuid::now_v7(),
    }]);
    assert!(invalid.validate().is_err());
    let mut duplicate = input();
    duplicate.build_secrets = Some(vec![
        BuildSecretSpec {
            id: "npmrc".into(),
            secret_id: Uuid::now_v7(),
        },
        BuildSecretSpec {
            id: "NPMRC".into(),
            secret_id: Uuid::now_v7(),
        },
    ]);
    assert!(duplicate.validate().is_err());
    let mut unbound = input();
    unbound.build_secrets = Some(vec![BuildSecretSpec {
        id: "npmrc".into(),
        secret_id: Uuid::nil(),
    }]);
    assert!(unbound.validate().is_err());
}

#[test]
fn local_builds_do_not_require_or_retain_registry_configuration() {
    let mut value = input();
    value.push_to_registry = false;
    value.validate().unwrap();
    assert!(value.registry_id.is_none());
    assert!(value.image_repository.is_empty());
    value.validate().unwrap();
    value.push_to_registry = true;
    assert!(value.validate().is_err());
    value.registry_id = Some(Uuid::now_v7());
    assert!(value.validate().is_err());
    value.image_repository = "team/app".into();
    value.validate().unwrap();
}

#[test]
fn pushing_defaults_to_enabled_when_omitted() {
    let mut json = serde_json::to_value(input()).unwrap();
    json.as_object_mut().unwrap().remove("pushToRegistry");
    let mut value: BuildProjectConfiguration = serde_json::from_value(json).unwrap();
    assert!(value.push_to_registry);
    value.validate().unwrap();
}
