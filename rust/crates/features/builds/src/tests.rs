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
        registry_id: Uuid::now_v7(),
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
    value.webhook =
        Some(serde_json::json!({"enabled":true,"provider":"Generic","authScheme":"None"}));
    assert!(value.validate().is_err());
    value.webhook = Some(
        serde_json::json!({"enabled":true,"provider":"Generic","authScheme":"BearerToken","secret":"test-shared-secret"}),
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
