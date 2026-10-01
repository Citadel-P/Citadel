use super::{adoption_views::AdoptionIssue, spec::*, views::DeploymentView};
use crate::api::resources::common::*;
use chrono::Utc;
use citadel_primitives::AuthorizedResource;
use citadel_primitives::{AutoUpdateState, AutoUpdateStatus, ResourceControlState};
use serde_json::json;
use uuid::Uuid;

fn details() -> AuthorizedResource<citadel_deployments::Deployment> {
    AuthorizedResource {
        resource: citadel_deployments::Deployment {
            id: Uuid::now_v7(),
            name: "web".into(),
            description: None,
            platform_id: Uuid::now_v7(),

            status: citadel_deployments::DeploymentStatus::Healthy,
            control_state: citadel_primitives::ResourceControlState::Idle,
            row_version: 1,
            auto_update_state: None,
            spec: serde_json::from_value::<DeploymentSpec>(json!({
                "image": {"$type": "Local", "imageId": "sha256:local"}
            }))
            .unwrap(),

            audit: citadel_primitives::AuditMetadata {
                created_at: Utc::now(),
                created_by_actor_id: citadel_primitives::ActorId::new(Uuid::now_v7()),
            },

            platform_status: citadel_primitives::PlatformStatus::Online,
            platform_name: Some("docker".into()),
            image_name: None,
            image_id: None,
            container_id: None,
            docker_container_id: None,
            docker_image_id: None,
            tags: vec![],
            latest_activity: None,
        },
        effective_permission: citadel_primitives::EffectivePermission::Administrator,
    }
}

#[test]
fn native_statuses_preserve_wire_values_and_reject_invalid_stored_state() {
    for status in [
        "Unknown", "Created", "Pending", "Applying", "Healthy", "Degraded", "Failed", "Stopped",
    ] {
        let mut record = details();
        record.resource.status = status.parse().unwrap();
        for control in ["Idle", "Processing"] {
            record.resource.control_state = control.parse().unwrap();
            for platform in ["Offline", "Online"] {
                record.resource.platform_status = platform.parse().unwrap();
                let wire = serde_json::to_value(DeploymentView::try_from(record.clone()).unwrap())
                    .unwrap();
                assert_eq!(wire["status"], status);
                assert_eq!(wire["controlState"], control);
                assert_eq!(wire["platformStatus"], platform);
            }
        }
    }
    assert!(
        "Invalid"
            .parse::<citadel_deployments::DeploymentStatus>()
            .is_err()
    );
    assert!("Invalid".parse::<ResourceControlState>().is_err());
    assert!(
        "Invalid"
            .parse::<citadel_primitives::PlatformStatus>()
            .is_err()
    );
}

#[test]
fn update_checks_keep_digests_and_errors_across_serialization() {
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
        let json = serde_json::to_value(&state).unwrap();
        assert_eq!(json["status"], status.as_str());
        assert_eq!(
            serde_json::from_value::<AutoUpdateState>(json).unwrap(),
            state
        );
    }
}

#[test]
fn latest_activity_preserves_failures_and_normalizes_payload_keys() {
    let mut record = details();
    record.resource.latest_activity = Some(serde_json::from_value(json!({
        "id": Uuid::now_v7(), "resourceType": "Deployment", "eventType": "DeploymentApplied",
        "status": "Failure", "createdAt": Utc::now(),
        "info": {"$type": "DeploymentApplied", "Result": {"Message": "Container failed", "IsSuccess": false},
                 "Spec": {"Labels": {"Owner": "OPS"}}}
    })).unwrap());
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
    assert!(
        serde_json::from_value::<citadel_activities::ActivitySummary>(json!({"status": "Failure"}))
            .is_err()
    );
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
        "#/components/schemas/PublicActivityEventInfo"
    );
    assert_eq!(
        schemas["ActivityStatus"]["enum"],
        json!(["Success", "Failure", "Warning", "Information"])
    );
}

#[test]
fn deployments_and_swarm_services_publish_the_shared_auto_update_state() {
    let doc = serde_json::to_value(crate::openapi::document(false)).unwrap();
    let schemas = &doc["components"]["schemas"];
    for resource in ["DeploymentView", "ManagedSwarmServiceView"] {
        assert!(
            schemas[resource]["properties"]["autoUpdateState"]
                .to_string()
                .contains("#/components/schemas/AutoUpdateState")
        );
    }
    assert_eq!(
        schemas["AutoUpdateState"]["properties"]["status"]["$ref"],
        "#/components/schemas/AutoUpdateStatus"
    );
    assert!(schemas.get("deployments.model.AutoUpdateState").is_none());
    assert!(
        schemas
            .get("swarm_services.model.AutoUpdateState")
            .is_none()
    );
}
