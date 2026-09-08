use citadel_deployments::adoption::*;
use serde_json::{Value, json};
use uuid::Uuid;

fn inspect() -> Value {
    json!({"Id":"container","Config":{"Image":"nginx:latest","Env":["LOG_LEVEL=info","API_KEY=never-disclose","DATABASE_URL=private-db"],"Labels":{},"Entrypoint":["/entrypoint"],"User":"root"},"HostConfig":{"RestartPolicy":{"Name":"always"},"Memory":268435456,"MemorySwap":536870912,"ShmSize":67108864,"CpuPeriod":100000,"IpcMode":"private","PortBindings":{"80/tcp":[{"HostPort":"8080","HostIp":"0.0.0.0"}]}},"Mounts":[{"Type":"volume","Name":"data","Destination":"/data","RW":false}],"NetworkSettings":{"Networks":{"bridge":{}}}})
}
fn draft(inspection: &Value) -> ContainerAdoptionDraft {
    map_draft(
        AdoptionSource {
            id: Uuid::nil(),
            docker_container_id: "container".into(),
            name: "/nginx".into(),
            platform_id: Uuid::nil(),
            platform_name: "local".into(),
            state: "Running".into(),
        },
        inspection,
        Some(Uuid::nil()),
        Some(&json!({"Entrypoint":["/entrypoint"],"User":"0"})),
        "fingerprint".into(),
        "nginx".into(),
    )
    .unwrap()
}

#[test]
fn maps_supported_settings_and_redacts_sensitive_environment_like_dotnet() {
    let result = draft(&inspect());
    assert!(result.issues.iter().all(|i| i.severity != "Blocker"));
    assert_eq!(result.draft.spec.ports.unwrap(), ["8080:80/tcp"]);
    assert_eq!(result.draft.spec.volumes.unwrap(), ["data:/data:ro"]);
    assert_eq!(result.draft.spec.networks.unwrap(), ["bridge"]);
    assert_eq!(
        result.draft.spec.resource_spec.unwrap().memory_limit,
        Some(256.0)
    );
    assert!(result.can_import_sensitive_environment_values);
    let env = result.draft.spec.environment_variables.unwrap();
    assert!(env.contains(&"API_KEY=${API_KEY}".into()));
    assert!(
        !env.iter()
            .any(|e| e.contains("never-disclose") || e.contains("private-db"))
    );
}

#[test]
fn agent_and_edge_inspection_shapes_preserve_ports_and_networks() {
    let native = draft(&inspect());
    let mut agent = inspect();
    agent["HostConfig"]["PortBindings"] = json!({
        "80/tcp": {"host_port_binding": [{"host_port":"8080", "host_ip":"0.0.0.0"}]}
    });
    agent["NetworkSettings"]["Networks"] = json!([{"key":"bridge", "value":{}}]);
    let mapped = draft(&agent);
    assert_eq!(mapped.draft.spec.ports, native.draft.spec.ports);
    assert_eq!(mapped.draft.spec.networks, native.draft.spec.networks);
    assert!(
        mapped
            .issues
            .iter()
            .all(|issue| issue.severity != "Blocker")
    );
}

#[test]
fn blocks_unrepresentable_settings_but_accepts_docker_defaults() {
    for (key, value) in [
        ("Privileged", json!(true)),
        ("SecurityOpt", json!(["no-new-privileges"])),
        ("IpcMode", json!("host")),
        ("NetworkMode", json!("container:other")),
        ("MemorySwap", json!(-1)),
        ("OomScoreAdj", json!(100)),
        ("CpuPeriod", json!(1000)),
        ("Tmpfs", json!({"/run":""})),
    ] {
        let mut inspection = inspect();
        inspection["HostConfig"][key] = value;
        assert!(
            draft(&inspection)
                .issues
                .iter()
                .any(|i| i.severity == "Blocker"),
            "{key}"
        );
    }
    let mut inspection = inspect();
    inspection["Config"]["User"] = json!("1000");
    assert!(
        draft(&inspection)
            .issues
            .iter()
            .any(|i| i.code == "IMAGE_OVERRIDE_NOT_PRESERVED")
    );
    inspection["Config"]["Labels"] = json!({"com.docker.compose.project":"beszel"});
    assert!(
        draft(&inspection)
            .issues
            .iter()
            .any(|i| i.code == "COMPOSE_PROJECT_CONTAINER")
    );
}

#[test]
fn sensitive_values_accept_existing_bindings_and_reject_unresolved_placeholders() {
    let available = vec!["API_KEY".into(), "TOKEN".into()];
    for entry in [
        "API_KEY",
        "API_KEY=${API_KEY}",
        "API_KEY=prefix-${API_KEY}-${TOKEN}",
        "API_KEY=replaced",
    ] {
        assert!(
            has_resolved_sensitive_environment(&[entry.into()], "API_KEY", &available),
            "{entry}"
        );
    }
    for entry in [
        "API_KEY=",
        "API_KEY=********",
        "API_KEY=${MISSING}",
        "API_KEY=${API_KEY}-${MISSING}",
        "LOG_LEVEL=info",
    ] {
        assert!(
            !has_resolved_sensitive_environment(&[entry.into()], "API_KEY", &available),
            "{entry}"
        );
    }
    assert!(!has_resolved_sensitive_environment(
        &["API_KEY".into()],
        "API_KEY",
        &[]
    ));
}

#[test]
fn sensitive_classifier_matches_dotnet_boundaries_and_name_normalization() {
    for name in [
        "API_KEY",
        "stripe_api_key_1",
        "DATABASE_URL",
        "ConnectionStrings__Default",
        "BESZEL_AGENT_TOKEN",
        "PRIVATE.KEY",
        "db-password",
    ] {
        assert!(sensitive(name), "{name}");
    }
    for name in [
        "LOG_LEVEL",
        "TOKENIZER",
        "MONKEY",
        "PUBLIC_KEY",
        "PASSWORDLESS",
    ] {
        assert!(!sensitive(name), "{name}");
    }
    assert_eq!(draft_name("/nginx"), "nginx");
    assert_eq!(draft_name("nginx"), "nginx");
    assert_eq!(draft_name("/a"), "a-app");
    assert!(valid_binding_name("API_KEY"));
    assert!(!valid_binding_name("1TOKEN"));
}
