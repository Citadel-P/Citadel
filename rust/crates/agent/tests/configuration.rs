use std::collections::BTreeMap;
use std::path::Path;

use base64::{Engine, engine::general_purpose::STANDARD};
use citadel_agent::config::{AgentConfig, AgentMode, DockerEndpoint, EdgeProfile};
use ed25519_dalek::SigningKey;

fn direct() -> BTreeMap<String, String> {
    BTreeMap::from([(
        "HUB_PUBLIC_KEY".into(),
        STANDARD.encode(SigningKey::from_bytes(&[7; 32]).verifying_key().as_bytes()),
    )])
}

fn edge() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("CITADEL_AGENT_MODE".into(), "edge".into()),
        (
            "CITADEL_CORE_URL".into(),
            "https://core.example.com:8444".into(),
        ),
    ])
}

fn parse(
    values: &BTreeMap<String, String>,
) -> Result<AgentConfig, citadel_agent::config::ConfigError> {
    AgentConfig::from_lookup(|key| values.get(key).cloned())
}

fn swarm() -> BTreeMap<String, String> {
    let mut values = edge();
    values.insert("CITADEL_EDGE_AGENT_PROFILE".into(), "swarm-node".into());
    for (key, value) in [
        ("CITADEL_EDGE_BOOTSTRAP_FILE", "/run/secrets/bootstrap"),
        ("CITADEL_PLATFORM_ID", "platform-id"),
        ("CITADEL_SWARM_SERVICE_ID", "service-id"),
        ("CITADEL_SWARM_TASK_ID", "task-id"),
        ("CITADEL_SWARM_NODE_ID", "node-id"),
        ("CITADEL_SWARM_NODE_HOSTNAME", "node-hostname"),
        ("CITADEL_SWARM_CLUSTER_ID", "cluster-id"),
    ] {
        values.insert(key.into(), value.into());
    }
    values
}

#[test]
fn all_five_deployment_profiles_parse_the_real_templates_and_bootstrap() {
    for template in [
        include_str!("../../../.env.agent.example"),
        include_str!("../../../.env.build-agent.example"),
    ] {
        let mut values = template_values(template);
        values.extend(direct());
        let config = parse(&values).unwrap();
        assert!(matches!(config.mode, AgentMode::Direct(_)));
        assert_eq!(config.listen_address().to_string(), "0.0.0.0:9000");
    }
    for (template, build) in [
        (include_str!("../../../.env.edge.example"), false),
        (include_str!("../../../.env.edge-build-agent.example"), true),
    ] {
        let config = parse(&template_values(template)).unwrap();
        assert_eq!(config.listen_address().to_string(), "127.0.0.1:9000");
        let AgentMode::Edge(edge) = config.mode else {
            panic!("Edge profile")
        };
        assert_eq!(matches!(edge.profile, EdgeProfile::BuildPool), build);
        assert_eq!(edge.key_path, Path::new("/app/data/edge-agent.key"));
        assert_eq!(
            edge.identity_path,
            Path::new("/app/data/edge-agent.identity.json")
        );
    }
    let config = parse(&swarm()).unwrap();
    assert!(config.listen_address().ip().is_loopback());
    let AgentMode::Edge(edge) = config.mode else {
        panic!("Edge profile")
    };
    let EdgeProfile::SwarmNode(identity) = edge.profile else {
        panic!("Swarm identity")
    };
    assert_eq!(identity.bootstrap_file, Path::new("/run/secrets/bootstrap"));
    assert_eq!(identity.platform_id, "platform-id");
    assert_eq!(identity.service_id, "service-id");
    assert_eq!(identity.task_id, "task-id");
    assert_eq!(identity.node_id, "node-id");
    assert_eq!(identity.node_hostname, "node-hostname");
    assert_eq!(identity.cluster_id, "cluster-id");
}

