use sha2::{Digest, Sha256};

#[test]
fn protocol_sources_preserve_the_accepted_wire_contract() {
    // The pre-release Script rename changes service/message names only;
    // execution field numbers and Edge command discriminants are retained.
    let sources = [
        (
            "script_service.proto",
            include_str!("../proto/script_service.proto"),
            "4fffd0e3ec41c89921749d525ecabd3c3a452561dd1ce72b710f0ae85788e0e6",
        ),
        (
            "edge_agent_service.proto",
            include_str!("../proto/edge_agent_service.proto"),
            "d3aef861ec91fb951f9f3e17bb556d46fd9583b64b47d48d2a4ed177baac55f3",
        ),
        (
            "shared_models.proto",
            include_str!("../proto/shared_models.proto"),
            "1724f857da584bad61bd67048aeb56606c3d72c7ce4e3e29d4b947ec3a34072c",
        ),
        (
            "platform_service.proto",
            include_str!("../proto/platform_service.proto"),
            "c8d4c61b5122a39ac5ff3b247c4f3423fa470598d608a97ed8d77310709df66b",
        ),
        (
            "container_service.proto",
            include_str!("../proto/container_service.proto"),
            "57fe33f22bf8afafd764500c389cbb4e0a7284175f789279c392cc45e4d43934",
        ),
        (
            "deployment_service.proto",
            include_str!("../proto/deployment_service.proto"),
            "df073b4604ff9f8242535ba6661f37e3f811b36a67c9b84da2a4256bb9720cbc",
        ),
        (
            "stack_service.proto",
            include_str!("../proto/stack_service.proto"),
            "4deb6439dccc4d2e659af7be12f81c90969fa165bbc9cf7f35a5b66dca89f9de",
        ),
        (
            "image_service.proto",
            include_str!("../proto/image_service.proto"),
            "4d47b812a034b8bc561fa4d99f3854ef16446ed502fb28d0b92cfa75861b8bf3",
        ),
        (
            "network_service.proto",
            include_str!("../proto/network_service.proto"),
            "a8ace023dd582a007f74b211f69acfd28bb8ecb86813b44b8026b3df340e21bd",
        ),
        (
            "volume_service.proto",
            include_str!("../proto/volume_service.proto"),
            "270e53f225b16abacc1d63cdb5e23e1efdb1142a6b9cb9efc0d43f7a31d7a2f2",
        ),
        (
            "swarm_service.proto",
            include_str!("../proto/swarm_service.proto"),
            "994ef109395cfaf35d4491ab1fdd8931829fe3665dd4ba38670277285471dfbe",
        ),
    ];
    for (name, source, expected) in sources {
        let normalized = source.replace("\r\n", "\n");
        let actual: String = Sha256::digest(normalized.as_bytes())
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(
            actual, expected,
            "protocol source changed: {name}; review wire compatibility before updating the baseline"
        );
    }
    assert_eq!(citadel_contracts::EDGE_AGENT_PROTOCOL_VERSION, 2);
}

