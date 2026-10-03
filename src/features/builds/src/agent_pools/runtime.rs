use crate::BuildAgentPool;
use crate::BuildError;
use futures_util::future::BoxFuture;
use serde::Deserialize;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

#[derive(Debug, PartialEq, Eq)]
pub enum BuildPoolTarget {
    Inbound(String),
    Edge,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SelfManagedSpec {
    #[serde(default, alias = "Endpoint")]
    endpoint: Option<String>,
    #[serde(default, alias = "ConnectionMode")]
    connection_mode: Option<String>,
    #[serde(default, alias = "Architecture")]
    architecture: Option<String>,
    #[serde(default, alias = "MaxWorkers")]
    max_workers: Option<i32>,
}

pub fn pool_target(spec: &Value) -> Result<BuildPoolTarget, BuildError> {
    if spec.get("$type").and_then(Value::as_str) != Some("SelfManagedVm") {
        return Err(BuildError::Validation(
            "Only self-managed Citadel Agent build pools can be tested or used for Builds.".into(),
        ));
    }
    let spec: SelfManagedSpec = serde_json::from_value(spec.clone()).map_err(|_| {
        BuildError::Validation("Invalid self-managed Build Pool configuration.".into())
    })?;
    if !matches!(
        spec.architecture.as_deref().unwrap_or("Amd64"),
        "Amd64" | "Arm64"
    ) || !(1..=100).contains(&spec.max_workers.unwrap_or(1))
    {
        return Err(BuildError::Validation(
            "Invalid Build Pool architecture or worker count.".into(),
        ));
    }
    match spec.connection_mode.as_deref().unwrap_or("InboundAgent") {
        "EdgeAgent" => Ok(BuildPoolTarget::Edge),
        "InboundAgent" => {
            let endpoint = spec.endpoint.ok_or_else(|| {
                BuildError::Validation("Self-managed VM endpoint is required.".into())
            })?;
            let uri = url::Url::parse(&endpoint).map_err(|_| {
                BuildError::Validation(
                    "Build Agent endpoint must be an absolute HTTP or HTTPS URL.".into(),
                )
            })?;
            if !matches!(uri.scheme(), "http" | "https")
                || uri.host_str().is_none()
                || !uri.username().is_empty()
                || uri.password().is_some()
                || uri.query().is_some()
                || uri.fragment().is_some()
            {
                return Err(BuildError::Validation(
                    "Build Agent endpoint must not contain credentials, a query or a fragment."
                        .into(),
                ));
            }
            Ok(BuildPoolTarget::Inbound(endpoint))
        }
        _ => Err(BuildError::Validation(
            "Invalid Build Pool connection mode.".into(),
        )),
    }
}

pub struct BuildPoolCheck {
    pub ready: bool,
    pub message: String,
}

pub trait BuildPoolChecker: Send + Sync {
    fn check<'a>(
        &'a self,
        pool: &'a BuildAgentPool,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<BuildPoolCheck, BuildError>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn self_managed_targets_match_dotnet_defaults_and_edge_mode() {
        assert_eq!(
            pool_target(&json!({"$type":"SelfManagedVm","endpoint":"https://builder.test"}))
                .unwrap(),
            BuildPoolTarget::Inbound("https://builder.test".into())
        );
        assert_eq!(
            pool_target(
                &json!({"$type":"SelfManagedVm","connectionMode":"EdgeAgent","endpoint":null})
            )
            .unwrap(),
            BuildPoolTarget::Edge
        );
        assert_eq!(
            pool_target(&json!({"$type":"SelfManagedVm","ConnectionMode":"EdgeAgent"})).unwrap(),
            BuildPoolTarget::Edge
        );
    }

    #[test]
    fn unsupported_providers_and_invalid_targets_fail_before_execution() {
        for spec in [
            json!({"$type":"AwsEc2"}),
            json!({"$type":"GenericEdge"}),
            json!({"$type":"HetznerCloud"}),
            json!({"$type":"SelfManagedVm"}),
            json!({"$type":"SelfManagedVm","endpoint":"https://user:password@builder.test"}),
            json!({"$type":"SelfManagedVm","endpoint":"file:///tmp/docker.sock"}),
            json!({"$type":"SelfManagedVm","connectionMode":"EdgeAgent","maxWorkers":0}),
            json!({"$type":"SelfManagedVm","connectionMode":"wrong"}),
        ] {
            assert!(pool_target(&spec).is_err());
        }
    }
}
