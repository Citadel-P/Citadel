//! Finite Docker operations use generated tag clients and the shared negotiated configuration.
use super::projection::*;
use super::transport::validate_identifier;
use super::{DockerClient, DockerError};
use citadel_docker_api::{
    apis::{
        self, config_api::*, configuration::Configuration, container_api::*, distribution_api::*,
        exec_api::*, image_api::*, network_api::*, node_api::*, secret_api::*, service_api::*,
        swarm_api::*, system_api::*, task_api::*, volume_api::*,
    },
    models,
};
use citadel_runtime::runtime_metrics::RuntimeWork;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use std::sync::Arc;

fn body<T: DeserializeOwned>(value: &(impl Serialize + ?Sized)) -> Result<T, DockerError> {
    Ok(serde_json::from_value(serde_json::to_value(value)?)?)
}
fn document(value: impl Serialize) -> Result<Value, DockerError> {
    Ok(serde_json::to_value(value)?)
}
fn convert_list<T, R: TryFrom<T, Error = DockerError>>(
    items: Vec<T>,
) -> Result<Vec<R>, DockerError> {
    items.into_iter().map(TryInto::try_into).collect()
}

impl DockerClient {
    pub(super) async fn configuration(&self) -> Result<Arc<Configuration>, DockerError> {
        let version = self.negotiated_version().await?;
        let mut cached = self.configuration.write().await;
        if let Some((cached_version, configuration)) = &*cached
            && *cached_version == version
        {
            return Ok(configuration.clone());
        }
        let configuration = Arc::new(Configuration {
            base_path: format!("{}/v{version}", self.base_url),
            client: self.client.clone(),
            request_timeout: Some(self.request_timeout),
            max_response_bytes: 16 * 1024 * 1024,
            max_error_bytes: 64 * 1024,
            user_agent: None,
            basic_auth: None,
            oauth_access_token: None,
            bearer_access_token: None,
            api_key: None,
        });
        *cached = Some((version, configuration.clone()));
        Ok(configuration)
    }

    pub(super) async fn api_result<T, E>(
        &self,
        result: Result<T, apis::Error<E>>,
    ) -> Result<T, DockerError> {
        match result {
            Ok(value) => Ok(value),
            Err(apis::Error::ResponseRead { status, source }) => {
                if status == reqwest::StatusCode::BAD_REQUEST {
                    self.invalidate_version().await;
                }
                Err(DockerError::Transport(source))
            }
            Err(apis::Error::Reqwest(error)) => Err(DockerError::Transport(error)),
            Err(apis::Error::Serde(error)) => Err(DockerError::InvalidJson(error)),
            Err(apis::Error::ResponseTooLarge { limit, status }) => {
                if status == reqwest::StatusCode::BAD_REQUEST {
                    self.invalidate_version().await;
                }
                Err(DockerError::ResponseTooLarge { limit })
            }
            Err(apis::Error::Io(error)) => Err(DockerError::ProtocolIo(error)),
            Err(apis::Error::ResponseError(response)) => {
                if response.status == reqwest::StatusCode::BAD_REQUEST {
                    self.invalidate_version().await;
                }
                Err(DockerError::Api {
                    status: response.status,
                    message: response.content,
                })
            }
        }
    }

    pub async fn info(&self) -> Result<DockerInfo, DockerError> {
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::DockerInfo.start();
        let info: DockerInfo = self
            .api_result(
                SystemApiClient::new(self.configuration().await?)
                    .system_info()
                    .await,
            )
            .await?
            .try_into()?;
        let changed = {
            let mut previous = self.daemon_id.lock().await;
            let changed = previous.as_ref().is_some_and(|id| id != &info.id);
            *previous = Some(info.id.clone());
            changed
        };
        if changed {
            self.invalidate_daemon().await;
        }
        Ok(info)
    }
    pub async fn list_containers(&self, all: bool) -> Result<Vec<ContainerSummary>, DockerError> {
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::DockerList.start();
        convert_list(
            self.list_container_models(Some(all), None, None, None)
                .await?,
        )
    }