// The accepted additions use new field numbers and preserve old-peer defaults.
#[test]
fn health_collection_is_opt_in_and_old_inventory_preserves_unknown_evidence() {
    use citadel_contracts::citadel::containers::v1::ListContainersRequest;
    use citadel_contracts::citadel::shared_models::v1::{
        ContainerHealthObservation, ContainerMessage,
    };
    use prost::Message;

    #[derive(Clone, PartialEq, Message)]
    struct OldContainer {
        #[prost(string, tag = "1")]
        id: String,
    }

    assert!(
        !ListContainersRequest::decode(&[][..])
            .unwrap()
            .observe_health
    );
    let request = ListContainersRequest {
        observe_health: true,
        ..Default::default()
    };
    assert_eq!(request.encode_to_vec(), [0x30, 1]); // field 6, bool

    let old = OldContainer {
        id: "exact-container-id".into(),
    };
    let decoded = ContainerMessage::decode(old.encode_to_vec().as_slice()).unwrap();
    assert_eq!(decoded.id, old.id);
    assert!(decoded.health_observation.is_none());
    let mut current = ContainerMessage {
        health_observation: Some(Default::default()),
        ..Default::default()
    };
    assert_eq!(current.encode_to_vec(), [0xb2, 0x01, 0]); // field 22, message
    current.id = old.id.clone();
    current.health_observation = Some(ContainerHealthObservation {
        configuration: "Enabled".into(),
        result: Some("Healthy".into()),
        observed_at: 42,
        exit_code: Some(0),
        oom_killed: Some(false),
        ..Default::default()
    });
    let encoded = current.encode_to_vec();
    assert_eq!(OldContainer::decode(encoded.as_slice()).unwrap().id, old.id);
    assert_eq!(
        ContainerMessage::decode(encoded.as_slice()).unwrap(),
        current
    );
    let missing = ContainerHealthObservation::decode(&[][..]).unwrap();
    assert!(missing.result.is_none());
    assert!(missing.exit_code.is_none());
    assert!(missing.oom_killed.is_none());
}

#[test]
fn health_inspection_additions_preserve_old_status_and_unknown_configuration() {
    use citadel_contracts::citadel::shared_models::v1::{
        ContainerConfig, ContainerHealthcheck, Health, HealthLog, InspectContainerResponse,
    };
    use prost::Message;

    assert!(
        !InspectContainerResponse::decode(&[][..])
            .unwrap()
            .health_metadata_observed
    );
    assert!(
        ContainerConfig::decode(&[][..])
            .unwrap()
            .healthcheck
            .is_none()
    );
    let inspected = InspectContainerResponse {
        health_metadata_observed: true,
        ..Default::default()
    };
    assert_eq!(inspected.encode_to_vec(), [0xd0, 0x01, 1]); // field 26, bool
    let mut config = ContainerConfig {
        healthcheck: Some(Default::default()),
        ..Default::default()
    };
    assert_eq!(config.encode_to_vec(), [0xc2, 0x01, 0]); // field 24, message
    config.healthcheck = Some(ContainerHealthcheck {
        test: vec!["CMD".into(), "check".into()],
        interval: Some(30),
        timeout: Some(5),
        retries: Some(3),
        start_period: Some(10),
        start_interval: Some(2),
    });
    assert_eq!(
        ContainerConfig::decode(config.encode_to_vec().as_slice()).unwrap(),
        config
    );

    #[derive(Clone, PartialEq, Message)]
    struct OldHealth {
        #[prost(message, optional, tag = "1")]
        status: Option<String>,
        #[prost(message, optional, tag = "2")]
        failing_streak: Option<i32>,
    }
    let old = OldHealth {
        status: Some("healthy".into()),
        failing_streak: Some(0),
    };
    assert!(
        Health::decode(old.encode_to_vec().as_slice())
            .unwrap()
            .log
            .is_empty()
    );
    let mut current = Health {
        log: vec![Default::default()],
        ..Default::default()
    };
    assert_eq!(current.encode_to_vec(), [0x1a, 0]); // field 3, message
    current.status = old.status.clone();
    current.failing_streak = old.failing_streak;
    current.log = vec![HealthLog {
        start: Some("start".into()),
        end: Some("end".into()),
        exit_code: Some(0),
        output: Some("ok".into()),
    }];
    let encoded = current.encode_to_vec();
    assert_eq!(OldHealth::decode(encoded.as_slice()).unwrap(), old);
    assert_eq!(Health::decode(encoded.as_slice()).unwrap(), current);
}

