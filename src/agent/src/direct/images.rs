use super::{Runtime, container_mapping, docker_error, runtime_error, text};
use citadel_contracts::citadel::{
    images::v1::{image_service_server::ImageService, *},
    shared_models::v1::{ImageReply, ListImageResponse},
};
use citadel_docker::client::DockerImagePullOptions;
use citadel_platforms::images::ImageInspectionPort;
use futures_util::{Stream, StreamExt};
use std::pin::Pin;
use tonic::{Request, Response, Status};
type Output<T> = Pin<Box<dyn Stream<Item = Result<T, Status>> + Send>>;
#[tonic::async_trait]
impl ImageService for Runtime {
    async fn list(
        &self,
        _: Request<ListImagesRequest>,
    ) -> Result<Response<ListImageResponse>, Status> {
        Ok(Response::new(ListImageResponse {
            images: self
                .docker
                .list_image_models_with_usage()
                .await
                .map_err(docker_error)?
                .into_iter()
                .map(summary)
                .collect(),
        }))
    }
    async fn get(&self, r: Request<GetImageRequest>) -> Result<Response<ImageReply>, Status> {
        let id = r.into_inner().id;
        let item = self
            .docker
            .image_event_model(&id)
            .await
            .map_err(docker_error)?;
        Ok(Response::new(summary(item)))
    }
    async fn delete(
        &self,
        r: Request<DeleteImageRequest>,
    ) -> Result<Response<DeleteImageResponse>, Status> {
        let r = r.into_inner();
        let mut items = Vec::new();
        if r.ids.iter().any(|id| id.trim().is_empty()) {
            return Err(Status::invalid_argument("Image ids must not be empty"));
        }
        for id in r.ids {
            items.extend(
                self.docker
                    .delete_image(&id, r.force, r.noprune)
                    .await
                    .map_err(docker_error)?
                    .into_iter()
                    .map(|v| DeleteImageResponseItem {
                        result: v.into_iter().collect(),
                    }),
            );
        }
        Ok(Response::new(DeleteImageResponse { items }))
    }
    async fn inspect(
        &self,
        r: Request<InspectImageRequest>,
    ) -> Result<Response<InspectImageResponse>, Status> {
        let id = r.into_inner().id;
        let image = ImageInspectionPort::inspect_image(&self.docker, &id, &self.shutdown)
            .await
            .map_err(runtime_error)?;
        Ok(Response::new(InspectImageResponse {
            config_digest: image.config_digest,
            content_digest: image.content_digest,
            variant: image.variant,
            id: image.id,
            size: image.size,
            created: image.created,
            env: image.env,
            cmd: image.cmd,
            repo_tags: image.repo_tags,
            volumes: image.volumes,
            labels: image.labels.into_iter().collect(),
            exposed_ports: image.exposed_ports,
            os: Some(image.os),
            architecture: Some(image.architecture),
            user: image.user,
            working_dir: image.working_dir,
            entry_point: image.entry_point,
            stop_signal: image.stop_signal,
            layers: image
                .layers
                .into_iter()
                .map(|v| HistoryImageResult {
                    id: v.id,
                    created: v.created,
                    created_by: v.created_by,
                    size: v.size,
                    comment: v.comment,
                })
                .collect(),
            containers: image
                .containers
                .into_iter()
                .map(|v| ContainerImageResult {
                    id: v.id,
                    name: v.name,
                    state: container_mapping::state(&v.state),
                    volumes: v.volumes,
                    networks: v.networks.into_iter().collect(),
                    ports: v
                        .ports
                        .as_object()
                        .into_iter()
                        .flatten()
                        .map(|(key, v)| (key.clone(), container_mapping::host_port_binding_list(v)))
                        .collect(),
                })
                .collect(),
        }))
    }
    async fn history(
        &self,
        r: Request<HistoryImageRequest>,
    ) -> Result<Response<HistoryImageResponse>, Status> {
        Ok(Response::new(HistoryImageResponse {
            items: self
                .docker
                .image_history(&r.into_inner().id)
                .await
                .map_err(docker_error)?
                .into_iter()
                .map(|v| HistoryImageItemResponse {
                    id: v.id,
                    created: v.created,
                    created_by: v.created_by,
                    size: v.size,
                    comment: v.comment,
                })
                .collect(),
        }))
    }
    async fn get_exposed_ports(
        &self,
        r: Request<GetExposedPortsRequest>,
    ) -> Result<Response<GetExposedPortsResponse>, Status> {
        Ok(Response::new(GetExposedPortsResponse {
            ports: self
                .docker
                .exposed_ports(&r.into_inner().id, &self.shutdown)
                .await
                .map_err(runtime_error)?,
        }))
    }
    async fn distribution_inspect(
        &self,
        r: Request<DistributionInspectRequest>,
    ) -> Result<Response<DistributionInspectResponse>, Status> {
        let r = r.into_inner();
        if let Some(target) = &r.target_platform {
            let target = citadel_platforms::image_updates::ImagePlatform {
                os: target.os.clone(),
                architecture: target.architecture.clone(),
                variant: target.variant.clone(),
            };
            let observed = citadel_docker::image_metadata::inspect(
                &r.image_name,
                r.auth.as_deref(),
                &target,
                &self.shutdown,
            )
            .await
            .map_err(runtime_error)?;
            return Ok(Response::new(DistributionInspectResponse {
                config_digest: observed.config_digest,
                manifest_digest: observed.manifest_digest,
                descriptor: Some(OciDescriptorResponse {
                    digest: observed.index_digest,
                    ..Default::default()
                }),
                ..Default::default()
            }));
        }
        let v = self
            .docker
            .distribution_inspect_authenticated(&r.image_name, r.auth.as_deref())
            .await
            .map_err(docker_error)?;
        let d = &v["Descriptor"];
        Ok(Response::new(DistributionInspectResponse {
            config_digest: String::new(),
            manifest_digest: String::new(),
            descriptor: (!d.is_null()).then(|| OciDescriptorResponse {
                media_type: text(d, "mediaType"),
                digest: text(d, "digest"),
                size: d["size"].as_i64(),
                artifact_type: text(d, "artifactType"),
                platform: (!d["platform"].is_null()).then(|| platform(&d["platform"])),
            }),
            platforms: v["Platforms"]
                .as_array()
                .into_iter()
                .flatten()
                .map(platform)
                .collect(),
        }))
    }
    async fn check_build_host(
        &self,
        _: Request<()>,
    ) -> Result<Response<CheckBuildHostResponse>, Status> {
        self.docker.ping().await.map_err(docker_error)?;
        let v = self.docker.version().await.map_err(docker_error)?;
        let (capacity, occupied, uncertain) = self.builds.capacity()?;
        Ok(Response::new(CheckBuildHostResponse {
            docker_daemon_id: self.docker.info().await.map_err(docker_error)?.id,
            admission_authority_id: self.builds.authority()?,
            admission_generation: self.builds.generation()?,
            build_capacity: capacity as u32,
            data_capacity: self.data.capacity().0 as u32,
            occupied_data: self.data.capacity().1 as u32,
            heavy_capacity: self.heavy_capacity as u32,
            occupied_builds: occupied as u32,
            uncertain_builds: uncertain as u32,
            available: true,
            docker_version: v.version,
            api_version: v.api_version,
            operating_system: v.os,
            architecture: v.arch,
            build_kit_version: v
                .components
                .into_iter()
                .find(|v| v.name.eq_ignore_ascii_case("buildkit"))
                .map(|v| v.version)
                .unwrap_or_default(),
        }))
    }
    async fn reserve_data(
        &self,
        request: Request<DataOperationRequest>,
    ) -> Result<Response<DataAdmissionResponse>, Status> {
        let request = request.into_inner();
        self.validate_data_host(&request).await?;
        let admission = self.builds.reserve_data(&self.data, &request)?;
        Ok(Response::new(DataAdmissionResponse {
            admission: admission as i32,
        }))
    }
    async fn reconcile_data(
        &self,
        request: Request<DataOperationRequest>,
    ) -> Result<Response<ReconcileBuildResponse>, Status> {
        Ok(Response::new(ReconcileBuildResponse {
            state: self.reconcile_data_operation(request.into_inner()).await? as i32,
        }))
    }
    async fn reconcile_build(
        &self,
        request: Request<ReconcileBuildRequest>,
    ) -> Result<Response<ReconcileBuildResponse>, Status> {
        let request = request.into_inner();
        let daemon = self.docker.info().await.map_err(docker_error)?.id;
        if daemon.is_empty() || daemon != request.docker_daemon_id {
            return Err(Status::failed_precondition(
                "Build executor identity changed",
            ));
        }
        let state = self.builds.reconcile(
            &request.run_id,
            &request.attempt_id,
            &request.admission_authority_id,
            request.admission_generation,
        )?;
        Ok(Response::new(ReconcileBuildResponse {
            state: state as i32,
        }))
    }
    type PullStream = Output<PullImageResponse>;
    async fn pull(
        &self,
        r: Request<PullImageRequest>,
    ) -> Result<Response<Self::PullStream>, Status> {
        let r = r.into_inner();
        let mut input = self
            .docker
            .pull_image_with_options(
                DockerImagePullOptions {
                    from_image: Some(&r.from_image),
                    from_source: r.from_src.as_deref(),
                    repository: r.repo.as_deref(),
                    tag: r.tag.as_deref(),
                    changes: &r.changes,
                },
                r.auth.as_deref(),
            )
            .await
            .map_err(docker_error)?;
        let cancel = self.shutdown.child_token();
        let guard = cancel.clone().drop_guard();
        Ok(Response::new(Box::pin(
            async_stream::try_stream! {let _guard=guard;loop{
                let item=tokio::select!{()=cancel.cancelled()=>break,item=input.next()=>item};let Some(item)=item else{break};let v=item.map_err(docker_error)?;
                let failed=v.error.is_some()||v.error_detail.is_some();
                yield PullImageResponse{stream:v.stream,status:v.status,progress_message:v.progress,id:v.id,from:v.from,error_message:v.error,
                    progress:v.progress_detail.map(|v|JsonProgressReply{current:v.current,total:v.total,start:v.start,units:v.units}),error:v.error_detail.map(|v|JsonErrorReply{code:v.code,message:Some(v.message)})};
                if failed{break;}
            }},
        )))
    }
    type BuildStream = Output<ImageBuildResponse>;
    async fn build(
        &self,
        r: Request<BuildImageRequest>,
    ) -> Result<Response<Self::BuildStream>, Status> {
        super::image_builds::build(self, r.into_inner()).await
    }
    type PushStream = Output<ImageBuildResponse>;
    async fn push(
        &self,
        r: Request<PushImageRequest>,
    ) -> Result<Response<Self::PushStream>, Status> {
        super::image_builds::push(self, r.into_inner()).await
    }
}
pub(super) fn summary(v: citadel_docker_api::models::ImageSummary) -> ImageReply {
    ImageReply {
        id: v.id,
        parent_id: v.parent_id,
        repo_tags: v.repo_tags.unwrap_or_default(),
        repo_digests: v.repo_digests.unwrap_or_default(),
        created: v.created.into(),
        size: v.size as f64,
        shared_size: v.shared_size,
        virtual_size: v.virtual_size.unwrap_or_default() as f64,
        containers: v.containers,
        labels: v.labels.unwrap_or_default(),
    }
}
fn platform(v: &serde_json::Value) -> OciPlatformResponse {
    OciPlatformResponse {
        variant: text(v, "variant"),
        architecture: text(v, "architecture"),
        os: text(v, "os"),
        os_version: text(v, "os.version"),
    }
}
