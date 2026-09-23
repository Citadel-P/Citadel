//! Docker storage totals are expensive; share one refresh per minute per daemon client.
use super::{DockerClient, projection::DockerInfo};
use citadel_platforms::{RuntimeCapabilityError, RuntimePlatformStats};
use serde::Deserialize;
use std::time::Duration;
use tokio::time::Instant;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Usage {
    images: Option<i64>,
    volumes: Option<i64>,
}

#[derive(Default)]
pub(super) struct UsageCache {
    refreshed: Option<Instant>,
    usage: Usage,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct DataUsage {
    layers_size: Option<i64>,
    volumes: Option<Vec<VolumeUsage>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct VolumeUsage {
    usage_data: Option<VolumeSize>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct VolumeSize {
    size: Option<i64>,
}
impl From<citadel_docker_api::models::SystemDataUsageResponse> for DataUsage {
    fn from(v: citadel_docker_api::models::SystemDataUsageResponse) -> Self {
        Self {
            layers_size: v.layers_size,
            volumes: v.volumes.map(|vs| {
                vs.into_iter()
                    .map(|v| VolumeUsage {
                        usage_data: v.usage_data.flatten().map(|u| VolumeSize { size: u.size }),
                    })
                    .collect()
            }),
        }
    }
}
impl DataUsage {
    fn totals(self) -> Usage {
        Usage {
            images: self.layers_size.filter(|size| *size >= 0),
            volumes: self.volumes.and_then(|volumes| {
                volumes.into_iter().try_fold(0_i64, |total, volume| {
                    total.checked_add(volume.usage_data?.size.filter(|size| *size >= 0)?)
                })
            }),
        }
    }
}

impl DockerClient {
    async fn storage_totals(&self) -> Usage {
        let mut cache = self.storage_usage.lock().await;
        if cache
            .refreshed
            .is_none_or(|at| at.elapsed() >= Duration::from_secs(60))
        {
            // Preserve the last successful totals on a transient error.
            // Cancellation drops this future before advancing the retry deadline.
            match self
                .system_data_usage(vec!["image".into(), "volume".into()])
                .await
            {
                Ok(usage) => cache.usage = DataUsage::from(usage).totals(),
                Err(error) => tracing::debug!(%error, "Docker storage usage unavailable"),
            }
            cache.refreshed = Some(Instant::now());
        }
        cache.usage
    }

    pub async fn platform_stats(&self) -> Result<RuntimePlatformStats, RuntimeCapabilityError> {
        let info = self
            .info()
            .await
            .map_err(super::runtime::normalize_docker_error)?;
        self.platform_stats_from_info(info).await
    }

    pub(super) async fn platform_stats_from_info(
        &self,
        info: DockerInfo,
    ) -> Result<RuntimePlatformStats, RuntimeCapabilityError> {
        let containers = self
            .list_containers(true)
            .await
            .map_err(super::runtime::normalize_docker_error)?;
        self.platform_stats_with_counts(
            info,
            super::runtime::ContainerCounts::from_list(&containers),
        )
        .await
    }

    pub(super) async fn platform_stats_with_counts(
        &self,
        info: DockerInfo,
        counts: super::runtime::ContainerCounts,
    ) -> Result<RuntimePlatformStats, RuntimeCapabilityError> {
        // Slow metadata is sequential: one external request at a time, independent of stats fan-out.
        let images = self.list_images().await;
        let networks = self.list_networks().await;
        let volumes = self.volumes().await;
        let usage = self.storage_totals().await;
        let disk = self.read_host_disk(&info.docker_root_dir).await;
        Ok(RuntimePlatformStats {
            image_used_bytes: usage.images,
            volume_used_bytes: usage.volumes,
            image_count: i64::try_from(
                images
                    .map_err(super::runtime::normalize_docker_error)?
                    .len(),
            )
            .unwrap_or(i64::MAX),
            network_count: i32::try_from(
                networks
                    .map_err(super::runtime::normalize_docker_error)?
                    .len(),
            )
            .unwrap_or(i32::MAX),
            volume_count: i32::try_from(
                volumes
                    .map_err(super::runtime::normalize_docker_error)?
                    .volumes
                    .len(),
            )
            .unwrap_or(i32::MAX),
            mem_total: i64::try_from(info.memory_total).unwrap_or(i64::MAX),
            container_count: counts.total,
            containers_running: counts.running,
            containers_paused: counts.paused,
            containers_stopped: counts.stopped,
            disk_used_bytes: disk.map(|d| d.used_bytes),
            disk_total_bytes: disk.map(|d| d.total_bytes),
            disk_usage: disk.map(|d| d.usage_percent),
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn storage_totals_preserve_unknown_values_and_reject_partial_or_overflowing_sizes() {
        for (json, expected) in [
            (
                r#"{"LayersSize":2048,"Volumes":[{"UsageData":{"Size":1024}},{"UsageData":{"Size":3072}}]}"#,
                Usage {
                    images: Some(2048),
                    volumes: Some(4096),
                },
            ),
            (
                r#"{"LayersSize":0,"Volumes":[]}"#,
                Usage {
                    images: Some(0),
                    volumes: Some(0),
                },
            ),
            (r#"{"LayersSize":-1,"Volumes":null}"#, Usage::default()),
            (
                r#"{"Volumes":[{"UsageData":{"Size":1}},{"UsageData":null}]}"#,
                Usage::default(),
            ),
            (
                r#"{"Volumes":[{"UsageData":{"Size":-1}}]}"#,
                Usage::default(),
            ),
            (
                r#"{"Volumes":[{"UsageData":{"Size":9223372036854775807}},{"UsageData":{"Size":1}}]}"#,
                Usage::default(),
            ),
        ] {
            assert_eq!(
                serde_json::from_str::<DataUsage>(json).unwrap().totals(),
                expected,
                "{json}"
            );
        }
    }
    #[test]
    fn docker_image_inventory_accepts_null_optional_collections() {
        let wire: citadel_docker_api::models::ImageSummary = serde_json::from_str(
            r#"{"Id":"sha256:alpine","Labels":null,"RepoTags":null,"RepoDigests":null}"#,
        )
        .unwrap();
        let image: super::super::projection::ImageSummary = wire.try_into().unwrap();
        assert!(image.labels.is_empty());
        assert!(image.repo_tags.is_empty());
        assert!(image.repo_digests.is_empty());
    }
}

#[cfg(all(test, unix))]
mod cache_tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    #[tokio::test]
    async fn storage_cache_coalesces_refreshes_retries_failures_and_expires_unknown_totals() {
        let path =
            std::env::temp_dir().join(format!("citadel-usage-{}.sock", uuid::Uuid::now_v7()));
        let listener = tokio::net::UnixListener::bind(&path).unwrap();
        let server = tokio::spawn(async move {
            for (request, status, body) in [
                (
                    "GET /version ",
                    "200 OK",
                    r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#,
                ),
                (
                    "GET /v1.49/system/df?type=image&type=volume ",
                    "200 OK",
                    r#"{"LayersSize":2048,"Volumes":[{"Name":"data","UsageData":{"Size":4096}}]}"#,
                ),
                (
                    "GET /v1.49/system/df?type=image&type=volume ",
                    "500 Internal Server Error",
                    r#"{"message":"temporary failure"}"#,
                ),
                (
                    "GET /v1.49/system/df?type=image&type=volume ",
                    "200 OK",
                    r#"{"LayersSize":null,"Volumes":null}"#,
                ),
            ] {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut bytes = Vec::new();
                let mut buffer = [0; 2048];
                while !bytes.windows(4).any(|w| w == b"\r\n\r\n") {
                    let n = socket.read(&mut buffer).await.unwrap();
                    assert!(n > 0 && bytes.len() < 8192);
                    bytes.extend_from_slice(&buffer[..n]);
                }
                assert!(String::from_utf8_lossy(&bytes).starts_with(request));
                socket.write_all(format!("HTTP/1.1 {status}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
            }
        });
        let client = DockerClient::new(&path, Duration::from_secs(2)).unwrap();
        let cloned = client.clone();
        let (first, second) = tokio::join!(client.storage_totals(), cloned.storage_totals());
        let expected = Usage {
            images: Some(2048),
            volumes: Some(4096),
        };
        assert_eq!(first, expected);
        assert_eq!(second, expected);
        client.storage_usage.lock().await.refreshed =
            Some(Instant::now() - Duration::from_secs(61));
        assert_eq!(
            client.storage_totals().await,
            expected,
            "failed refresh retains last successful totals"
        );
        assert_eq!(
            client.storage_totals().await,
            expected,
            "failed refresh also has a retry delay"
        );
        client.storage_usage.lock().await.refreshed =
            Some(Instant::now() - Duration::from_secs(61));
        assert_eq!(
            client.storage_totals().await,
            Usage::default(),
            "successful unknown observations clear prior totals"
        );
        server.await.unwrap();
        std::fs::remove_file(path).unwrap();
    }
}
