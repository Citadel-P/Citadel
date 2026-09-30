use super::{spec::*, views::*};
use chrono::Utc;
use citadel_primitives::{WebhookAuthScheme, WebhookProvider};
use serde_json::json;
use uuid::Uuid;

fn spec() -> SwarmServiceSpec {
    serde_json::from_value(json!({
        "image": {"$type": "External", "registryId": Uuid::now_v7(), "imageTag": "redis:7"},
        "replicas": 2, "command": ["redis-server"], "environment": ["MODE=prod"],
        "labels": {"Owner": "OPS"}, "networkIds": ["network"],
        "webhook": {"enabled": true, "provider": "GitLab", "authScheme": "GitLabSignedToken", "secret": "test-secret", "branchFilter": "main"}
    })).unwrap()
}

#[test]
fn spec_roundtrip_preserves_configuration_and_native_webhook_vocabulary() {
    let wire = spec();
    let domain: citadel_swarm_services::SwarmServiceSpec = wire.clone().into();
    assert_eq!(
        domain.webhook.as_ref().unwrap().provider,
        WebhookProvider::GitLab
    );
    assert_eq!(SwarmServiceSpec::try_from(domain).unwrap(), wire);
    let defaults: citadel_primitives::WebhookConfig = serde_json::from_value(json!({})).unwrap();
    assert_eq!(defaults.provider, WebhookProvider::GitHub);
    assert_eq!(defaults.auth_scheme, WebhookAuthScheme::GitHubHmacSha256);
    assert!(
        serde_json::from_value::<citadel_primitives::WebhookConfig>(json!({"provider": "Invalid"}))
            .is_err()
    );
    assert!(
        serde_json::from_value::<citadel_primitives::WebhookConfig>(
            json!({"authScheme": "Invalid"})
        )
        .is_err()
    );
}

#[test]
fn operation_conversion_preserves_failures_and_rejects_unknown_states() {
    let operation = citadel_swarm_services::SwarmServiceOperation {
        id: Uuid::now_v7(),
        kind: citadel_swarm_services::SwarmServiceOperationKind::Delete,
        state: citadel_swarm_services::SwarmServiceOperationState::Rejected,
        prepared_at: Utc::now(),
        attempted_at: None,
        completed_at: None,
        result_code: Some("DockerRejected".into()),
        warnings: vec!["Review the service".into()],
        result_message: Some("Docker rejected deletion".into()),
    };
    let wire =
        serde_json::to_value(SwarmServiceOperationView::try_from(operation.clone()).unwrap())
            .unwrap();
    assert_eq!(wire["kind"], "Delete");
    assert_eq!(wire["state"], "Rejected");
    assert_eq!(wire["resultMessage"], "Docker rejected deletion");
    assert!(
        "Invalid"
            .parse::<citadel_swarm_services::SwarmServiceOperationState>()
            .is_err()
    );
}

#[test]
fn draft_views_preserve_warnings_and_adoption_data() {
    let source_id = Uuid::now_v7();
    let draft = citadel_swarm_services::SwarmServiceDuplicateDraft {
        name: "copy".into(),
        source_name: "source".into(),
        platform_id: Uuid::now_v7(),
        description: None,
        spec: spec().into(),
        tag_ids: vec![],
        warnings: vec!["Review published ports".into()],
    };
    let wire =
        serde_json::to_value(SwarmServiceDuplicateDraftView::from_draft(draft, source_id).unwrap())
            .unwrap();
    assert_eq!(wire["warnings"], json!(["Review published ports"]));
    assert_eq!(
        wire["draft"]["duplicateSource"]["resourceType"],
        "SwarmService"
    );
    assert_eq!(
        wire["draft"]["duplicateSource"]["resourceId"],
        source_id.to_string()
    );
    let adoption = citadel_swarm_services::adoption::SwarmServiceAdoptionDraft {
        source: citadel_swarm_services::adoption::SwarmServiceAdoptionSource {
            docker_service_id: "docker-service".into(),
            name: "external".into(),
            platform_id: Uuid::now_v7(),
            platform_name: "swarm".into(),
        },
        name: "external".into(),
        description: None,
        spec: spec().into(),
        issues: vec![
            citadel_swarm_services::adoption::SwarmServiceAdoptionIssue {
                code: "registry".into(),
                message: "Select registry".into(),
            },
        ],
        preview_fingerprint: "fingerprint".into(),
    };
    let wire =
        serde_json::to_value(SwarmServiceAdoptionDraftView::try_from(adoption).unwrap()).unwrap();
    assert_eq!(wire["draft"]["platformId"], wire["source"]["platformId"]);
    assert_eq!(wire["draft"]["tagIds"], json!([]));
    assert_eq!(wire["issues"][0]["message"], "Select registry");
    assert_eq!(wire["previewFingerprint"], "fingerprint");
}

#[test]
fn swarm_contracts_are_native_and_describe_real_progress_and_warning_shapes() {
    let doc = serde_json::to_value(crate::openapi::document(false)).unwrap();
    let schemas = &doc["components"]["schemas"];
    assert_eq!(
        schemas["SwarmServiceDuplicateDraftView"]["properties"]["warnings"]["items"]["type"],
        "string"
    );
    assert!(
        schemas["CreateSwarmServiceInput"]["properties"]["duplicateSource"]
            .to_string()
            .contains("#/components/schemas/DuplicateSourceInput")
    );
    for (field, kind) in [
        ("health", "SwarmServiceHealth"),
        ("synchronizationState", "SwarmServiceSynchronizationState"),
        ("controlState", "ResourceControlState"),
    ] {
        assert_eq!(
            schemas["ManagedSwarmServiceView"]["properties"][field]["$ref"],
            format!("#/components/schemas/{kind}")
        );
    }
    for op in ["apply", "scale", "force-update"] {
        assert_eq!(
            doc["paths"][format!("/api/v1/swarmServices/{{id}}/{op}")]["post"]["responses"]["200"]
                ["content"]["application/json"]["schema"]["items"]["$ref"],
            "#/components/schemas/SwarmServiceProgressItem"
        );
    }
}
