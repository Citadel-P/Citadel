//! Docker storage totals are expensive; share one refresh per minute per daemon client.
use super::{
    DockerClient,
    projection::{DockerInfo, DockerVolume},
};
use chrono::{DateTime, TimeDelta, Utc};
use citadel_platforms::{RuntimeCapabilityError, RuntimePlatformStats};
use futures_util::{
    FutureExt,
    future::{BoxFuture, WeakShared},
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use std::{collections::HashMap, sync::Arc};
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
    succeeded: Option<Instant>,
}

#[derive(Default)]
pub(super) struct StorageObservations {
    flights: std::sync::Mutex<UsageFlights>,
    pub(super) totals: tokio::sync::Mutex<UsageCache>,
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
    #[serde(default)]
    name: String,
    created_at: Option<String>,
    usage_data: Option<VolumeSize>,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
struct VolumeSize {
    #[serde(skip_serializing_if = "Option::is_none")]
    ref_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    size: Option<i64>,
}
impl DataUsage {
    fn totals(&self) -> Usage {
        Usage {
            images: self.layers_size.filter(|size| *size >= 0),
            volumes: self.volumes.as_ref().and_then(|volumes| {
                volumes.iter().try_fold(0_i64, |total, volume| {
                    total.checked_add(volume.usage_data.as_ref()?.size.filter(|size| *size >= 0)?)
                })
            }),
        }
    }
}

struct StorageSnapshot {
    totals: Usage,
    started: Instant,
    daemon_start_lower_bound: Option<DateTime<Utc>>,
    volumes: HashMap<String, VolumeEvidence>,
}
impl StorageSnapshot {
    fn covers_creation(&self, volume: &DockerVolume) -> bool {
        if self
            .volumes
            .get(&volume.name)
            .and_then(|e| e.same_creation(volume))
            != Some(true)
        {
            return false;
        }
        // Docker can truncate CreatedAt to seconds. A matching timestamp in the
        // scan's own second could identify either creation of a reused name.
        self.daemon_start_lower_bound.is_some_and(|start| {
            DateTime::parse_from_rfc3339(&volume.created_at)
                .is_ok_and(|created| created.timestamp() < start.timestamp())
        })
    }
}

struct VolumeEvidence {
    created_at: Option<String>,
    usage: VolumeSize,
}
impl VolumeEvidence {
    fn same_creation(&self, volume: &DockerVolume) -> Option<bool> {
        let created_at = self.created_at.as_deref().filter(|at| !at.is_empty())?;
        (!volume.created_at.is_empty()).then(|| volume.created_at == created_at)
    }
}
type Scan = BoxFuture<'static, Result<Arc<StorageSnapshot>, Arc<super::DockerError>>>;
#[derive(Default)]
struct UsageFlights {
    all: Option<UsageFlight>,
    volumes: Option<UsageFlight>,
}
struct UsageFlight {
    generation: u64,
    started: Instant,
    scan: WeakShared<Scan>,
}
impl DockerClient {
    pub(super) fn invalidate_volume_usage(&self) {
        *self.storage_usage.flights.lock().unwrap() = UsageFlights::default();
    }

    pub(super) async fn fill_volume_usage<'a>(
        &self,
        volumes: impl IntoIterator<Item = &'a mut DockerVolume>,
        lifecycle: bool,
    ) -> Result<(), super::DockerError> {
        let volumes: Vec<_> = volumes
            .into_iter()
            .filter(|v| v.usage_data.is_none())
            .collect();
        if volumes.is_empty() {
            return Ok(());
        }
        let requested = Instant::now();
        let mut scan = self
            .fresh_usage(false, lifecycle.then_some(requested))
            .await?;
        // External mutations can precede their event. If an older shared scan
        // cannot identify a current volume, read again after the metadata lookup.
        if scan.started < requested && volumes.iter().any(|volume| !scan.covers_creation(volume)) {
            scan = self.fresh_usage(false, Some(requested)).await?;
        }
        for volume in volumes {
            volume.usage_data = scan
                .volumes
                .get(&volume.name)
                .filter(|e| e.same_creation(volume) != Some(false))
                .map(|e| serde_json::json!(e.usage));
        }
        Ok(())
    }

