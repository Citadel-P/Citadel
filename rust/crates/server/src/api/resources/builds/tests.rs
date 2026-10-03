use super::{patch::*, requests::*, spec::*};
use serde_json::json;

#[test]
fn project_patch_preserves_null_and_omission_and_rejects_invalid_types() {
    let empty: UpdateBuildProjectInput = serde_json::from_value(json!({})).unwrap();
    assert_eq!(serde_json::to_value(empty).unwrap(), json!({}));
    let null: UpdateBuildProjectInput = serde_json::from_value(
        json!({"webhook":null,"platformId":null,"target":null,"buildArgs":null,"registryId":null,"pushToRegistry":false}),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(null).unwrap(),
        json!({"webhook":null,"platformId":null,"target":null,"buildArgs":null,"registryId":null,"pushToRegistry":false})
    );
    for input in [
        json!({"builderKind":"Unknown"}),
        json!({"enabled":null}),
        json!({"pushToRegistry":null}),
        json!({"name":"rename"}),
        json!({"webhook":{"provider":"Unknown"}}),
    ] {
        assert!(serde_json::from_value::<UpdateBuildProjectInput>(input).is_err());
    }
}

#[test]
fn pool_patch_preserves_null_retention_semantics() {
    let patch: UpdateBuildAgentPoolInput = serde_json::from_value(
        json!({"description":null,"maxActiveBuilders":null,"providerSpec":null,"enabled":null}),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(patch).unwrap(),
        json!({"description":null,"maxActiveBuilders":null,"providerSpec":null,"enabled":null})
    );
    assert!(
        serde_json::from_value::<UpdateBuildAgentPoolInput>(
            json!({"providerSpec":{"$type":"Invalid"}})
        )
        .is_err()
    );
}

#[test]
fn provider_specs_preserve_fields_and_self_managed_defaults() {
    let spec: BuildAgentPoolProviderSpec =
        serde_json::from_value(json!({"$type":"SelfManagedVm","connectionMode":"EdgeAgent"}))
            .unwrap();
    let wire = serde_json::to_value(spec).unwrap();
    assert_eq!(wire["architecture"], "Amd64");
    assert_eq!(wire["maxWorkers"], 1);
    assert_eq!(wire["connectionMode"], "EdgeAgent");
    assert!(wire["endpoint"].is_null());
    let spec: BuildAgentPoolProviderSpec = serde_json::from_value(json!({"$type":"AwsEc2","region":"eu-west-1","tags":{"Owner":"OPS"},"securityGroupIds":["sg-1"]})).unwrap();
    let wire = serde_json::to_value(spec).unwrap();
    assert_eq!(wire["region"], "eu-west-1");
    assert_eq!(wire["tags"]["Owner"], "OPS");
    assert_eq!(wire["securityGroupIds"], json!(["sg-1"]));
    for field in [
        json!({"architecture":"Invalid"}),
        json!({"connectionMode":"Invalid"}),
    ] {
        let mut spec = json!({"$type":"SelfManagedVm"});
        spec.as_object_mut()
            .unwrap()
            .extend(field.as_object().unwrap().clone());
        assert!(serde_json::from_value::<BuildAgentPoolProviderSpec>(spec).is_err());
    }
}

#[test]
fn queue_and_response_triggers_cover_dependency_builds() {
    let input: QueueInput = serde_json::from_value(json!({"trigger":"Dependency"})).unwrap();
    assert_eq!(input.trigger.unwrap().as_str(), "Dependency");
    assert_eq!(
        serde_json::from_value::<BuildRunTrigger>(json!("Dependency")).unwrap(),
        BuildRunTrigger::Dependency
    );
    assert!(serde_json::from_value::<QueueInput>(json!({"trigger":"Schedule"})).is_err());
}

#[test]
fn builds_schemas_are_native_and_patch_webhooks_are_typed() {
    let doc = serde_json::to_value(crate::openapi::document(false)).unwrap();
    let schemas = &doc["components"]["schemas"];
    assert!(
        schemas["UpdateBuildProjectInput"]["properties"]["webhook"]
            .to_string()
            .contains("#/components/schemas/WebhookPatch")
    );
    assert!(
        schemas["WebhookPatch"]["required"]
            .as_array()
            .is_none_or(Vec::is_empty)
    );
    assert!(
        schemas["BuildRunTrigger"]["enum"]
            .as_array()
            .unwrap()
            .contains(&json!("Dependency"))
    );
}

#[test]
fn run_views_preserve_dependency_and_failure_details() {
    use super::views::BuildRunView;
    use uuid::Uuid;
    let run = citadel_builds::BuildRun {
        id: Uuid::new_v4(),
        build_project_id: Uuid::new_v4(),
        project_name_snapshot: "project".into(),
        git_repository_id: Uuid::new_v4(),
        git_repository_name_snapshot: "repo".into(),
        platform_snapshot: citadel_builds::BuildPlatformSnapshot {
            id: None,
            name: None,
            address: None,
            builder_kind: Some("BuildAgentPool".into()),
            build_agent_pool_id: Some(Uuid::new_v4()),
        },
        branch: "main".into(),
        resolved_commit_sha: None,
        context_path: ".".into(),
        dockerfile_path: "Dockerfile".into(),
        target: None,
        registry_id: Some(Uuid::new_v4()),
        registry_host: "registry.test".into(),
        image_repository: "app".into(),
        image_references: vec![],
        trigger: "Dependency".into(),
        status: citadel_builds::BuildRunStatus::Interrupted,
        image_digest: None,
        timeout_seconds: 600,
        queued_at: chrono::Utc::now(),
        started_at: None,
        completed_at: None,
        exit_code: None,
        error_code: Some("build.interrupted".into()),
        error_message: Some("Agent disconnected".into()),
        triggered_by_actor_id: Uuid::new_v4(),
    };
    let wire = serde_json::to_value(BuildRunView::try_from(run.clone()).unwrap()).unwrap();
    assert_eq!(wire["trigger"], "Dependency");
    assert_eq!(wire["status"], "Interrupted");
    assert_eq!(wire["errorMessage"], "Agent disconnected");
    assert_eq!(wire["platformSnapshot"]["builderKind"], "BuildAgentPool");
    assert!("Unknown".parse::<citadel_builds::BuildRunStatus>().is_err());
}
