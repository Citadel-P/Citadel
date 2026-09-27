use super::{Runtime, docker_error, filters, strings, text};
use citadel_adapters::connectors::docker::projection::{DockerVolume, VolumeCreateOptions};
use citadel_contracts::citadel::{shared_models::v1::*, volumes::v1::*};
use serde_json::Value;
use tonic::{Request, Response, Status};

#[tonic::async_trait]
impl volume_service_server::VolumeService for Runtime {
    async fn list(
        &self,
        request: Request<ListVolumesRequest>,
    ) -> Result<Response<ListVolumesResponse>, Status> {
        let request = request.into_inner();
        let filter = filters([
            ("dangling", request.dangling.map(|v| v.to_string())),
            ("driver", request.driver),
            ("name", request.name),
        ]);
        let volumes = self
            .docker
            .list_volumes_filtered(filter.as_deref())
            .await
            .map_err(docker_error)?;
        Ok(Response::new(ListVolumesResponse {
            volumes: volumes.into_iter().map(volume).collect(),
        }))
    }
    async fn inspect(
        &self,
        request: Request<InspectVolumeRequest>,
    ) -> Result<Response<VolumeResponse>, Status> {
        let name = request.into_inner().name;
        let (value, containers) = tokio::try_join!(
            self.docker.inspect_volume(&name),
            self.docker.volume_containers(&name)
        )
        .map_err(docker_error)?;
        let mut result = volume(value);
        result.in_use |= !containers.is_empty();
        result.containers = containers
            .into_iter()
            .map(|c| {
                let networks = c
                    .network_settings
                    .as_ref()
                    .and_then(|n| n.networks.as_ref())
                    .into_iter()
                    .flatten()
                    .map(|(name, endpoint)| {
                        (
                            name.clone(),
                            endpoint
                                .network_id
                                .as_ref()
                                .filter(|id| !id.is_empty())
                                .unwrap_or(name)
                                .clone(),
                        )
                    })
                    .collect();
                let name = c
                    .names
                    .as_ref()
                    .and_then(|names| names.first())
                    .cloned()
                    .unwrap_or_default();
                let c = super::containers::summary(c)?;
                Ok(ContainerVolumeResult {
                    id: c.id,
                    name,
                    image: c.image,
                    image_id: c.image_id,
                    state: c.state,
                    networks,
                    ports: c.ports,
                })
            })
            .collect::<Result<_, Status>>()?;
        Ok(Response::new(result))
    }

    async fn create(
        &self,
        request: Request<CreateVolumeRequest>,
    ) -> Result<Response<VolumeResponse>, Status> {
        let request = request.into_inner();
        let input = VolumeCreateOptions {
            name: request.name,
            driver: request.driver,
            labels: request.labels,
            driver_options: request.options,
        };
        Ok(Response::new(volume(
            self.docker
                .create_volume(&input)
                .await
                .map_err(docker_error)?,
        )))
    }
    async fn remove(
        &self,
        request: Request<RemoveVolumeRequest>,
    ) -> Result<Response<RemoveVolumeResponse>, Status> {
        let request = request.into_inner();
        // Validate the entire batch before starting any mutation.
        if request.names.iter().any(|name| name.trim().is_empty()) {
            return Err(Status::invalid_argument("Volume names must not be empty"));
        }
        for name in request.names {
            self.docker
                .delete_volume(&name, request.force)
                .await
                .map_err(docker_error)?;
        }
        Ok(Response::new(RemoveVolumeResponse {}))
    }
}

pub(super) fn volume(value: DockerVolume) -> VolumeResponse {
    let usage_data = value.usage_data.as_ref().map(|data| UsageDataMessage {
        size: Some(data["Size"].as_i64().unwrap_or(-1)),
        ref_count: Some(data["RefCount"].as_i64().unwrap_or(-1)),
    });
    VolumeResponse {
        name: value.name,
        driver: value.driver,
        labels: value.labels,
        options: value.options,
        mountpoint: value.mountpoint,
        created_at: value.created_at,
        scope: value.scope,
        status: value
            .status
            .into_iter()
            .map(|(key, value)| {
                (
                    key,
                    if value.is_null() {
                        String::new()
                    } else {
                        value
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| value.to_string())
                    },
                )
            })
            .collect(),
        in_use: usage_data
            .as_ref()
            .and_then(|data| data.ref_count)
            .is_some_and(|count| count > 0),
        usage_data,
        cluster_volume: value.cluster_volume.as_ref().map(cluster),
        containers: vec![],
    }
}

fn present<T>(value: &Value, map: impl FnOnce(&Value) -> T) -> Option<T> {
    (!value.is_null()).then(|| map(value))
}
fn array<T>(value: &Value, map: impl Fn(&Value) -> T) -> Vec<T> {
    value.as_array().into_iter().flatten().map(map).collect()
}

fn cluster(value: &Value) -> ClusterVolumeMessage {
    ClusterVolumeMessage {
        id: text(value, "ID"),
        created_at: text(value, "CreatedAt"),
        updated_at: text(value, "UpdatedAt"),
        version: present(&value["Version"], |v| VolumVersionMessage {
            index: v["Index"].as_i64(),
        }),
        spec: present(&value["Spec"], |v| VolumeSpecMessage {
            group: text(v, "Group"),
            access_mode: present(&v["AccessMode"], |a| VolumeAccessModeMessage {
                scope: i32::from(a["Scope"].as_str() == Some("multi")),
                sharing: match a["Sharing"].as_str() {
                    Some("readonly") => 1,
                    Some("onewriter") => 2,
                    Some("all") => 3,
                    _ => 0,
                },
                availability: text(a, "Availability"),
                secrets: array(&a["Secrets"], |s| VolumeSecretMessage {
                    key: text(s, "Key"),
                    secret: text(s, "Secret"),
                }),
                capacity_range: present(&a["CapacityRange"], |c| VolumeCapacityRange {
                    required_bytes: c["RequiredBytes"].as_i64(),
                    limit_bytes: c["LimitBytes"].as_i64(),
                }),
            }),
        }),
        info: present(&value["Info"], |v| ClusterVolumeInfoMessage {
            capacity_bytes: v["CapacityBytes"].as_i64(),
            volume_context: strings(&v["VolumeContext"]),
            volume_id: text(v, "VolumeID"),
            accessible_topology: array(&v["AccessibleTopology"], |t| TopologyEntryMessage {
                labels: strings(t),
            }),
        }),
        publish_status: array(&value["PublishStatus"], |v| PublishStatusMessage {
            node_id: text(v, "NodeID"),
            state: text(v, "State"),
            publish_context: strings(&v["PublishContext"]),
        }),
    }
}

#[cfg(test)]
mod status_tests {
    #[test]
    fn null_status_is_empty_in_the_agent_contract() {
        let volume: super::DockerVolume = serde_json::from_value(
            serde_json::json!({"Name":"data","Status":{"missing":null,"text":"ready","number":42}}),
        )
        .unwrap();
        let mapped = super::volume(volume);
        assert_eq!(mapped.status["missing"], "");
        assert_eq!(mapped.status["text"], "ready");
        assert_eq!(mapped.status["number"], "42");
    }
}