#[test]
fn container_mount_inventory_uses_additive_fields_and_preserves_unknown_state() {
    use citadel_contracts::citadel::shared_models::v1::ContainerMessage;
    use prost::Message;

    let absent = ContainerMessage::decode(&[][..]).unwrap();
    assert!(absent.mounts.is_empty());
    assert!(!absent.mounts_observed);

    let mut observed = ContainerMessage {
        mounts_observed: true,
        ..Default::default()
    };
    assert_eq!(observed.encode_to_vec(), [0xa8, 0x01, 1]); // field 21, bool
    observed.mounts.push(Default::default());
    assert_eq!(observed.encode_to_vec(), [0xa2, 0x01, 0, 0xa8, 0x01, 1]); // field 20, message
    let round_trip = ContainerMessage::decode(observed.encode_to_vec().as_slice()).unwrap();
    assert!(round_trip.mounts_observed);
    assert_eq!(round_trip.mounts.len(), 1);
}

#[test]
fn container_metadata_and_capture_time_are_additive_wire_fields() {
    use citadel_contracts::citadel::containers::v1::{
        ContainersStatsResponse, ListContainersRequest,
    };
    use prost::Message;

    let old_request = ListContainersRequest::decode(&[][..]).unwrap();
    assert!(!old_request.metadata_only);
    let request = ListContainersRequest {
        metadata_only: true,
        ..Default::default()
    };
    assert_eq!(request.encode_to_vec(), [0x28, 1]); // field 5, bool

    let old_stats = ContainersStatsResponse::decode(&[][..]).unwrap();
    assert_eq!(old_stats.captured_at, None);
    let stats = ContainersStatsResponse {
        captured_at: Some(42),
        ..Default::default()
    };
    assert_eq!(stats.encode_to_vec(), [0x10, 42]); // field 2, int64
}

#[test]
fn image_comparison_fields_are_additive_and_old_peers_preserve_unknown_metadata() {
    use citadel_contracts::citadel::images::v1::{
        DistributionInspectRequest, DistributionInspectResponse, InspectImageResponse,
        OciDescriptorResponse, OciPlatformResponse,
    };
    use prost::Message;

    assert!(
        DistributionInspectRequest::decode(&[][..])
            .unwrap()
            .target_platform
            .is_none()
    );
    let old_response = DistributionInspectResponse::decode(&[][..]).unwrap();
    assert!(old_response.config_digest.is_empty());
    assert!(old_response.manifest_digest.is_empty());
    assert!(
        InspectImageResponse::decode(&[][..])
            .unwrap()
            .variant
            .is_none()
    );
    assert!(
        OciPlatformResponse::decode(&[][..])
            .unwrap()
            .variant
            .is_empty()
    );

    let target = DistributionInspectRequest {
        target_platform: Some(Default::default()),
        ..Default::default()
    };
    assert_eq!(target.encode_to_vec(), [0x1a, 0]); // field 3, message
    let metadata = DistributionInspectResponse {
        config_digest: "c".into(),
        manifest_digest: "m".into(),
        ..Default::default()
    };
    assert_eq!(metadata.encode_to_vec(), [0x1a, 1, b'c', 0x22, 1, b'm']); // fields 3 and 4
    let platform = OciPlatformResponse {
        variant: "v".into(),
        ..Default::default()
    };
    assert_eq!(platform.encode_to_vec(), [0x22, 1, b'v']); // field 4
    let inspection = InspectImageResponse {
        variant: Some("v".into()),
        ..Default::default()
    };
    assert_eq!(inspection.encode_to_vec(), [0x92, 0x01, 3, 0x0a, 1, b'v']); // field 18, StringValue

    #[derive(Clone, PartialEq, Message)]
    struct OldDistributionResponse {
        #[prost(message, optional, tag = "1")]
        descriptor: Option<OciDescriptorResponse>,
        #[prost(message, repeated, tag = "2")]
        platforms: Vec<OciPlatformResponse>,
    }
    let response = DistributionInspectResponse {
        descriptor: Some(OciDescriptorResponse {
            digest: "top-level-digest".into(),
            ..Default::default()
        }),
        platforms: vec![OciPlatformResponse {
            architecture: "amd64".into(),
            os: "linux".into(),
            ..Default::default()
        }],
        ..metadata
    };
    let old = OldDistributionResponse::decode(response.encode_to_vec().as_slice()).unwrap();
    assert_eq!(old.descriptor.unwrap().digest, "top-level-digest");
    assert_eq!(old.platforms[0].architecture, "amd64");
}

