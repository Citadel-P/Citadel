use super::operation_views::{EdgeEnrollmentView, EdgeInstructionsView};
use crate::api::error::ApiError;
use citadel_platforms::edge_management::{EdgeManagementError, EdgeTarget};
use std::sync::Arc;
use uuid::Uuid;
#[derive(Clone)]
pub struct EdgeHttpContext {
    pub node_agent_policy: citadel_platforms::node_agents::setup::NodeAgentSetupPolicy,
    pub store: Arc<dyn citadel_platforms::edge_management::EdgeEnrollmentStore>,
    pub registry: Arc<dyn citadel_platforms::edge_management::EdgeSessionControl>,
    pub core_url: String,
    pub agent_image: String,
    pub node_agent_ca_bundle: Option<Arc<[u8]>>,
}

fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\"'\"'"))
}

impl EdgeHttpContext {
    pub(crate) async fn enrollment(
        &self,
        target: EdgeTarget,
        actor_id: Uuid,
    ) -> Result<EdgeEnrollmentView, ApiError> {
        let (enrollment, token, expires) = self
            .store
            .create_enrollment(&target, actor_id)
            .await
            .map_err(Self::error)?;
        let name = if target.resource_type == 1 {
            "edge-build-agent"
        } else {
            "edge-agent"
        };
        let volume = name.replace('-', "_") + "_data";
        let environment = std::collections::BTreeMap::from([
            ("CITADEL_AGENT_MODE", "edge".to_owned()),
            ("CITADEL_CORE_URL", self.core_url.clone()),
            ("CITADEL_EDGE_ENROLLMENT_TOKEN", token.clone()),
            (
                "CITADEL_EDGE_AGENT_KEY_PATH",
                format!("/app/data/{name}.key"),
            ),
            (
                "CITADEL_EDGE_IDENTITY_PATH",
                format!("/app/data/{name}.identity.json"),
            ),
        ]);
        let mut lines = vec![
            "docker run -d".to_owned(),
            format!("  --name {name}"),
            "  --restart unless-stopped".into(),
            "  -v /var/run/docker.sock:/var/run/docker.sock".into(),
        ];
        if target.resource_type == 0 {
            lines.extend([
                "  -v /:/host:ro".into(),
                "  --label com.citadel.system=true".into(),
                "  --label com.citadel.system-role=edge-agent".into(),
            ]);
        }
        lines.push(format!("  -v {volume}:/app/data"));
        lines.extend(
            environment
                .iter()
                .map(|(key, value)| format!("  -e {}", shell_quote(&format!("{key}={value}")))),
        );
        lines.push(format!("  {}", shell_quote(&self.agent_image)));
        let command = lines.join(" \\\n");
        Ok(EdgeEnrollmentView {
            enrollment_id: enrollment,
            platform_id: target.platform_id,
            token,
            expires_at_utc: expires,
            instructions: EdgeInstructionsView {
                core_url: self.core_url.clone(),
                environment: environment
                    .into_iter()
                    .map(|(key, value)| (key.to_owned(), value))
                    .collect(),
                agent_image: self.agent_image.clone(),
                docker_run_command: command,
            },
        })
    }

    pub(crate) fn error(error: EdgeManagementError) -> ApiError {
        match error {
            EdgeManagementError::NotFound => ApiError::NotFound,
            EdgeManagementError::Unauthorized => ApiError::Conflict(error.to_string()),
            EdgeManagementError::Invalid(message) => ApiError::Validation(message.into()),
            source @ EdgeManagementError::Storage(_) => ApiError::internal(source),
        }
    }
}
