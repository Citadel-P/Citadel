use super::*;

#[test]
fn binding_references_are_unique_and_case_insensitive() {
    let references = referenced_binding_names(&[
        "TOKEN=${API_TOKEN}".to_owned(),
        "URL=https://${HOST}/api/${api_token}".to_owned(),
        "PLAIN".to_owned(),
    ])
    .unwrap();

    assert_eq!(references, ["API_TOKEN", "HOST", "PLAIN"]);
}

#[test]
fn incomplete_binding_reference_is_rejected() {
    assert!(referenced_binding_names(&["TOKEN=${MISSING".to_owned()]).is_err());
}

#[test]
fn secret_values_are_redacted_from_runtime_errors() {
    assert_eq!(
        redact(
            "Docker rejected value very-secret".to_owned(),
            &["very-secret".to_owned()]
        ),
        "Docker rejected value ********"
    );
}

#[test]
fn duplicate_clears_triggers_and_provenance_without_changing_mounts_or_placement() {
    let source: SwarmServiceSpec = serde_json::from_value(serde_json::json!({
        "image": {"$type": "External", "registryId": Uuid::from_u128(123), "imageTag": "nginx:latest", "resolvedDigest": "sha256:old"},
        "updateBehavior": "AutoDeploy",
        "webhook": {"enabled": true, "secret": "never-copy"},
        "placementConstraints": ["node.hostname==worker-1"],
        "mounts": [{"kind": "Volume", "source": "production", "target": "/data", "readOnly": true}]
    })).unwrap();
    let copy = source.clone().for_duplicate();
    assert!(copy.validate_duplicate().is_ok());
    assert!(source.validate_duplicate().is_err());
    assert_eq!(copy.mounts, source.mounts);
    assert_eq!(copy.placement_constraints, source.placement_constraints);
    assert!(copy.webhook.is_none());
    assert!(source.webhook.is_some());
    assert_eq!(
        source.clone().for_create().update_behavior,
        UpdateBehavior::AutoDeploy
    );
    let mut forged = copy;
    forged.webhook = source.webhook;
    assert!(forged.validate_duplicate().is_err());
}
