//! Statistics discovery needs only identity and state, not inventory metadata.
use super::{DockerClient, DockerError, MAX_JSON_BODY_BYTES, bounded_body};
use citadel_docker_api::models::container_summary::State;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(in crate::client) struct SamplingContainer {
    #[serde(rename = "Id")]
    pub id: Option<String>,
    #[serde(rename = "State")]
    pub state: Option<State>,
}

impl DockerClient {
    pub(in crate::client) async fn list_sampling_containers(
        &self,
    ) -> Result<Vec<SamplingContainer>, DockerError> {
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::DockerContainerList.start();
        let version = self.negotiated_version().await?;
        let response = self
            .client
            .get(format!(
                "{}/v{version}/containers/json?all=true",
                self.base_url
            ))
            .timeout(self.request_timeout)
            .send()
            .await?;
        let response = self.checked_response(response).await?;
        Ok(serde_json::from_slice(
            &bounded_body(response, MAX_JSON_BODY_BYTES).await?,
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use citadel_docker_api::models::ContainerSummary;

    #[test]
    fn discovery_retains_generated_identity_and_state_semantics() {
        for value in [
            r#"{}"#,
            r#"{"Id":null,"State":null}"#,
            r#"{"Id":"example","State":"running","Labels":{"unused":"value"},"Future":{"nested":[1,2]}}"#,
            r#"{"Id":12,"State":"running"}"#,
            r#"{"State":"unknown"}"#,
            r#"{"State":false}"#,
            r#"{"Id":"truncated""#,
        ] {
            let narrow = serde_json::from_str::<SamplingContainer>(value);
            let full = serde_json::from_str::<ContainerSummary>(value);
            assert_eq!(narrow.is_ok(), full.is_ok(), "{value}");
            if let (Ok(narrow), Ok(full)) = (narrow, full) {
                assert_eq!(narrow.id, full.id);
                assert_eq!(narrow.state, full.state);
            }
        }
    }
}
