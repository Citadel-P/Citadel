use super::*;
use serde::Serialize;
use serde_json::{Value, json};
use utoipa::PartialSchema;
use uuid::Uuid;

fn assert_wire_matches<T: Serialize, S: Serialize + From<T>>(value: T) -> Value {
    let expected = serde_json::to_value(&value).unwrap();
    assert_eq!(serde_json::to_value(S::from(value)).unwrap(), expected);
    expected
}

#[test]
fn deployment_description_preserves_defaults_and_build_provenance() {
    for image in [
        json!({"$type":"Local", "imageId":"sha256:local"}),
        json!({"$type":"External", "registryId":Uuid::nil(), "imageTag":"nginx:latest"}),
        json!({"$type":"Build", "buildProjectId":Uuid::nil(), "redeployOnBuild":true,
            "resolvedDigest":"sha256:new", "appliedDigest":"sha256:old",
            "appliedAt":"2026-09-30T12:00:00Z"}),
    ] {
        let native: citadel_deployments::DeploymentSpec =
            serde_json::from_value(json!({"image":image})).unwrap();
        let wire = assert_wire_matches::<_, deployments::DeploymentSpecSchema>(native);
        assert_eq!(wire["updateBehavior"], "Disabled");
        assert!(wire.get("resourceSpec").is_none());
    }
}

#[test]
fn webhook_description_preserves_missing_null_and_supplied_patch_fields() {
    for input in [
        json!({}),
        json!({"secret":null}),
        json!({"enabled":false,"secret":"new"}),
    ] {
        let native: citadel_primitives::WebhookPatch =
            serde_json::from_value(input.clone()).unwrap();
        assert_eq!(
            assert_wire_matches::<_, primitives::WebhookPatchSchema>(native),
            input
        );
    }
    assert_wire_matches::<_, primitives::WebhookConfigSchema>(
        citadel_primitives::WebhookConfig::default(),
    );
}

#[test]
fn stack_webhook_description_keeps_shared_configuration_flat() {
    let native: citadel_stacks::StackWebhookConfig = serde_json::from_value(json!({
        "enabled":true, "provider":"GitHub", "authScheme":"GitHubHmacSha256",
        "secret":"credential", "forceDeploy":true
    }))
    .unwrap();
    let wire = assert_wire_matches::<_, stacks::StackWebhookConfigSchema>(native);
    assert_eq!(wire["forceDeploy"], true);
    assert_eq!(wire["provider"], "GitHub");
    assert!(wire.get("config").is_none());
}

#[test]
fn backup_source_description_keeps_discriminators_and_defaults() {
    for input in [
        json!({"$type":"CitadelSystem"}),
        json!({"$type":"DockerVolume","platformId":Uuid::nil(),"volumeName":"data"}),
        json!({"$type":"Stack","stackId":Uuid::nil()}),
        json!({"$type":"Deployment","deploymentId":Uuid::nil()}),
        json!({"$type":"SwarmService","swarmServiceId":Uuid::nil()}),
    ] {
        let native: citadel_backups::spec::BackupSourceSpec =
            serde_json::from_value(input.clone()).unwrap();
        let wire = assert_wire_matches::<_, backups::BackupSourceSpecSchema>(native);
        assert_eq!(wire["$type"], input["$type"]);
    }
}

#[test]
fn activity_description_preserves_native_payload_and_private_change_fields() {
    assert_wire_matches::<_, activities::ActivityEventInfoSchema>(
        citadel_activities::ActivityEventInfo::UserSessionRevoked {
            session_id: Uuid::nil(),
        },
    );
    let change =
        citadel_activities::ActivityChangedField::display_name("before".into(), "after".into());
    let wire = serde_json::to_value(change).unwrap();
    let schema = serde_json::to_value(activities::ActivityChangedFieldSchema::schema()).unwrap();
    for field in wire.as_object().unwrap().keys() {
        assert!(schema["properties"].get(field).is_some(), "missing {field}");
    }
}

