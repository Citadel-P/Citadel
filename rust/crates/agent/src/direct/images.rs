use super::{Runtime, container_mapping, docker_error, runtime_error, text};
use citadel_adapters::connectors::docker::DockerImagePullOptions;
use citadel_contracts::citadel::{
    images::v1::{image_service_server::ImageService, *},
    shared_models::v1::{ImageReply, ListImageResponse},
};
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
                .list_image_models()
                .await
                .map_err(docker_error)?
                .into_iter()
                .map(summary)
                .collect(),
        }))
    }
    async fn get(&self, r: Request<GetImageRequest>) -> Result<Response<ImageReply>, Status> {
        let id = r.into_inner().id;
        let image = self.docker.inspect_image(&id).await.map_err(docker_error)?;
        let item = self
            .docker
            .list_image_models()
            .await
            .map_err(docker_error)?
            .into_iter()
            .find(|v| v.id == image.id)
            .ok_or_else(|| Status::not_found("Image not found"))?;
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
        let raw = self
            .docker
            .inspect_image_document(&id)
            .await
            .map_err(docker_error)?;
        let config = &raw["Config"];
        Ok(Response::new(InspectImageResponse {
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
            user: config["User"].as_str().map(str::to_owned),
            working_dir: config["WorkingDir"].as_str().map(str::to_owned),
            entry_point: config["Entrypoint"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect(),
            stop_signal: config["StopSignal"].as_str().map(str::to_owned),
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
        let v = self
            .docker
            .distribution_inspect_authenticated(&r.image_name, r.auth.as_deref())
            .await
            .map_err(docker_error)?;
        let d = &v["Descriptor"];
        Ok(Response::new(DistributionInspectResponse {
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
        Ok(Response::new(CheckBuildHostResponse {
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
        architecture: text(v, "architecture"),
        os: text(v, "os"),
        os_version: text(v, "os.version"),
    }
}
