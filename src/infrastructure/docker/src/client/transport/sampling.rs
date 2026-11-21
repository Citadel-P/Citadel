//! Bounded projections for statistics discovery and platform metadata.
use super::{DockerClient, DockerError, MAX_JSON_BODY_BYTES, bounded_body};
use citadel_docker_api::models::container_summary::State;
use citadel_runtime::runtime_metrics::RuntimeWork;
use serde::Deserialize;
use std::borrow::Cow;

#[derive(Debug, Deserialize)]
pub(in crate::client) struct SamplingContainer {
    #[serde(rename = "Id")]
    pub id: Option<String>,
    #[serde(rename = "State")]
    pub state: Option<State>,
    #[serde(rename = "Status")]
    pub status: Option<String>,
}

// Leave labels, tags, mountpoints, endpoints, and other inventory fields in the
// response buffer. Image IDs and volume names are required by the wire schema.
#[derive(Deserialize)]
struct ImageEntry<'a> {
    #[serde(rename = "Id", borrow)]
    _id: Cow<'a, str>,
}

#[derive(Deserialize)]
struct NetworkEntry {}

#[derive(Deserialize)]
struct VolumeEntry<'a> {
    #[serde(rename = "Name", borrow)]
    _name: Cow<'a, str>,
}

#[derive(Deserialize)]
struct VolumeList<'a> {
    #[serde(rename = "Volumes", borrow)]
    volumes: Option<Vec<VolumeEntry<'a>>>,
}

impl DockerClient {
    pub(in crate::client) async fn list_sampling_containers(
        &self,
    ) -> Result<Vec<SamplingContainer>, DockerError> {
        let _iteration = RuntimeWork::DockerContainerList.start();
        let bytes = self.bounded_json_bytes("/containers/json?all=true").await?;
        Ok(serde_json::from_slice(&bytes)?)
    }

    pub(in crate::client) async fn sampling_inventory_counts(
        &self,
    ) -> Result<(i64, i32, i32), DockerError> {
        let image_count = {
            let _list = RuntimeWork::DockerList.start();
            let _timer = RuntimeWork::DockerImageList.start();
            let bytes = self.bounded_json_bytes("/images/json?all=true").await?;
            serde_json::from_slice::<Vec<ImageEntry>>(&bytes)?.len()
        };
        let network_count = {
            let _timer = RuntimeWork::DockerNetworkList.start();
            let bytes = self.bounded_json_bytes("/networks").await?;
            serde_json::from_slice::<Vec<NetworkEntry>>(&bytes)?.len()
        };
        let volume_count = {
            let _list = RuntimeWork::DockerList.start();
            let _timer = RuntimeWork::DockerVolumeList.start();
            let bytes = self.bounded_json_bytes("/volumes").await?;
            serde_json::from_slice::<VolumeList>(&bytes)?
                .volumes
                .unwrap_or_default()
                .len()
        };
        Ok((
            i64::try_from(image_count).unwrap_or(i64::MAX),
            i32::try_from(network_count).unwrap_or(i32::MAX),
            i32::try_from(volume_count).unwrap_or(i32::MAX),
        ))
    }

    pub(in crate::client) async fn bounded_json_bytes(
        &self,
        path: &str,
    ) -> Result<Vec<u8>, DockerError> {
        let version = self.negotiated_version().await?;
        let response = self
            .client
            .get(format!("{}/v{version}{path}", self.base_url))
            .timeout(self.request_timeout)
            .send()
            .await?;
        let response = self.checked_response(response).await?;
        bounded_body(response, MAX_JSON_BODY_BYTES).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use citadel_docker_api::models::ContainerSummary;

    #[test]
    fn inventory_counts_handle_missing_collections_and_require_resource_identities() {
        for json in [r#"{}"#, r#"{"Volumes":null}"#, r#"{"Volumes":[]}"#] {
            let list: VolumeList = serde_json::from_str(json).unwrap();
            assert!(list.volumes.unwrap_or_default().is_empty());
        }
        assert!(serde_json::from_str::<VolumeList>(r#"{"Volumes":[{}]}"#).is_err());
        assert!(serde_json::from_str::<Vec<ImageEntry>>(r#"[{"Id":null}]"#).is_err());
    }

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
