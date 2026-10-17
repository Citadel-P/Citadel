use sha2::{Digest, Sha256};

#[test]
fn protocol_sources_preserve_the_accepted_wire_contract() {
    let sources = [
        (
            "edge_agent_service.proto",
            include_str!("../proto/edge_agent_service.proto"),
            "f3280c2f341ad5860ed5b34f2e4371356052aa1b66204957e6f0de177efaff32",
        ),
        (
            "shared_models.proto",
            include_str!("../proto/shared_models.proto"),
            "62f88fd480eded70d918f80c46781cf30f2ea2be065b63bf898b964164abbc7d",
        ),
        (
            "platform_service.proto",
            include_str!("../proto/platform_service.proto"),
            "c8d4c61b5122a39ac5ff3b247c4f3423fa470598d608a97ed8d77310709df66b",
        ),
        (
            "container_service.proto",
            include_str!("../proto/container_service.proto"),
            "5b408a811393f13cc2257c32888c8228dbc923efb6860a23ab792776530a97a9",
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
            "7bf4ebf9deab755fb6868d646e6539b38cd08ee2a11e2a9c8ec307081a253321",
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