    pub async fn list_container_models(
        &self,
        all: Option<bool>,
        limit: Option<i32>,
        size: Option<bool>,
        filters: Option<&str>,
    ) -> Result<Vec<models::ContainerSummary>, DockerError> {
        let _list = if filters.is_some() {
            RuntimeWork::DockerFilteredContainerList
        } else {
            RuntimeWork::DockerContainerList
        }
        .start();
        self.api_result(
            ContainerApiClient::new(self.configuration().await?)
                .container_list(all, limit, size, filters)
                .await,
        )
        .await
    }
    pub async fn inspect_container(&self, id: &str) -> Result<ContainerInspect, DockerError> {
        validate_identifier(id)?;
        self.api_result(
            ContainerApiClient::new(self.configuration().await?)
                .container_inspect(id, None)
                .await,
        )
        .await?
        .try_into()
    }

    /// Event projection needs list metadata, not the full inspection document.
    /// Match the exact identity: Docker's ID filter also accepts prefixes.
    pub async fn container_event_observation(
        &self,
        id: &str,
    ) -> Result<Option<citadel_platforms::RuntimeContainerSummary>, DockerError> {
        validate_identifier(id)?;
        let filters = serde_json::json!({"id": [id]}).to_string();
        let Some(container) = self
            .list_container_models(Some(true), None, None, Some(&filters))
            .await?
            .into_iter()
            .find(|container| container.id.as_deref() == Some(id))
        else {
            // A disappearing container is not an authoritative deletion event.
            return Ok(None);
        };
        if container.image_id.as_deref().is_none_or(str::is_empty) {
            // Match the .NET compatibility fallback for incomplete summaries.
            let _inspect = RuntimeWork::ContainerEventInspect.start();
            return Ok(Some(super::container_observation(
                self.inspect_container_document(id).await?,
            )?));
        }
        Ok(Some(super::runtime::map_container(container.try_into()?)))
    }
    pub async fn delete_container(
        &self,
        id: &str,
        remove_volumes: bool,
        force: bool,
    ) -> Result<(), DockerError> {
        self.delete_container_with_options(id, remove_volumes, force, false)
            .await
    }
    pub async fn delete_container_with_options(
        &self,
        id: &str,
        v: bool,
        force: bool,
        link: bool,
    ) -> Result<(), DockerError> {
        validate_identifier(id)?;
        self.api_result(
            ContainerApiClient::new(self.configuration().await?)
                .container_delete(id, Some(v), Some(force), Some(link))
                .await,
        )
        .await
    }
    pub async fn create_container<T: Serialize + ?Sized>(
        &self,
        name: &str,
        value: &T,
    ) -> Result<String, DockerError> {
        validate_identifier(name)?;
        Ok(self
            .api_result(
                ContainerApiClient::new(self.configuration().await?)
                    .container_create(body(value)?, Some(name), None)
                    .await,
            )
            .await?
            .id)
    }
    pub async fn start_container(&self, id: &str) -> Result<(), DockerError> {
        self.change_container_state(id, citadel_platforms::containers::ContainerAction::Start)
            .await
    }
    pub async fn change_container_state(
        &self,
        id: &str,
        action: citadel_platforms::containers::ContainerAction,
    ) -> Result<(), DockerError> {
        use citadel_platforms::containers::ContainerAction;
        let (signal, timeout) = match action {
            ContainerAction::Stop => (Some("SIGTERM"), Some(10)),
            ContainerAction::Restart => (Some("SIGINT"), Some(5)),
            _ => (None, None),
        };
        self.change_container_state_with_options(id, action, signal, timeout)
            .await
    }
    pub async fn change_container_state_with_options(
        &self,
        id: &str,
        action: citadel_platforms::containers::ContainerAction,
        signal: Option<&str>,
        timeout: Option<i32>,
    ) -> Result<(), DockerError> {
        use citadel_platforms::containers::ContainerAction::*;
        validate_identifier(id)?;
        let api = ContainerApiClient::new(self.configuration().await?);
        let result = match action {
            Start => self.api_result(api.container_start(id, None).await).await,
            Stop => {
                self.api_result(api.container_stop(id, signal, timeout).await)
                    .await
            }
            Restart => {
                self.api_result(api.container_restart(id, signal, timeout).await)
                    .await
            }
            Pause => self.api_result(api.container_pause(id).await).await,
            Unpause => self.api_result(api.container_unpause(id).await).await,
            Delete(options) => {
                return self
                    .delete_container_with_options(id, options.v, options.force, options.link)
                    .await;
            }
        };
        match result {
            Err(DockerError::Api {
                status: reqwest::StatusCode::NOT_MODIFIED,
                ..
            }) => Ok(()),
            other => other,
        }
    }
    pub async fn prune_resources(
        &self,
        resource: citadel_platforms::prune::PruneResource,
    ) -> Result<Value, DockerError> {
        use citadel_platforms::prune::PruneResource::*;
        let config = self.configuration().await?;
        match resource {
            Volume => document(
                self.api_result(VolumeApiClient::new(config).volume_prune(Some("{}")).await)
                    .await?,
            ),
            Network => document(
                self.api_result(
                    NetworkApiClient::new(config)
                        .network_prune(Some("{}"))
                        .await,
                )
                .await?,
            ),
            Image => document(
                self.api_result(
                    ImageApiClient::new(config)
                        .image_prune(Some(r#"{"dangling":["false"]}"#))
                        .await,
                )
                .await?,
            ),
            Build => document(
                self.api_result(
                    ImageApiClient::new(config)
                        .build_prune(None, None, None, None, Some(true), Some("{}"))
                        .await,
                )
                .await?,
            ),
            All => Err(DockerError::InvalidIdentifier),
        }
    }
    pub async fn inspect_swarm(&self) -> Result<SwarmInspect, DockerError> {
        self.api_result(
            SwarmApiClient::new(self.configuration().await?)
                .swarm_inspect()
                .await,
        )
        .await?
        .try_into()
    }
    pub async fn list_image_models(
        &self,
    ) -> Result<Vec<citadel_docker_api::models::ImageSummary>, DockerError> {
        let _list = RuntimeWork::DockerImageList.start();
        self.api_result(
            ImageApiClient::new(self.configuration().await?)
                .image_list(Some(true), None, None, None, None)
                .await,
        )
        .await
    }
    pub async fn list_images(&self) -> Result<Vec<ImageSummary>, DockerError> {
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::DockerList.start();
        convert_list(self.list_image_models().await?)
    }
    /// Lifecycle observations need one image and its usage, never the image set.
    pub async fn image_event_model(
        &self,
        id: &str,
    ) -> Result<citadel_docker_api::models::ImageSummary, DockerError> {
        validate_identifier(id)?;
        let inspect = async {
            self.api_result(
                ImageApiClient::new(self.configuration().await?)
                    .image_inspect(id, None)
                    .await,
            )
            .await
        };
        let (image, containers) = tokio::try_join!(inspect, self.image_containers(id))?;
        Ok(citadel_docker_api::models::ImageSummary {
            id: image.id.unwrap_or_default(),
            parent_id: image.parent.unwrap_or_default(),
            repo_tags: image.repo_tags,
            repo_digests: image.repo_digests,
            created: image
                .created
                .flatten()
                .and_then(|v| chrono::DateTime::parse_from_rfc3339(&v).ok())
                .map_or(0, |v| {
                    v.timestamp().clamp(i32::MIN as i64, i32::MAX as i64) as i32
                }),
            size: image.size.unwrap_or_default(),
            virtual_size: image.virtual_size,
            shared_size: -1,
            labels: image.config.and_then(|c| c.labels),
            containers: containers.len().min(i32::MAX as usize) as i32,
            ..Default::default()
        })
    }
    pub async fn inspect_image(&self, id: &str) -> Result<ImageInspect, DockerError> {
        validate_identifier(id)?;
        self.api_result(
            ImageApiClient::new(self.configuration().await?)
                .image_inspect(id, None)
                .await,
        )
        .await?
        .try_into()
    }
    pub async fn image_history(&self, id: &str) -> Result<Vec<ImageHistoryItem>, DockerError> {
        validate_identifier(id)?;
        convert_list(
            self.api_result(
                ImageApiClient::new(self.configuration().await?)
                    .image_history(id, None)
                    .await,
            )
            .await?,
        )
    }
    pub async fn delete_image(
        &self,
        id: &str,
        force: bool,
        no_prune: bool,
    ) -> Result<Vec<std::collections::BTreeMap<String, String>>, DockerError> {
        validate_identifier(id)?;
        body(
            &self
                .api_result(
                    ImageApiClient::new(self.configuration().await?)
                        .image_delete(id, Some(force), Some(no_prune))
                        .await,
                )
                .await?,
        )
    }
    pub async fn image_containers(
        &self,
        id: &str,
    ) -> Result<Vec<ImageUsageContainer>, DockerError> {
        validate_identifier(id)?;
        let filters = serde_json::json!({"ancestor":[id]}).to_string();
        convert_list(
            self.api_result(
                ContainerApiClient::new(self.configuration().await?)
                    .container_list(Some(true), None, None, Some(&filters))
                    .await,
            )
            .await?,
        )
    }
    pub(super) async fn volumes(&self) -> Result<VolumeListResponse, DockerError> {
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::DockerList.start();
        self.list_volume_models(None).await?.try_into()
    }
    pub async fn list_volume_models(
        &self,
        filters: Option<&str>,
    ) -> Result<models::VolumeListResponse, DockerError> {
        let _list = RuntimeWork::DockerVolumeList.start();
        self.api_result(
            VolumeApiClient::new(self.configuration().await?)
                .volume_list(filters)
                .await,
        )
        .await
    }
    pub async fn list_volumes(&self) -> Result<Vec<DockerVolume>, DockerError> {
        self.list_volumes_filtered(None).await
    }
    pub async fn list_volumes_filtered(
        &self,
        filters: Option<&str>,
    ) -> Result<Vec<DockerVolume>, DockerError> {
        let mut volumes: Vec<DockerVolume> = convert_list(
            self.list_volume_models(filters)
                .await?
                .volumes
                .unwrap_or_default(),
        )?;
        if volumes.iter().any(|v| v.usage_data.is_none()) {
            let mut usage = self.volume_usage().await?;
            for volume in &mut volumes {
                if volume.usage_data.is_none() {
                    volume.usage_data = usage.remove(&volume.name);
                }
            }
        }
        Ok(volumes)
    }
    pub(super) async fn system_data_usage(
        &self,
        types: Vec<String>,
    ) -> Result<models::SystemDataUsageResponse, DockerError> {
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::DockerStorage.start();
        self.api_result(
            SystemApiClient::new(self.configuration().await?)
                .system_data_usage(Some(types))
                .await,
        )
        .await
    }
    async fn volume_usage(&self) -> Result<std::collections::HashMap<String, Value>, DockerError> {
        let usage = self.system_data_usage(vec!["volume".into()]).await?;
        usage
            .volumes
            .unwrap_or_default()
            .into_iter()
            .filter_map(|v| v.usage_data.flatten().map(|data| (v.name, data)))
            .map(|(name, data)| Ok((name, document(data)?)))
            .collect()
    }
    /// Only explicit volume inspection needs its attached containers.
    pub async fn volume_containers(
        &self,
        name: &str,
    ) -> Result<Vec<models::ContainerSummary>, DockerError> {
        validate_identifier(name)?;
        let filters = serde_json::json!({"volume": [name]}).to_string();
        self.list_container_models(Some(true), None, None, Some(&filters))
            .await
    }
    pub async fn inspect_volume(&self, name: &str) -> Result<DockerVolume, DockerError> {
        validate_identifier(name)?;
        let mut volume: DockerVolume = self
            .api_result(
                VolumeApiClient::new(self.configuration().await?)
                    .volume_inspect(name)
                    .await,
            )
            .await?
            .try_into()?;
        if volume.usage_data.is_none() {
            volume.usage_data = self.volume_usage().await?.remove(&volume.name);
        }
        Ok(volume)
    }
    pub async fn create_volume(
        &self,
        request: &VolumeCreateOptions,
    ) -> Result<DockerVolume, DockerError> {
        self.api_result(
            VolumeApiClient::new(self.configuration().await?)
                .volume_create(body(request)?)
                .await,
        )
        .await?
        .try_into()
    }
    pub async fn delete_volume(&self, name: &str, force: bool) -> Result<(), DockerError> {
        validate_identifier(name)?;
        self.api_result(
            VolumeApiClient::new(self.configuration().await?)
                .volume_delete(name, Some(force))
                .await,
        )
        .await
    }
    pub async fn list_networks(&self) -> Result<Vec<DockerNetwork>, DockerError> {
        self.list_networks_filtered(None).await
    }
    pub async fn list_networks_filtered(
        &self,
        filters: Option<&str>,
    ) -> Result<Vec<DockerNetwork>, DockerError> {
        let _list = RuntimeWork::DockerNetworkList.start();
        convert_list(
            self.api_result(
                NetworkApiClient::new(self.configuration().await?)
                    .network_list(filters)
                    .await,
            )
            .await?,
        )
    }
    pub async fn inspect_network(&self, id: &str) -> Result<DockerNetwork, DockerError> {
        validate_identifier(id)?;
        self.api_result(
            NetworkApiClient::new(self.configuration().await?)
                .network_inspect(id, None, None)
                .await,
        )
        .await?
        .try_into()
    }
    pub async fn create_network(
        &self,
        request: &NetworkCreateRequest,
    ) -> Result<NetworkCreateResponse, DockerError> {
        let value = self
            .api_result(
                NetworkApiClient::new(self.configuration().await?)
                    .network_create(body(request)?)
                    .await,
            )
            .await?;
        Ok(NetworkCreateResponse {
            id: value.id,
            warning: value.warning,
        })
    }
    pub async fn delete_network(&self, id: &str) -> Result<(), DockerError> {
        validate_identifier(id)?;
        self.api_result(
            NetworkApiClient::new(self.configuration().await?)
                .network_delete(id)
                .await,
        )
        .await
    }
    pub async fn list_swarm_nodes(&self) -> Result<Vec<SwarmNode>, DockerError> {
        self.list_swarm_nodes_filtered(None).await
    }
    pub async fn list_swarm_nodes_filtered(
        &self,
        filters: Option<&str>,
    ) -> Result<Vec<SwarmNode>, DockerError> {
        convert_list(
            self.api_result(
                NodeApiClient::new(self.configuration().await?)
                    .node_list(filters)
                    .await,
            )
            .await?,
        )
    }
    pub async fn inspect_swarm_node(&self, id: &str) -> Result<SwarmNode, DockerError> {
        validate_identifier(id)?;
        self.api_result(
            NodeApiClient::new(self.configuration().await?)
                .node_inspect(id)
                .await,
        )
        .await?
        .try_into()
    }
    pub async fn update_swarm_node(
        &self,
        id: &str,
        version: i64,
        spec: &Value,
    ) -> Result<(), DockerError> {
        validate_identifier(id)?;
        self.api_result(
            NodeApiClient::new(self.configuration().await?)
                .node_update(id, version, Some(body(spec)?))
                .await,
        )
        .await
    }
    pub async fn update_swarm_material(
        &self,
        secret: bool,
        id: &str,
        version: i64,
        spec: &Value,
    ) -> Result<(), DockerError> {
        validate_identifier(id)?;
        let config = self.configuration().await?;
        if secret {
            self.api_result(
                SecretApiClient::new(config)
                    .secret_update(id, version, Some(body(spec)?))
                    .await,
            )
            .await
        } else {
            self.api_result(
                ConfigApiClient::new(config)
                    .config_update(id, version, Some(body(spec)?))
                    .await,
            )
            .await
        }
    }
    async fn tasks(&self, filters: &str) -> Result<Vec<SwarmTask>, DockerError> {
        self.list_swarm_tasks_filtered(Some(filters)).await
    }
    pub async fn list_swarm_tasks_filtered(
        &self,
        filters: Option<&str>,
    ) -> Result<Vec<SwarmTask>, DockerError> {
        convert_list(
            self.api_result(
                TaskApiClient::new(self.configuration().await?)
                    .task_list(filters)
                    .await,
            )
            .await?,
        )
    }
    pub async fn list_swarm_service_tasks(&self, id: &str) -> Result<Vec<SwarmTask>, DockerError> {
        validate_identifier(id)?;
        self.tasks(&serde_json::json!({"service":[id],"desired-state":["running"]}).to_string())
            .await
    }
    pub async fn list_swarm_node_tasks(&self, id: &str) -> Result<Vec<SwarmTask>, DockerError> {
        validate_identifier(id)?;
        self.tasks(&serde_json::json!({"node":[id],"desired-state":["running"]}).to_string())
            .await
    }
    pub async fn list_swarm_tasks(&self) -> Result<Vec<SwarmTask>, DockerError> {
        self.tasks(r#"{"desired-state":["running"]}"#).await
    }
    pub async fn list_swarm_services(&self) -> Result<Vec<SwarmService>, DockerError> {
        self.list_swarm_services_filtered(None).await
    }
    pub async fn list_swarm_services_filtered(
        &self,
        filters: Option<&str>,
    ) -> Result<Vec<SwarmService>, DockerError> {
        convert_list(
            self.api_result(
                ServiceApiClient::new(self.configuration().await?)
                    .service_list(filters, Some(true))
                    .await,
            )
            .await?,
        )
    }
    pub async fn inspect_swarm_service(&self, id: &str) -> Result<SwarmService, DockerError> {
        validate_identifier(id)?;
        self.api_result(
            ServiceApiClient::new(self.configuration().await?)
                .service_inspect(id, None)
                .await,
        )
        .await?
        .try_into()
    }
    pub async fn inspect_swarm_task(&self, id: &str) -> Result<SwarmTask, DockerError> {
        validate_identifier(id)?;
        self.api_result(
            TaskApiClient::new(self.configuration().await?)
                .task_inspect(id)
                .await,
        )
        .await?
        .try_into()
    }
    pub async fn create_swarm_service(&self, spec: &Value) -> Result<Value, DockerError> {
        self.create_swarm_service_authenticated(spec, None).await
    }
    pub async fn create_swarm_service_authenticated(
        &self,
        spec: &Value,
        auth: Option<&str>,
    ) -> Result<Value, DockerError> {
        document(
            self.api_result(
                ServiceApiClient::new(self.configuration().await?)
                    .service_create(body(spec)?, auth)
                    .await,
            )
            .await?,
        )
    }
    pub async fn update_swarm_service(
        &self,
        id: &str,
        version: i64,
        spec: &Value,
    ) -> Result<Value, DockerError> {
        self.update_swarm_service_authenticated(id, version, spec, None)
            .await
    }
    pub async fn update_swarm_service_authenticated(
        &self,
        id: &str,
        version: i64,
        spec: &Value,
        auth: Option<&str>,
    ) -> Result<Value, DockerError> {
        validate_identifier(id)?;
        document(
            self.api_result(
                ServiceApiClient::new(self.configuration().await?)
                    .service_update(id, version, body(spec)?, Some("spec"), None, auth)
                    .await,
            )
            .await?,
        )
    }
    pub async fn delete_swarm_service(&self, id: &str) -> Result<(), DockerError> {
        validate_identifier(id)?;
        self.api_result(
            ServiceApiClient::new(self.configuration().await?)
                .service_delete(id)
                .await,
        )
        .await
    }
    pub async fn list_swarm_secrets(&self) -> Result<Vec<SwarmSecret>, DockerError> {
        self.list_swarm_secrets_filtered(None).await
    }
    pub async fn list_swarm_secrets_filtered(
        &self,
        filters: Option<&str>,
    ) -> Result<Vec<SwarmSecret>, DockerError> {
        convert_list(
            self.api_result(
                SecretApiClient::new(self.configuration().await?)
                    .secret_list(filters)
                    .await,
            )
            .await?,
        )
    }
    pub async fn inspect_swarm_secret(&self, id: &str) -> Result<SwarmSecret, DockerError> {
        validate_identifier(id)?;
        self.api_result(
            SecretApiClient::new(self.configuration().await?)
                .secret_inspect(id)
                .await,
        )
        .await?
        .try_into()
    }
    pub async fn list_swarm_configs(&self) -> Result<Vec<SwarmConfig>, DockerError> {
        self.list_swarm_configs_filtered(None).await
    }
    pub async fn list_swarm_configs_filtered(
        &self,
        filters: Option<&str>,
    ) -> Result<Vec<SwarmConfig>, DockerError> {
        convert_list(
            self.api_result(
                ConfigApiClient::new(self.configuration().await?)
                    .config_list(filters)
                    .await,
            )
            .await?,
        )
    }
    pub async fn inspect_swarm_config(&self, id: &str) -> Result<SwarmConfig, DockerError> {
        validate_identifier(id)?;
        self.api_result(
            ConfigApiClient::new(self.configuration().await?)
                .config_inspect(id)
                .await,
        )
        .await?
        .try_into()
    }
    pub async fn create_swarm_material(
        &self,
        secret: bool,
        spec: &Value,
    ) -> Result<Value, DockerError> {
        let config = self.configuration().await?;
        let response = if secret {
            self.api_result(
                SecretApiClient::new(config)
                    .secret_create(Some(body(spec)?))
                    .await,
            )
            .await?
        } else {
            self.api_result(
                ConfigApiClient::new(config)
                    .config_create(Some(body(spec)?))
                    .await,
            )
            .await?
        };
        document(response)
    }
    pub async fn delete_swarm_secret(&self, id: &str) -> Result<(), DockerError> {
        validate_identifier(id)?;
        self.api_result(
            SecretApiClient::new(self.configuration().await?)
                .secret_delete(id)
                .await,
        )
        .await
    }
    pub async fn delete_swarm_config(&self, id: &str) -> Result<(), DockerError> {
        validate_identifier(id)?;
        self.api_result(
            ConfigApiClient::new(self.configuration().await?)
                .config_delete(id)
                .await,
        )
        .await
    }
    pub async fn distribution_inspect(&self, image: &str) -> Result<Value, DockerError> {
        self.distribution_inspect_authenticated(image, None).await
    }
    pub async fn distribution_inspect_authenticated(
        &self,
        image: &str,
        auth: Option<&str>,
    ) -> Result<Value, DockerError> {
        validate_identifier(image)?;
        match self
            .api_result(
                DistributionApiClient::new(self.configuration().await?)
                    .distribution_inspect(image, auth)
                    .await,
            )
            .await
        {
            Ok(value) => document(value),
            Err(DockerError::Api { status, .. }) => Err(DockerError::Api {
                status,
                message: "Registry inspection failed.".into(),
            }),
            Err(error) => Err(error),
        }
    }
    pub async fn container_stats_once(&self, id: &str) -> Result<ContainerStats, DockerError> {
        let _iteration = citadel_runtime::runtime_metrics::RuntimeWork::DockerStats.start();
        validate_identifier(id)?;
        self.api_result(
            ContainerApiClient::new(self.configuration().await?)
                .container_stats(id, Some(false), Some(false))
                .await,
        )
        .await?
        .try_into()
    }
    pub(super) async fn create_exec(&self, id: &str, config: Value) -> Result<String, DockerError> {
        validate_identifier(id)?;
        Ok(self
            .api_result(
                ExecApiClient::new(self.configuration().await?)
                    .container_exec(id, body(&config)?)
                    .await,
            )
            .await?
            .id)
    }
    pub(crate) async fn resize_exec(
        &self,
        id: &str,
        cols: u16,
        rows: u16,
    ) -> Result<(), DockerError> {
        self.api_result(
            ExecApiClient::new(self.configuration().await?)
                .exec_resize(id, i32::from(rows), i32::from(cols))
                .await,
        )
        .await
    }
    pub(crate) async fn exec_inspect(
        &self,
        id: &str,
    ) -> Result<models::ExecInspectResponse, DockerError> {
        self.api_result(
            ExecApiClient::new(self.configuration().await?)
                .exec_inspect(id)
                .await,
        )
        .await
    }
}
