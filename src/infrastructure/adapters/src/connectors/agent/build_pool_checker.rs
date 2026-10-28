use crate::connectors::agent::client::AgentClient;
use crate::connectors::edge::EdgeRegistry;
use crate::connectors::edge::EdgeTarget;
use citadel_builds::{
    BuildAgentPool, BuildError, BuildPoolCheck, BuildPoolChecker, BuildPoolTarget, pool_target,
};
use citadel_contracts::citadel::{edge::v1::EdgeCommandKind, images::v1::CheckBuildHostResponse};
use futures_util::future::BoxFuture;
use tokio_util::sync::CancellationToken;

pub struct AgentBuildPoolChecker {
    pub agent: Option<AgentClient>,
    pub edge: EdgeRegistry,
    pub admission: std::sync::Arc<dyn citadel_builds::BuildAdmissionStore>,
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
                    crate::connectors::agent::execution::unary::<_, CheckBuildHostResponse>(
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
                        .for_address(&endpoint, pool.id, cancellation)
                        .await
                        .map_err(|error| BuildError::Validation(error.to_string()))?;
                    client.check_build_host(cancellation).await
                }
            }
            .map_err(|error| BuildError::Validation(error.to_string()))?;
            let occupancy = self
                .admission
                .executor_occupancy(&capabilities.docker_daemon_id)
                .await?;
            let authority = capabilities.admission_authority_id.clone();
            let effective = capabilities
                .build_capacity
                .min(capabilities.heavy_capacity)
                .min(pool.max_active_builders as u32);
            let mut result = capability_result(capabilities);
            result.message.push_str(&format!(
                " - Effective build capacity {effective} (pool policy {})",
                pool.max_active_builders
            ));
            if occupancy.reserved > 0 {
                result.message.push_str(&format!(
                    " - Core reservations {} ({} awaiting cleanup verification)",
                    occupancy.reserved, occupancy.uncertain
                ));
                if occupancy
                    .authority_id
                    .is_some_and(|id| id.to_string() != authority)
                {
                    result
                        .message
                        .push_str(" - Previous Agent execution requires cleanup verification");
                }
            }
            Ok(result)
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
    if info.build_capacity > 0 {
        parts.push(format!(
            "Build slots {}/{} occupied",
            info.occupied_builds, info.build_capacity
        ));
        if info.uncertain_builds > 0 {
            parts.push(format!(
                "{} awaiting cleanup verification",
                info.uncertain_builds
            ));
        }
    }
    if info.data_capacity > 0 {
        parts.push(format!(
            "Data slots {}/{} occupied",
            info.occupied_data, info.data_capacity
        ));
    }
    if info.heavy_capacity > 0 {
        parts.push(format!("Shared heavy-work limit {}", info.heavy_capacity));
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
            ..Default::default()
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
