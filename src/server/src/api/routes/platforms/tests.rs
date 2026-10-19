#[test]
fn platform_tags_accept_names_repeated_keys_and_ids() {
    let id = uuid::Uuid::now_v7();
    assert_eq!(
        super::parse_tag_filters(Some(&format!(
            "TaGs=Prod%20%26%20Europe&tags=prod+%26+europe&tags=%20&tags={id}"
        )))
        .unwrap(),
        vec!["Prod & Europe".to_owned(), id.to_string()]
    );
    assert!(super::parse_tag_filters(None).unwrap().is_empty());
    let query = (0..100)
        .map(|n| format!("tags=tag-{n}"))
        .collect::<Vec<_>>()
        .join("&");
    assert_eq!(super::parse_tag_filters(Some(&query)).unwrap().len(), 100);
    assert!(super::parse_tag_filters(Some(&format!("{query}&tags=extra"))).is_err());
}

use crate::api::routes::platforms::*;

#[test]
fn volume_storage_policy_restricts_even_administrator_capabilities() {
    for allowed in [Some(true), Some(false), None] {
        let view = map_volume(
            RuntimeVolumeSummary {
                content_access_allowed: allowed,
                ..Default::default()
            },
            VolumeCapabilitiesView {
                can_browse: true,
                can_download: true,
                can_read: true,
                ..Default::default()
            },
        )
        .unwrap();
        let capabilities = view.capabilities.unwrap();
        assert_eq!(capabilities.can_browse, allowed == Some(true));
        assert_eq!(capabilities.can_download, allowed == Some(true));
        assert!(capabilities.can_read);
    }
    let view = map_volume(
        RuntimeVolumeSummary {
            content_access_allowed: Some(true),
            ..Default::default()
        },
        VolumeCapabilitiesView::default(),
    )
    .unwrap();
    assert!(
        !view.capabilities.unwrap().can_browse,
        "storage safety must not grant permissions"
    );
}

#[test]
fn volume_usage_is_serialized_with_the_existing_ui_field_names() {
    for usage in [
        serde_json::json!({"Size": 1024, "RefCount": 2}),
        serde_json::json!({"size": 1024, "refCount": 2}),
    ] {
        let view = map_volume(
            RuntimeVolumeSummary {
                usage_data: Some(usage),
                ..Default::default()
            },
            VolumeCapabilitiesView::default(),
        )
        .unwrap();
        let json = serde_json::to_value(view).unwrap();
        assert_eq!(
            json["usageData"],
            serde_json::json!({"size":1024, "refCount":2})
        );
    }
    let view = map_volume(
        RuntimeVolumeSummary::default(),
        VolumeCapabilitiesView::default(),
    )
    .unwrap();
    assert!(view.usage_data.is_none());
}

#[test]
fn null_volume_status_matches_the_empty_dotnet_string() {
    let volume = RuntimeVolumeSummary {
        status: std::collections::BTreeMap::from([("missing".into(), serde_json::Value::Null)]),
        ..Default::default()
    };
    assert_eq!(
        map_volume(volume, VolumeCapabilitiesView::default())
            .unwrap()
            .status["missing"],
        ""
    );
}

#[test]
fn execute_implies_write_and_read_without_granting_specific_operations() {
    let capabilities = platform_capabilities(EffectivePlatformPermission {
        level_mask: PermissionLevel::Execute as i32,
        specific_mask: SpecificPermission::ManageNodeAgents as i32,
    });

    assert!(capabilities.can_read);
    assert!(capabilities.can_write);
    assert!(capabilities.can_execute);
    assert!(capabilities.can_manage_node_agents);
    assert!(!capabilities.can_inspect);
}

#[test]
fn docker_ids_are_bounded_before_storage_or_transport() {
    assert!(validate_docker_resource_id("node-1").is_ok());
    assert!(validate_docker_resource_id(" ").is_err());
    assert!(validate_docker_resource_id(&"a".repeat(257)).is_err());
}

