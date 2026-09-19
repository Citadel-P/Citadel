use crate::{
    agent::AgentClient,
    edge::{EdgeRegistry, EdgeTarget},
};
use citadel_builds::{
    BuildAgentPool, BuildError, BuildPoolCheck, BuildPoolChecker, BuildPoolTarget, pool_target,
};
use citadel_contracts::citadel::{edge::v1::EdgeCommandKind, images::v1::CheckBuildHostResponse};
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;

pub struct AgentBuildPoolChecker {
    pub agent: Option<AgentClient>,
    pub edge: EdgeRegistry,
}

impl BuildPoolChecker for AgentBuildPoolChecker {
    fn check<'a>(
        &'a self,
        pool: &'a BuildAgentPool,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<BuildPoolCheck, BuildError>> {
        Box::pin(async move {
            let capabilities = match pool_target(&pool.provider_spec)? {
                BuildPoolTarget::Edge => {
                    let session = self
                        .edge
                        .get(&EdgeTarget::build_pool(pool.id))
                        .map_err(|error| BuildError::Validation(error.to_string()))?;
                    crate::agent_execution::unary::<_, CheckBuildHostResponse>(
                        &session,
                        EdgeCommandKind::ImageCheckBuildHost,
                        (),
                        cancellation,
                    )
                    .await
                }
                BuildPoolTarget::Inbound(endpoint) => {
                    let configured = self.agent.as_ref().ok_or_else(|| {
                        BuildError::Validation("Signed Agent transport is not configured.".into())
                    })?;
                    let client = configured
                        .for_address(&endpoint, cancellation)
                        .await
                        .map_err(|error| BuildError::Validation(error.to_string()))?;
                    client.check_build_host(cancellation).await
                }
            }
            .map_err(|error| BuildError::Validation(error.to_string()))?;
            Ok(capability_result(capabilities))
        })
    }
}

fn capability_result(info: CheckBuildHostResponse) -> BuildPoolCheck {
    if !info.available {
        return BuildPoolCheck {
            ready: false,
            message:
                "Citadel Agent can be reached, but Docker build capabilities are not available."
                    .into(),
        };
    }
    let mut parts = vec![if info.docker_version.is_empty() {
        "Docker".into()
    } else {
        format!("Docker {}", info.docker_version)
    }];
    if !info.api_version.is_empty() {
        parts.push(format!("API {}", info.api_version));
    }
    let platform = [info.operating_system, info.architecture]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/");
    if !platform.is_empty() {
        parts.push(platform);
    }
    if !info.build_kit_version.is_empty() {
        parts.push(format!("BuildKit {}", info.build_kit_version));
    }
    BuildPoolCheck {
        ready: true,
        message: parts.join(" - "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn capability_messages_match_dotnet_with_and_without_buildkit() {
        let mut info = CheckBuildHostResponse {
            available: true,
            docker_version: "28.0.0".into(),
            api_version: "1.49".into(),
            operating_system: "linux".into(),
            architecture: "amd64".into(),
            build_kit_version: "v0.20.2".into(),
        };
        assert_eq!(
            capability_result(info.clone()).message,
            "Docker 28.0.0 - API 1.49 - linux/amd64 - BuildKit v0.20.2"
        );
        info.build_kit_version.clear();
        assert_eq!(
            capability_result(info.clone()).message,
            "Docker 28.0.0 - API 1.49 - linux/amd64"
        );
        info.available = false;
        assert!(!capability_result(info).ready);
    }
}