#[test]
fn image_inspection_identity_fields_are_additive_and_absence_stays_unknown() {
    use citadel_contracts::citadel::images::v1::InspectImageResponse;
    use prost::Message;
    #[derive(Clone, PartialEq, Message)]
    struct OldInspection {
        #[prost(string, tag = "11")]
        id: String,
    }
    let old = OldInspection {
        id: "display-id".into(),
    };
    let decoded = InspectImageResponse::decode(old.encode_to_vec().as_slice()).unwrap();
    assert!(decoded.config_digest.is_none() && decoded.content_digest.is_none());
    for (config_digest, content_digest, field) in [
        (Some("c".into()), None, 0x9a),
        (None, Some("c".into()), 0xa2),
    ] {
        let response = InspectImageResponse {
            config_digest,
            content_digest,
            ..Default::default()
        };
        assert_eq!(response.encode_to_vec(), [field, 0x01, 3, 0x0a, 1, b'c']);
        let response = InspectImageResponse {
            id: old.id.clone(),
            ..response
        };
        assert_eq!(
            OldInspection::decode(response.encode_to_vec().as_slice())
                .unwrap()
                .id,
            old.id
        );
    }
}

#[test]
fn network_attachments_are_additive_and_old_peers_preserve_network_identity() {
    use citadel_contracts::citadel::shared_models::v1::{Network, NetworkAttachment};
    use prost::Message;

    #[derive(Clone, PartialEq, Message)]
    struct OldNetwork {
        #[prost(string, tag = "1")]
        name: String,
        #[prost(string, tag = "2")]
        id: String,
    }
    let old = OldNetwork {
        name: "application_backend".into(),
        id: "network-id".into(),
    };
    let mut current = Network::decode(old.encode_to_vec().as_slice()).unwrap();
    assert!(current.containers.is_empty());
    let attachment = NetworkAttachment {
        name: "web".into(),
        endpoint_id: "endpoint-id".into(),
        mac_address: "02:42:ac:14:00:02".into(),
        ipv4_address: "172.20.0.2/24".into(),
        ipv6_address: "fd00::2/64".into(),
    };
    let map_only = Network {
        containers: [(
            "c".into(),
            NetworkAttachment {
                name: "n".into(),
                ..Default::default()
            },
        )]
        .into(),
        ..Default::default()
    };
    // Field 17, map entry: string key (field 1), message value (field 2).
    assert_eq!(
        map_only.encode_to_vec(),
        [0x8a, 0x01, 8, 0x0a, 1, b'c', 0x12, 3, 0x0a, 1, b'n']
    );
    current.containers.insert("container-id".into(), attachment);
    let encoded = current.encode_to_vec();
    assert_eq!(OldNetwork::decode(encoded.as_slice()).unwrap(), old);
    assert_eq!(Network::decode(encoded.as_slice()).unwrap(), current);
}

#[test]
fn network_attachment_fields_keep_their_wire_numbers() {
    use citadel_contracts::citadel::shared_models::v1::NetworkAttachment;
    use prost::Message;

    let attachment = NetworkAttachment {
        name: "n".into(),
        endpoint_id: "e".into(),
        mac_address: "m".into(),
        ipv4_address: "4".into(),
        ipv6_address: "6".into(),
    };
    // Pin each field independently of generated types. A self-round-trip alone
    // would still pass if a field were accidentally renumbered.
    let bytes = [
        0x0a, 1, b'n', 0x12, 1, b'e', 0x1a, 1, b'm', 0x22, 1, b'4', 0x2a, 1, b'6',
    ];
    assert_eq!(attachment.encode_to_vec(), bytes);
    assert_eq!(
        NetworkAttachment::decode(bytes.as_slice()).unwrap(),
        attachment
    );
}