#[test]
fn task_filters_preserve_dotnet_bounds() {
    assert!(
        validate_swarm_task_filters(&SwarmTaskFilters {
            limit: 1,
            service_id: Some("service-1".into()),
        })
        .is_ok()
    );
    assert!(
        validate_swarm_task_filters(&SwarmTaskFilters {
            limit: 0,
            service_id: None,
        })
        .is_err()
    );
    assert!(
        validate_swarm_task_filters(&SwarmTaskFilters {
            limit: 201,
            service_id: None,
        })
        .is_err()
    );
    assert!(
        validate_swarm_task_filters(&SwarmTaskFilters {
            limit: 50,
            service_id: Some("x".repeat(65)),
        })
        .is_err()
    );
}

#[test]
fn docker_inventory_filters_use_runtime_identity_and_usage() {
    let network = RuntimeNetworkSummary {
        id: "abcdef".into(),
        name: "frontend".into(),
        driver: "bridge".into(),
        container_count: 0,
        ..RuntimeNetworkSummary::default()
    };
    assert!(network_matches(
        &network,
        &NetworkFilters {
            dangling: Some(true),
            driver: Some("BRIDGE".into()),
            id: Some("ABC".into()),
            name: Some("FRONT".into()),
        }
    ));
    let volume = RuntimeVolumeSummary {
        name: "database".into(),
        in_use: true,
        driver: "local".into(),
        ..RuntimeVolumeSummary::default()
    };
    assert!(volume_matches(
        &volume,
        &VolumeFilters {
            dangling: Some(false),
            driver: Some("LOCAL".into()),
            name: Some("DATA".into()),
        }
    ));
}

#[test]
fn disconnected_platform_uses_the_existing_conflict_contract() {
    let response = runtime_error_response(
        RuntimeCapabilityError::new(RuntimeErrorKind::Unavailable, "offline", true),
        &HeaderMap::new(),
    );
    assert_eq!(response.status(), StatusCode::CONFLICT);
}

#[test]
fn network_create_accepts_the_existing_ui_ipam_and_ip_version_fields() {
    let input: CreateNetworkInput = serde_json::from_value(serde_json::json!({
        "platformId": Uuid::new_v4(), "name":"fscsd", "driver":"bridge",
        "scope":"local", "enableIPv4":true, "enableIPv6":false,
        "internal":false, "attachable":false, "ingress":false,
        "labels":{}, "options":{}, "ipam":{"driver":"default", "config":[{},{}]},
        "configOnly":false
    }))
    .unwrap();
    assert!(validate_network_input(&input.network).is_ok());
    assert_eq!(input.network.enable_ipv4, Some(true));
    assert_eq!(input.network.enable_ipv6, Some(false));
    assert!(input.network.ipam.unwrap().config[0].subnet.is_none());
    let invalid: CreateRuntimeNetwork = serde_json::from_value(serde_json::json!({
        "name":"invalid", "driver":"bridge", "scope":"local",
        "enableIPv4":false, "enableIPv6":false
    }))
    .unwrap();
    assert!(validate_network_input(&invalid).is_err());
}

#[test]
fn network_mutation_validation_preserves_swarm_scope_rules() {
    let valid = CreateRuntimeNetwork {
        name: "frontend".into(),
        driver: "overlay".into(),
        scope: "swarm".into(),
        internal: None,
        attachable: Some(true),
        ingress: None,
        enable_ipv6: Some(false),
        enable_ipv4: Some(true),
        config_only: None,
        ipam: None,
        config_from: None,
        labels: Default::default(),
        options: Default::default(),
    };
    assert!(validate_network_input(&valid).is_ok());
    assert!(
        validate_network_input(&CreateRuntimeNetwork {
            driver: "bridge".into(),
            ..valid.clone()
        })
        .is_err()
    );
    assert!(
        validate_network_input(&CreateRuntimeNetwork {
            enable_ipv4: Some(false),
            ..valid
        })
        .is_err()
    );
}

#[test]
fn batch_mutation_ids_are_bounded_trimmed_and_deduplicated() {
    let mut ids = vec![" network-1 ".into(), "network-1".into(), "network-2".into()];
    validate_resource_ids(&mut ids, 100, "Network").unwrap();
    assert_eq!(ids, ["network-1", "network-2"]);
    assert!(validate_resource_ids(&mut Vec::new(), 100, "Network").is_err());
}
