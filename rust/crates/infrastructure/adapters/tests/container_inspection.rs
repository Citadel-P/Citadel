use citadel_adapters::connectors::containers::inspection::map_inspection;
use serde_json::json;

// Verify redaction of sensitive container inspection fields.
#[test]
fn inspection_masks_sensitive_environment_and_preserves_ordinary_values() {
    let mut env = vec![
        "API_TOKEN",
        "STRIPE_API_KEY",
        "POSTGRES_PASSWORD",
        "AWS_SECRET_ACCESS_KEY",
        "ConnectionStrings__Default",
        "SSH_PRIVATE_KEY",
    ]
    .into_iter()
    .map(|name| format!("{name}=do-not-return"))
    .collect::<Vec<_>>();
    env.extend(
        [
            "PUBLIC_VALUE=visible",
            "APP_MODE=production",
            "MONKEY=banana",
            "TOKENIZER_MODEL=standard",
            "EMPTY=",
            "INHERITED",
            "API_KEY=",
        ]
        .into_iter()
        .map(str::to_owned),
    );
    let original = json!({"Id":"container", "Created":"today", "Config":{"Env":env}});
    let result = map_inspection(original.clone(), "container").unwrap();
    assert!(!result.to_string().contains("do-not-return"));
    assert_eq!(
        result["config"]["env"],
        json!([
            "API_TOKEN=********",
            "STRIPE_API_KEY=********",
            "POSTGRES_PASSWORD=********",
            "AWS_SECRET_ACCESS_KEY=********",
            "ConnectionStrings__Default=********",
            "SSH_PRIVATE_KEY=********",
            "PUBLIC_VALUE=visible",
            "APP_MODE=production",
            "MONKEY=banana",
            "TOKENIZER_MODEL=standard",
            "EMPTY=",
            "INHERITED",
            "API_KEY="
        ])
    );
    assert_eq!(original["Config"]["Env"][0], "API_TOKEN=do-not-return");
}

// The inspection assertion from VaultKvV2CompatibilityTests. This is a
// redaction port, not a substitute for resolving/injecting a secret using Vault.
#[test]
fn inspection_redacts_vault_values_including_equals_and_newlines() {
    let result = map_inspection(
        json!({
            "Id":"container", "Config":{"Env":[
                "CITADEL_VAULT_SECRET=private=value\nsecond-line",
                "PUBLIC_VALUE=one=two", "CITADEL_VAULT_SECRET=", "CITADEL_VAULT_SECRET"
            ]}
        }),
        "container",
    )
    .unwrap();
    assert_eq!(
        result["config"]["env"],
        json!([
            "CITADEL_VAULT_SECRET=********",
            "PUBLIC_VALUE=one=two",
            "CITADEL_VAULT_SECRET=",
            "CITADEL_VAULT_SECRET"
        ])
    );
    assert!(!result.to_string().contains("private=value"));
}

#[test]
fn inspection_without_configuration_preserves_other_fields() {
    let original = json!({"id":"container", "config":null, "name":"/web",
        "created":"2026-07-26T00:00:00Z", "args":[], "execIDs":[], "mounts":[]});
    assert_eq!(
        map_inspection(original.clone(), "container").unwrap(),
        original
    );
}

#[test]
fn inspection_preserves_dictionary_keys_and_the_existing_ui_shape() {
    let result = map_inspection(json!({"Id":"container", "State":{"Status":"running", "OOMKilled":false},
        "Config":{"Labels":{"MixedCase_Label":"Value"}, "ExposedPorts":{"80/tcp":{}}, "Volumes":{"/Data_Path":{}}},
        "HostConfig":{"PortBindings":{"80/tcp":[{"HostIP":"127.0.0.1","HostPort":"8080"}]}, "LogConfig":{"Type":"json-file", "Config":{"max-size":"1m"}}},
        "NetworkSettings":{"Networks":{"Network_UPPER":{"IPAddress":"10.0.0.2"}}},
        "GraphDriver":{"Data":{"UpperDir":"/some/path"}}
    }), "container").unwrap();
    assert_eq!(result["state"]["status"], "Running");
    assert_eq!(result["state"]["oomKilled"], false);
    assert_eq!(result["config"]["labels"]["MixedCase_Label"], "Value");
    assert_eq!(result["config"]["exposedPorts"], json!(["80/tcp"]));
    assert_eq!(result["config"]["volumes"], json!(["/Data_Path"]));
    assert_eq!(
        result["hostConfig"]["portBindings"]["80/tcp"][0]["hostIP"],
        "127.0.0.1"
    );
    assert_eq!(
        result["hostConfig"]["logConfig"]["config"]["max-size"],
        "1m"
    );
    assert_eq!(
        result["networkSettings"]["networks"]["Network_UPPER"]["ipAddress"],
        "10.0.0.2"
    );
    assert_eq!(result["graphDriver"]["data"]["UpperDir"], "/some/path");
}

#[test]
fn protobuf_inspection_uses_the_same_ports_networks_and_redaction() {
    use citadel_contracts::citadel::shared_models::v1::*;
    let response = InspectContainerResponse {
        id: "container".into(),
        state: Some(ContainerState {
            status: ContainerStateType::Running as i32,
            ..Default::default()
        }),
        config: Some(ContainerConfig {
            env: vec!["API_KEY=private".into()],
            ..Default::default()
        }),
        host_config: Some(HostConfig {
            port_bindings: [(
                "80/tcp".into(),
                HostPortBindingList {
                    host_port_binding: vec![HostPortBinding {
                        host_ip: Some("127.0.0.1".into()),
                        host_port: Some("8080".into()),
                    }],
                },
            )]
            .into(),
            ..Default::default()
        }),
        network_settings: Some(NetworkSettings {
            networks: vec![MapFieldNetwork {
                key: "My_Network".into(),
                value: Some(EndpointSettings {
                    ip_address: Some("10.0.0.2".into()),
                    ..Default::default()
                }),
            }],
            ..Default::default()
        }),
        ..Default::default()
    };
    let result = map_inspection(serde_json::to_value(response).unwrap(), "container").unwrap();
    assert_eq!(result["state"]["status"], "Running");
    assert_eq!(result["config"]["env"], json!(["API_KEY=********"]));
    assert_eq!(
        result["hostConfig"]["portBindings"]["80/tcp"][0]["hostIP"],
        "127.0.0.1"
    );
    assert_eq!(
        result["networkSettings"]["networks"]["My_Network"]["ipAddress"],
        "10.0.0.2"
    );
}

#[test]
fn absent_configuration_is_supported_but_mismatched_identity_is_rejected() {
    assert!(
        map_inspection(json!({"Id":"container","Config":null}), "container").unwrap()["config"]
            .is_null()
    );
    assert!(map_inspection(json!({"Id":"different"}), "container").is_err());
}

#[tokio::test]
async fn cancelled_inspection_does_not_connect_to_docker() {
    use citadel_platforms::containers::ContainerInspectionPort;
    let client = citadel_adapters::connectors::docker::DockerClient::new(
        "/nonexistent/inspection.sock",
        std::time::Duration::from_secs(1),
    )
    .unwrap();
    let cancel = tokio_util::sync::CancellationToken::new();
    cancel.cancel();
    assert_eq!(
        client
            .inspection("container", &cancel)
            .await
            .unwrap_err()
            .kind,
        citadel_platforms::RuntimeErrorKind::Cancelled
    );
}