    async fn fresh_usage(
        &self,
        all: bool,
        since: Option<Instant>,
    ) -> Result<Arc<StorageSnapshot>, super::DockerError> {
        let generation = self.daemon_generation();
        let flight = {
            let mut flights = self.storage_usage.flights.lock().unwrap();
            let active = |slot: &Option<UsageFlight>| {
                slot.as_ref()
                    .filter(|f| {
                        f.generation == generation && since.is_none_or(|at| f.started >= at)
                    })
                    .and_then(|f| f.scan.upgrade())
                    .filter(|f| f.peek().is_none())
            };
            if let Some(existing) =
                active(&flights.all).or_else(|| if all { None } else { active(&flights.volumes) })
            {
                existing
            } else {
                let docker = self.clone();
                let started = Instant::now();
                let flight = async move {
                    let _timer =
                        citadel_runtime::runtime_metrics::RuntimeWork::DockerStorage.start();
                    let path = if all {
                        "/system/df?type=image&type=volume"
                    } else {
                        "/system/df?type=volume"
                    };
                    let (bytes, daemon_date) =
                        docker.dated_json_bytes(path).await.map_err(Arc::new)?;
                    // Bound the scan start on the daemon's clock even for remote Docker
                    // endpoints or a response delayed in transit.
                    let daemon_start_lower_bound = daemon_date.and_then(|date| {
                        date.checked_sub_signed(TimeDelta::from_std(started.elapsed()).ok()?)
                    });
                    let data: DataUsage = serde_json::from_slice(&bytes)
                        .map_err(|e| Arc::new(super::DockerError::from(e)))?;
                    if docker.daemon_generation() != generation {
                        return Err(Arc::new(super::DockerError::ProtocolIo(
                            std::io::Error::other(
                                "Docker daemon changed during storage observation",
                            ),
                        )));
                    }
                    let totals = data.totals();
                    let volumes = data
                        .volumes
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|v| {
                            Some((
                                v.name,
                                VolumeEvidence {
                                    created_at: v.created_at,
                                    usage: v.usage_data?,
                                },
                            ))
                        })
                        .collect();
                    Ok(Arc::new(StorageSnapshot {
                        totals,
                        started,
                        daemon_start_lower_bound,
                        volumes,
                    }))
                }
                .boxed()
                .shared();
                let slot = if all {
                    &mut flights.all
                } else {
                    &mut flights.volumes
                };
                *slot = flight.downgrade().map(|scan| UsageFlight {
                    generation,
                    started,
                    scan,
                });
                flight
            }
        };
        flight
            .await
            .map_err(|e| Arc::try_unwrap(e).unwrap_or_else(super::DockerError::Shared))
    }
}
impl DockerClient {
    async fn storage_totals(&self) -> Usage {
        let mut cache = self.storage_usage.totals.lock().await;
        if cache
            .refreshed
            .is_none_or(|at| at.elapsed() >= Duration::from_secs(60))
        {
            // Preserve the last successful totals on a transient error.
            // Cancellation drops this future before advancing the retry deadline.
            match self.fresh_usage(true, None).await {
                Ok(usage) => {
                    cache.usage = usage.totals;
                    cache.succeeded = Some(Instant::now());
                }
                Err(error) => tracing::debug!(%error, "Docker storage usage unavailable"),
            }
            cache.refreshed = Some(Instant::now());
        }
        if cache
            .succeeded
            .is_some_and(|at| at.elapsed() < Duration::from_secs(180))
        {
            cache.usage
        } else {
            Usage::default()
        }
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
        let (image_count, network_count, volume_count) = self
            .sampling_inventory_counts()
            .await
            .map_err(super::runtime::normalize_docker_error)?;
        let usage = self.storage_totals().await;
        let disk = self.read_host_disk(&info.docker_root_dir).await;
        Ok(RuntimePlatformStats {
            image_used_bytes: usage.images,
            volume_used_bytes: usage.volumes,
            image_count,
            network_count,
            volume_count,
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
        client.storage_usage.totals.lock().await.refreshed =
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
        client.storage_usage.totals.lock().await.succeeded =
            Some(Instant::now() - Duration::from_secs(181));
        assert_eq!(
            client.storage_totals().await,
            Usage::default(),
            "failed refreshes must not renew successful usage age"
        );
        client.storage_usage.totals.lock().await.refreshed =
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

#[cfg(all(test, unix))]
mod flight_tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::UnixListener,
        sync::Notify,
    };

    #[tokio::test]
    async fn fresh_scans_share_in_flight_coverage_and_survive_waiter_cancellation() {
        let path = std::env::temp_dir().join(format!("usage-{}.sock", uuid::Uuid::now_v7()));
        let listener = UnixListener::bind(&path).unwrap();
        let (seen, mut requests) = tokio::sync::mpsc::unbounded_channel();
        let releases: Vec<_> = (0..4).map(|_| Arc::new(Notify::new())).collect();
        let server_releases = releases.clone();
        let server = tokio::spawn(async move {
            let mut tasks = tokio::task::JoinSet::new();
            let mut scans = 0;
            while scans < 4 {
                let (mut conn, _) = listener.accept().await.unwrap();
                let mut request = vec![];
                while !request.ends_with(b"\r\n\r\n") {
                    request.push(conn.read_u8().await.unwrap());
                }
                let line = std::str::from_utf8(&request).unwrap();
                let (release, body) = if line.starts_with("GET /version ") {
                    (
                        None,
                        r#"{"ApiVersion":"1.49","MinAPIVersion":"1.41"}"#.to_owned(),
                    )
                } else {
                    seen.send(line.lines().next().unwrap().to_owned()).unwrap();
                    let gate = server_releases[scans].clone();
                    scans += 1;
                    (
                        Some(gate),
                        format!(
                            r#"{{"LayersSize":100,"Volumes":[{{"Name":"v","UsageData":{{"Size":{scans},"RefCount":1}}}}]}}"#
                        ),
                    )
                };
                tasks.spawn(async move{
                    if let Some(gate)=release {gate.notified().await;}
                    let _=conn.write_all(format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await;
                });
            }
            while let Some(result) = tasks.join_next().await {
                result.unwrap();
            }
        });
        let client = DockerClient::new(&path, Duration::from_secs(8)).unwrap();
        let start = |all| {
            let c = client.clone();
            tokio::spawn(async move { c.fresh_usage(all, None).await })
        };
        let owner = start(true);
        assert!(
            requests
                .recv()
                .await
                .unwrap()
                .contains("type=image&type=volume")
        );
        let mut waiter = Box::pin(client.fresh_usage(false, None));
        assert!(futures_util::poll!(&mut waiter).is_pending());
        owner.abort();
        let _ = owner.await;
        releases[0].notify_one();
        assert_eq!(waiter.await.unwrap().volumes["v"].usage.size, Some(1));

        let abandoned = start(false);
        assert!(requests.recv().await.unwrap().contains("df?type=volume "));
        abandoned.abort();
        let _ = abandoned.await;
        releases[1].notify_one();
        let older = start(false);
        assert!(requests.recv().await.unwrap().contains("df?type=volume "));
        let mutation = {
            let c = client.clone();
            tokio::spawn(async move { c.fresh_usage(false, Some(Instant::now())).await })
        };
        assert!(requests.recv().await.unwrap().contains("df?type=volume "));
        releases[3].notify_one();
        releases[2].notify_one();
        assert_eq!(
            mutation.await.unwrap().unwrap().volumes["v"].usage.size,
            Some(4)
        );
        assert_eq!(
            older.await.unwrap().unwrap().volumes["v"].usage.size,
            Some(3)
        );
        server.await.unwrap();
        std::fs::remove_file(path).unwrap();
    }
}

#[cfg(all(test, unix))]
mod volume_lifecycle_tests;