#[test]
fn registry_browser_schema_matches_public_casing_not_upstream_casing() {
    let repository = citadel_registries::registry_images::DockerHubRepositoryInfo::default();
    let wire = assert_wire_matches::<_, registries::DockerHubRepositoryInfoSchema>(repository);
    let schema = serde_json::to_value(registries::DockerHubRepositoryInfoSchema::schema()).unwrap();
    for field in wire.as_object().unwrap().keys() {
        assert!(schema["properties"].get(field).is_some(), "missing {field}");
    }
    let version: citadel_registries::registry_images::GithubPackageVersion =
        serde_json::from_value(json!({"id":1,"name":"v1","html_url":"https://example.test"}))
            .unwrap();
    let wire = assert_wire_matches::<_, registries::GithubPackageVersionSchema>(version);
    let schema = serde_json::to_value(registries::GithubPackageVersionSchema::schema()).unwrap();
    for field in wire.as_object().unwrap().keys() {
        assert!(schema["properties"].get(field).is_some(), "missing {field}");
    }
    assert!(wire.get("htmlUrl").is_some());
    assert!(wire.get("html_url").is_none());
}

#[test]
fn job_category_schema_uses_the_shared_serialized_vocabulary() {
    let document = serde_json::to_value(crate::openapi::document(false)).unwrap();
    let schemas = &document["components"]["schemas"];
    assert_eq!(
        schemas["JobCategory"]["enum"],
        json!(citadel_primitives::JobCategory::ALL)
    );
    assert_eq!(
        schemas["JobStatusView"]["properties"]["resourceType"]["$ref"],
        "#/components/schemas/JobCategory"
    );
    assert!(schemas.get("JobResourceTypeView").is_none());
}

#[test]
fn shared_categories_survive_runtime_feature_and_http_boundaries() {
    use citadel_instance::InstanceJobsReader;
    use citadel_primitives::JobCategory;
    use citadel_runtime::jobs::{JobCatalog, JobOwnership};
    let catalog = JobCatalog::new(Uuid::now_v7());
    for category in JobCategory::ALL {
        catalog.register(category.as_str(), *category, JobOwnership::Coordinator);
    }
    let reader = citadel_adapters::external::instance_jobs::RuntimeInstanceJobs(catalog);
    let snapshot = reader.snapshot();
    for job in &snapshot.jobs {
        assert_eq!(job.resource_type.as_str(), job.id);
    }
    let view = crate::api::resources::instance::jobs::JobsSnapshotView::from(snapshot);
    let wire = serde_json::to_value(view).unwrap();
    for job in wire["jobs"].as_array().unwrap() {
        assert_eq!(job["resourceType"], job["id"]);
    }

    let catalog = JobCatalog::new(Uuid::now_v7());
    catalog.register(
        "custom-worker",
        "custom-category",
        JobOwnership::Coordinator,
    );
    let reader = citadel_adapters::external::instance_jobs::RuntimeInstanceJobs(catalog);
    assert_eq!(reader.snapshot().jobs[0].resource_type, JobCategory::Other);
}

#[test]
fn dashboard_widget_schemas_preserve_source_identity_and_field_names() {
    use citadel_dashboards::{MetricSource, WidgetSettings};
    let source = json!({"kind":"Container","id":Uuid::now_v7(),"platformId":Uuid::now_v7()});
    let source_native: MetricSource = serde_json::from_value(source.clone()).unwrap();
    assert_eq!(
        assert_wire_matches::<_, dashboards::DashboardMetricSourceSchema>(source_native),
        source
    );
    for settings in [
        json!({"kind":"Workloads","resourceKind":"SwarmService","limit":5}),
        json!({"kind":"Metrics","source":source,"metric":"Cpu","presentation":"Area","color":"Blue","legend":false}),
    ] {
        let native: WidgetSettings = serde_json::from_value(settings.clone()).unwrap();
        assert_eq!(
            assert_wire_matches::<_, dashboards::DashboardWidgetSettingsSchema>(native),
            settings
        );
    }
}
