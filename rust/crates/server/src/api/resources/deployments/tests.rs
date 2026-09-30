use super::{adoption_views::AdoptionIssue, spec::*, views::DeploymentView};
use crate::api::resources::{activities::views::LatestActivityView, common::*};
use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

fn details() -> citadel_deployments::DeploymentDetails {
    citadel_deployments::DeploymentDetails {
        deployment: citadel_deployments::Deployment {
            id: Uuid::now_v7(),
            name: "web".into(),
            description: None,
            platform_id: Uuid::now_v7(),
            created_at: Utc::now(),
            created_by_actor_id: Uuid::now_v7(),
            status: "Healthy".into(),
            control_state: "Idle".into(),
            row_version: 1,
            auto_update_state: None,
            spec: serde_json::from_value::<DeploymentSpec>(json!({
                "image": {"$type": "Local", "imageId": "sha256:local"}
            }))
            .unwrap()
            .into(),
        },
        platform_status: "Online".into(),
        platform_name: Some("docker".into()),
        image_name: None,
        image_id: None,
        container_id: None,
        docker_container_id: None,
        docker_image_id: None,
        tags: vec![],
        latest_activity: None,
        effective_permission: citadel_primitives::EffectivePermission::Administrator,
    }
}

#[test]
fn native_statuses_preserve_wire_values_and_reject_invalid_stored_state() {
    for status in [
        "Unknown", "Created", "Pending", "Applying", "Healthy", "Degraded", "Failed", "Stopped",
    ] {
        let mut record = details();
        record.deployment.status = status.into();
        for control in ["Idle", "Processing"] {
            record.deployment.control_state = control.into();
            for platform in ["Offline", "Online"] {
                record.platform_status = platform.into();
                let wire = serde_json::to_value(DeploymentView::try_from(record.clone()).unwrap())
                    .unwrap();
                assert_eq!(wire["status"], status);
                assert_eq!(wire["controlState"], control);
                assert_eq!(wire["platformStatus"], platform);
            }
        }
    }
    let mut record = details();
    record.deployment.status = "Invalid".into();
    assert!(DeploymentView::try_from(record).is_err());
    let mut record = details();
    record.deployment.control_state = "Invalid".into();
    assert!(DeploymentView::try_from(record).is_err());
    let mut record = details();
    record.platform_status = "Invalid".into();
    assert!(DeploymentView::try_from(record).is_err());
}

#[test]
fn update_checks_keep_digests_and_errors_across_native_conversion() {
    for status in [
        AutoUpdateStatus::Unknown,
        AutoUpdateStatus::UpToDate,
        AutoUpdateStatus::UpdateAvailable,
        AutoUpdateStatus::Updating,
        AutoUpdateStatus::Failed,
    ] {
        let state = AutoUpdateState {
            last_checked_at: Utc::now(),
            status,
            current_digest: Some("sha256:current".into()),
            remote_digest: Some("sha256:remote".into()),
            last_error: Some("Registry unavailable".into()),
        };
        let stored: citadel_deployments::AutoUpdateState = state.clone().into();
        assert_eq!(stored.status, status.as_str());
        assert_eq!(AutoUpdateState::try_from(stored).unwrap(), state);
    }
}

#[test]
fn latest_activity_preserves_failures_and_normalizes_payload_keys() {
    let mut record = details();
    record.latest_activity = Some(json!({
        "id": Uuid::now_v7(), "resourceType": "Deployment", "eventType": "DeploymentApplied",
        "status": "Failure", "createdAt": Utc::now(),
        "info": {"$type": "DeploymentApplied", "Result": {"Message": "Container failed", "IsSuccess": false},
                 "Spec": {"Labels": {"Owner": "OPS"}}}
    }));
    let wire = serde_json::to_value(DeploymentView::try_from(record).unwrap()).unwrap();
    assert_eq!(wire["latestActivityView"]["status"], "Failure");
    assert_eq!(
        wire["latestActivityView"]["info"]["result"]["message"],
        "Container failed"
    );
    assert_eq!(
        wire["latestActivityView"]["info"]["spec"]["labels"]["Owner"],
        "OPS"
    );
    assert!(LatestActivityView::from_stored(json!({"status": "Failure"})).is_err());
    assert!(
        serde_json::to_value(DeploymentView::try_from(details()).unwrap())
            .unwrap()
            .get("latestActivityView")
            .is_none()
    );
}

#[test]
fn duplicate_source_and_adoption_severity_are_typed() {
    let source =
        json!({"resourceId": Uuid::now_v7(), "resourceType": "Deployment", "resourceName": "web"});
    let wire: DuplicateSourceInput = serde_json::from_value(source.clone()).unwrap();
    let stored: citadel_deployments::DuplicateSource = wire.into();
    assert_eq!(
        serde_json::to_value(DuplicateSourceInput::try_from(stored).unwrap()).unwrap(),
        source
    );
    let mut invalid = source;
    invalid["resourceType"] = json!("NotAResource");
    assert!(serde_json::from_value::<DuplicateSourceInput>(invalid).is_err());
    for severity in ["Warning", "Blocker"] {
        let issue = citadel_deployments::adoption::AdoptionIssue {
            code: "unsupported".into(),
            message: "Review the configuration".into(),
            severity,
            field_path: None,
        };
        assert_eq!(
            serde_json::to_value(AdoptionIssue::try_from(issue).unwrap()).unwrap()["severity"],
            severity
        );
    }
}

#[test]
fn deployment_schemas_use_native_shared_contracts() {
    let doc = serde_json::to_value(crate::openapi::document(false)).unwrap();
    let schemas = &doc["components"]["schemas"];
    for (field, name) in [
        ("status", "DeploymentStatus"),
        ("controlState", "ResourceControlState"),
        ("platformStatus", "PlatformStatus"),
    ] {
        assert_eq!(
            schemas["DeploymentView"]["properties"][field]["$ref"],
            format!("#/components/schemas/{name}")
        );
    }
    assert!(
        schemas["DeploymentView"]["properties"]["latestActivityView"]
            .to_string()
            .contains("#/components/schemas/LatestActivityView")
    );
    assert_eq!(
        schemas["LatestActivityView"]["properties"]["status"]["$ref"],
        "#/components/schemas/ActivityStatus"
    );
    assert_eq!(
        schemas["LatestActivityView"]["properties"]["info"]["$ref"],
        "#/components/schemas/ActivityEventInfo"
    );
    assert_eq!(
        schemas["ActivityStatus"]["enum"],
        json!(["Success", "Failure", "Warning", "Information"])
    );
    let frozen = crate::openapi::compatibility::schemas();
    for name in [
        "DeploymentStatus",
        "ResourceControlState",
        "PlatformStatus",
        "AutoUpdateStatus",
        "LatestActivityView",
        "AdoptionIssueSeverity",
        "ActivityResourceType",
        "ActivityStatus",
    ] {
        assert!(!frozen.contains_key(name), "{name} is still frozen");
    }
}