#[test]
fn only_case_insensitive_exact_edge_selects_outbound_mode() {
    for mode in ["", "direct", "build", "edge-build", " edge ", "unknown"] {
        let mut values = direct();
        values.insert("CITADEL_AGENT_MODE".into(), mode.into());
        assert!(matches!(parse(&values).unwrap().mode, AgentMode::Direct(_)));
    }
    let mut values = edge();
    values.insert("CITADEL_AGENT_MODE".into(), "EdGe".into());
    values.insert(
        "CITADEL_EDGE_AGENT_PROFILE".into(),
        " EDGE-BUILD-AGENT ".into(),
    );
    let AgentMode::Edge(edge) = parse(&values).unwrap().mode else {
        panic!("Edge")
    };
    assert!(matches!(edge.profile, EdgeProfile::BuildPool));
}

#[test]
fn invalid_configuration_fails_with_field_name_without_echoing_credentials() {
    for (base, key, value) in [
        (direct(), "HUB_PUBLIC_KEY", ""),
        (direct(), "HUB_PUBLIC_KEY", "private-secret-invalid-base64"),
        (direct(), "HUB_PUBLIC_KEY", "c2hvcnQ="),
        (direct(), "CITADEL_AGENT_PORT", "0"),
        (direct(), "CITADEL_AGENT_PORT", "65536"),
        (direct(), "CITADEL_AGENT_PORT", "-1"),
        (direct(), "CITADEL_AGENT_PORT", "port"),
        (direct(), "CITADEL_AGENT_TLS_MODE", "0"),
        (direct(), "CITADEL_AGENT_TLS_MODE", "proxy"),
        (edge(), "CITADEL_EDGE_AGENT_PROFILE", "build"),
        (edge(), "CITADEL_EDGE_AGENT_KEY_PATH", ""),
        (edge(), "CITADEL_EDGE_IDENTITY_PATH", " "),
        (
            edge(),
            "CITADEL_CORE_URL",
            "https://user:private-secret@core.example.com",
        ),
        (edge(), "CITADEL_CORE_URL", "ftp://core.example.com"),
        (edge(), "CITADEL_CORE_URL", "https://core.example.com/path"),
        (
            edge(),
            "CITADEL_CORE_URL",
            "https://core.example.com/?query=1",
        ),
        (
            edge(),
            "CITADEL_CORE_URL",
            "https://core.example.com/#fragment",
        ),
    ] {
        let mut values = base;
        values.insert(key.into(), value.into());
        let error = parse(&values).err().expect("must reject invalid setting");
        assert_eq!(error.variable, key);
        assert!(!error.to_string().contains("private-secret"));
    }
    for port in ["1", "65535", " 9001 ", " "] {
        let mut values = direct();
        values.insert("CITADEL_AGENT_PORT".into(), port.into());
        assert!(parse(&values).is_ok());
    }
}

#[test]
fn tls_settings_are_validated_only_for_direct_mode() {
    for field in [
        "CITADEL_AGENT_TLS_CERTIFICATE_PATH",
        "CITADEL_AGENT_TLS_PRIVATE_KEY_PATH",
    ] {
        let mut values = direct();
        values.insert(field.into(), "/tmp/tls.pem".into());
        assert!(parse(&values).is_err());
        values.insert("CITADEL_AGENT_TLS_MODE".into(), "Direct".into());
        assert!(parse(&values).is_err(), "both files required");
    }
    let mut values = direct();
    values.extend([
        ("CITADEL_AGENT_TLS_MODE".into(), " direct ".into()),
        (
            "CITADEL_AGENT_TLS_CERTIFICATE_PATH".into(),
            "/tmp/cert.pem".into(),
        ),
        (
            "CITADEL_AGENT_TLS_PRIVATE_KEY_PATH".into(),
            "/tmp/key.pem".into(),
        ),
    ]);
    let AgentMode::Direct(config) = parse(&values).unwrap().mode else {
        panic!("Direct")
    };
    assert!(config.tls.is_some());
    values.extend(edge());
    values.insert("HUB_PUBLIC_KEY".into(), "invalid-but-unused".into());
    values.insert("CITADEL_AGENT_TLS_MODE".into(), "invalid-but-unused".into());
    assert!(matches!(parse(&values).unwrap().mode, AgentMode::Edge(_)));
}

