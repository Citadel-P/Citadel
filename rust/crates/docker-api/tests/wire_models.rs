use citadel_docker_api::models::*;
use serde_json::{Value, json};

#[test]
fn task_name_and_unknown_response_properties_are_compatible() {
    let task: Task = serde_json::from_value(json!({
        "ID": "task-1", "ServiceID": "service-1", "FutureDaemonField": {"enabled": true}
    }))
    .unwrap();
    assert_eq!(task.id.as_deref(), Some("task-1"));
    assert_eq!(task.service_id.as_deref(), Some("service-1"));
    let info: SystemInfo = serde_json::from_value(json!({
        "Swarm": {"RemoteManagers": null}, "FutureDaemonField": 42
    }))
    .unwrap();
    assert_eq!(info.swarm.unwrap().remote_managers, Some(None));
}

#[test]
fn swarm_create_responses_preserve_uppercase_id() {
    // Both ConfigCreate and SecretCreate return this generated response type.
    let created: SwarmResourceCreateResponse =
        serde_json::from_str(r#"{"ID":"new-id","Future":1}"#).unwrap();
    assert_eq!(created.id, "new-id");
    assert_eq!(
        serde_json::to_value(created).unwrap(),
        json!({"ID":"new-id","Future":1})
    );
    assert!(serde_json::from_str::<SwarmResourceCreateResponse>(r#"{"Id":"wrong-case"}"#).is_err());
}

#[test]
fn volume_nullability_matches_daemon_payloads() {
    let response: VolumeListResponse = serde_json::from_value(json!({
        "Volumes": [{"Name":"data", "Driver":"local", "Mountpoint":"/data",
            "Scope":null, "Labels":null, "Options":null, "Status":null, "UsageData":null}],
        "Warnings":null
    }))
    .unwrap();
    let volume = &response.volumes.unwrap()[0];
    assert!(volume.scope.is_none());
    assert!(volume.labels.is_none());
    assert!(volume.options.is_none());
    assert!(volume.status.is_none());
    assert!(response.warnings.is_none());
    let local: Volume = serde_json::from_value(json!({
        "Name":"data", "Driver":"local", "Mountpoint":"/data",
        "Scope":"local", "Labels":{}, "Options":{}
    }))
    .unwrap();
    assert_eq!(serde_json::to_value(local).unwrap()["Scope"], "local");
}

#[test]
fn dangling_images_accept_null_tags_digests_and_labels() {
    let image: ImageSummary = serde_json::from_value(json!({
        "Id":"sha256:alpine", "ParentId":"", "Created":1, "Size":42,
        "SharedSize":-1, "Containers":-1, "Labels":null, "RepoTags":null, "RepoDigests":null
    }))
    .unwrap();
    assert_eq!(image.id, "sha256:alpine");
    assert!(image.repo_tags.is_none());
    assert!(image.repo_digests.is_none());
    assert!(image.labels.is_none());
    let usage: SystemDataUsageResponse =
        serde_json::from_value(json!({"LayersSize":-1,"Volumes":null})).unwrap();
    assert!(usage.volumes.is_none());
}

#[test]
fn optional_collections_and_empty_object_maps_round_trip() {
    let config: ContainerConfig = serde_json::from_value(json!({
        "Env":null, "Cmd":null, "Entrypoint":null,
        "ExposedPorts":{"80/tcp":{}}, "Volumes":{"/data":{}}
    }))
    .unwrap();
    assert!(config.env.is_none());
    assert!(config.cmd.is_none());
    assert!(config.entrypoint.is_none());
    let encoded = serde_json::to_value(config).unwrap();
    assert_eq!(encoded["ExposedPorts"]["80/tcp"], json!({}));
    assert_eq!(encoded["Volumes"]["/data"], json!({}));
    let network: Network = serde_json::from_value(json!({"Containers":null,"Peers":null})).unwrap();
    assert!(network.containers.is_none());
    assert_eq!(network.peers, Some(None));
}

#[test]
fn statistics_preserve_unsigned_counters_above_i32_and_i64() {
    let stats: ContainerStatsResponse = serde_json::from_value(json!({
        "networks":{"eth0":{"rx_bytes":u64::MAX,"tx_bytes":5_000_000_000_u64,
            "endpoint_id":"endpoint", "instance_id":"instance", "future_counter":2}},
        "cpu_stats":{"cpu_usage":{"total_usage":9_000_000_000_u64,"percpu_usage":[8_000_000_000_u64]},
            "system_cpu_usage":10_000_000_000_u64,"online_cpus":4},
        "memory_stats":{"usage":6_000_000_000_u64,"stats":{"cache":5_000_000_000_u64}}
    })).unwrap();
    let networks = stats.networks.unwrap().unwrap();
    assert_eq!(networks["eth0"].rx_bytes, Some(u64::MAX));
    assert_eq!(networks["eth0"].tx_bytes, Some(5_000_000_000));
    let cpu = stats.cpu_stats.unwrap().unwrap();
    assert_eq!(cpu.system_cpu_usage, Some(Some(10_000_000_000)));
    assert_eq!(
        cpu.cpu_usage.unwrap().unwrap().total_usage,
        Some(9_000_000_000)
    );
    let memory = stats.memory_stats.unwrap();
    assert_eq!(memory.usage, Some(Some(6_000_000_000)));
    assert_eq!(memory.stats.unwrap()["cache"], 5_000_000_000);
    let empty: ContainerStatsResponse = serde_json::from_value(json!({"networks":null})).unwrap();
    assert_eq!(empty.networks, Some(None));
}

#[test]
fn numeric_enums_keep_their_wire_representation() {
    let change: FilesystemChange =
        serde_json::from_value(json!({"Path":"/file","Kind":1})).unwrap();
    assert_eq!(
        serde_json::to_value(change).unwrap()["Kind"],
        Value::from(1)
    );
}

#[test]
fn container_create_preserves_inherited_configuration_and_host_options() {
    // ContainerCreate is an allOf schema; verify inherited and endpoint-local fields.
    let wire = json!({
        "Image": "alpine:latest",
        "Env": ["MODE=test"],
        "ExposedPorts": {"80/tcp": {}},
        "HostConfig": {
            "Binds": ["data:/data:ro"],
            "PortBindings": {"80/tcp": [{"HostIp":"127.0.0.1", "HostPort":"8080"}]}
        },
        "NetworkingConfig": {"EndpointsConfig": {"private": {"Aliases": ["service"]}}}
    });
    let request: ContainerCreateRequest = serde_json::from_value(wire.clone()).unwrap();
    let encoded = serde_json::to_value(request).unwrap();
    for field in [
        "Image",
        "Env",
        "ExposedPorts",
        "HostConfig",
        "NetworkingConfig",
    ] {
        assert_eq!(encoded[field], wire[field], "{field}");
    }
}

#[test]
fn containerd_image_store_accepts_null_graph_driver_data() {
    let image: ImageInspect = serde_json::from_value(
        json!({"Id":"sha256:image","GraphDriver":{"Name":"overlayfs","Data":null}}),
    )
    .unwrap();
    assert!(image.graph_driver.unwrap().data.is_none());
}
