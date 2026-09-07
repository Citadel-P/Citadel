use serde::Serialize;
use std::collections::BTreeMap;

/// Public installation instructions. The signing private key never enters this model.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSetupView {
    pub hub_public_key: String,
    pub environment: BTreeMap<String, String>,
    pub agent_image: String,
    pub docker_run_command: String,
    pub requires_tls: bool,
}

impl AgentSetupView {
    pub fn new(hub_public_key: String, agent_image: String, requires_tls: bool) -> Self {
        let mut environment = BTreeMap::from([("HUB_PUBLIC_KEY".into(), hub_public_key.clone())]);
        if requires_tls {
            environment.extend([
                ("CITADEL_AGENT_TLS_MODE".into(), "Direct".into()),
                (
                    "CITADEL_AGENT_TLS_CERTIFICATE_PATH".into(),
                    "/etc/citadel/tls/agent-fullchain.pem".into(),
                ),
                (
                    "CITADEL_AGENT_TLS_PRIVATE_KEY_PATH".into(),
                    "/etc/citadel/tls/agent-key.pem".into(),
                ),
            ]);
        }
        let mut lines = vec![
            "docker run -d".to_owned(),
            "  --name citadel-agent".into(),
            "  --restart=always".into(),
            "  --label com.citadel.system=true".into(),
            "  --label com.citadel.system-role=agent".into(),
            "  -p 9000:9000".into(),
            "  -v /var/run/docker.sock:/var/run/docker.sock".into(),
            "  -v /:/host:ro".into(),
        ];
        if requires_tls {
            lines.push("  -v /path/to/agent-tls:/etc/citadel/tls:ro".into());
        }
        for (key, value) in &environment {
            lines.push(format!("  -e {key}=\"{}\"", quote(value)));
        }
        lines.push(format!("  \"{}\"", quote(&agent_image)));
        let docker_run_command = lines.join(" \\\n");
        Self {
            hub_public_key,
            environment,
            agent_image,
            docker_run_command,
            requires_tls,
        }
    }
}

fn quote(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`")
}

#[cfg(test)]
mod tests {
    use super::*;

    // Ports GetAgentSetupTests.Handle_ShouldReturnRegularAgentSetupWithoutPrivateKeyMaterial.
    #[test]
    fn regular_agent_setup_preserves_the_dotnet_ui_contract() {
        let setup = AgentSetupView::new(
            "public-key".into(),
            "registry.example.com/citadel-agent:1.2.3".into(),
            false,
        );
        assert_eq!(setup.environment["HUB_PUBLIC_KEY"], "public-key");
        assert!(!setup.requires_tls);
        for expected in [
            "-p 9000:9000",
            "-v /var/run/docker.sock:/var/run/docker.sock",
            "-v /:/host:ro",
            "-e HUB_PUBLIC_KEY=\"public-key\"",
        ] {
            assert!(setup.docker_run_command.contains(expected));
        }
        for excluded in [
            "PRIVATE_KEY=",
            "BEGIN PRIVATE KEY",
            "CITADEL_EDGE_ENROLLMENT_TOKEN",
            "CITADEL_AGENT_TLS_MODE",
        ] {
            assert!(!setup.docker_run_command.contains(excluded));
        }
        let json = serde_json::to_value(&setup).unwrap();
        for key in [
            "hubPublicKey",
            "environment",
            "agentImage",
            "dockerRunCommand",
            "requiresTls",
        ] {
            assert!(json.get(key).is_some());
        }
    }

    #[test]
    fn tls_setup_includes_certificate_paths_but_not_certificate_contents() {
        let setup = AgentSetupView::new("public-key".into(), "citadel-agent:1.2.3".into(), true);
        assert!(setup.requires_tls);
        assert_eq!(setup.environment["CITADEL_AGENT_TLS_MODE"], "Direct");
        assert!(
            setup
                .docker_run_command
                .contains("-v /path/to/agent-tls:/etc/citadel/tls:ro")
        );
        assert!(!setup.docker_run_command.contains("BEGIN PRIVATE KEY"));
    }
}