#[test]
fn edge_preserves_enrollment_and_identity_inputs_without_requiring_a_fresh_token() {
    assert!(
        parse(&edge()).is_ok(),
        "an enrolled Agent can restart without a token"
    );
    let mut values = edge();
    values.extend([
        (
            "CITADEL_EDGE_ENROLLMENT_TOKEN".into(),
            "  enrollment-secret  ".into(),
        ),
        (
            "CITADEL_EDGE_AGENT_KEY_PATH".into(),
            "/data/build.key".into(),
        ),
        (
            "CITADEL_EDGE_IDENTITY_PATH".into(),
            "/data/build.identity.json".into(),
        ),
        (
            "CITADEL_EDGE_CORE_CA_CERTIFICATE_PATH".into(),
            " /certs/ca.pem ".into(),
        ),
    ]);
    let AgentMode::Edge(config) = parse(&values).unwrap().mode else {
        panic!("Edge")
    };
    assert_eq!(
        config.enrollment_token.as_deref().map(|s| s.as_str()),
        Some("enrollment-secret")
    );
    assert_eq!(config.key_path, Path::new("/data/build.key"));
    assert_eq!(config.identity_path, Path::new("/data/build.identity.json"));
    assert_eq!(
        config.core_ca_path.as_deref(),
        Some(Path::new("/certs/ca.pem"))
    );
    values.insert("CITADEL_CORE_URL".into(), "http://core.example.com".into());
    assert_eq!(
        parse(&values).err().unwrap().variable,
        "CITADEL_EDGE_CORE_CA_CERTIFICATE_PATH"
    );
}

#[test]
fn swarm_profile_requires_every_injected_identity_field() {
    for field in swarm().keys().filter(|key| {
        key.starts_with("CITADEL_SWARM_")
            || matches!(
                key.as_str(),
                "CITADEL_PLATFORM_ID" | "CITADEL_EDGE_BOOTSTRAP_FILE"
            )
    }) {
        let mut values = swarm();
        values.remove(field);
        assert_eq!(parse(&values).err().unwrap().variable, field);
    }
}

#[test]
fn docker_endpoint_parsing_preserves_paths_and_rejects_ambiguous_urls() {
    assert_eq!(
        parse(&direct()).unwrap().docker_endpoint,
        DockerEndpoint::Unix("/var/run/docker.sock".into())
    );
    for (endpoint, path) in [
        ("unix:///run/docker.sock", "/run/docker.sock"),
        ("unix:///run/docker%20test.sock", "/run/docker test.sock"),
    ] {
        let mut values = direct();
        values.insert("DOCKER_HOST".into(), endpoint.into());
        assert_eq!(
            parse(&values).unwrap().docker_endpoint,
            DockerEndpoint::Unix(path.into())
        );
    }
    for endpoint in [
        "tcp://docker:2375",
        "http://docker:2375",
        "http://docker",
        "tcp://[::1]:2375",
    ] {
        let mut values = direct();
        values.insert("DOCKER_HOST".into(), endpoint.into());
        assert!(matches!(
            parse(&values).unwrap().docker_endpoint,
            DockerEndpoint::Tcp(_)
        ));
    }
    for endpoint in [
        "tcp://docker",
        "tcp://docker:0",
        "http://docker/path",
        "unix://remote/run/docker.sock",
        "unix:///",
        "npipe://./pipe/docker_engine",
        "https://docker:2376",
        "http://user:password@docker",
        "unix:///run/docker.sock?query=1",
    ] {
        let mut values = direct();
        values.insert("DOCKER_HOST".into(), endpoint.into());
        assert_eq!(
            parse(&values).err().expect(endpoint).variable,
            "DOCKER_HOST"
        );
    }
}

fn template_values(template: &str) -> BTreeMap<String, String> {
    template
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.into(), value.into()))
        .collect()
}
