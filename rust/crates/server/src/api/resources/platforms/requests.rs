use crate::api::resources::metadata_patch::MetadataPatch;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Default, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchPlatformMetadataInput {
    #[serde(default)]
    #[schema(value_type = Option<String>, required = false)]
    pub(crate) description: MetadataPatch<String>,
    #[serde(default, rename = "tags")]
    pub(crate) _tags: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct NetworkFilters {
    #[serde(rename = "Dangling", alias = "dangling")]
    pub(crate) dangling: Option<bool>,
    #[serde(rename = "Driver", alias = "driver")]
    pub(crate) driver: Option<String>,
    #[serde(rename = "Id", alias = "id")]
    pub(crate) id: Option<String>,
    #[serde(rename = "Name", alias = "name")]
    pub(crate) name: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct VolumeFilters {
    #[serde(rename = "Dangling", alias = "dangling")]
    pub(crate) dangling: Option<bool>,
    #[serde(rename = "Driver", alias = "driver")]
    pub(crate) driver: Option<String>,
    #[serde(rename = "Name", alias = "name")]
    pub(crate) name: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateNetworkInput {
    pub(crate) platform_id: Uuid,
    #[serde(flatten)]
    pub(crate) network: CreateRuntimeNetwork,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteNetworksInput {
    pub(crate) platform_id: Uuid,
    pub(crate) ids: Vec<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateVolumeInput {
    pub(crate) platform_id: Uuid,
    #[serde(flatten)]
    pub(crate) volume: CreateRuntimeVolume,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteVolumesInput {
    pub(crate) platform_id: Uuid,
    pub(crate) names: Vec<String>,
    pub(crate) force: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SwarmTaskFilters {
    #[serde(default = "default_swarm_task_limit")]
    pub(crate) limit: usize,
    pub(crate) service_id: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DockerNodeSelector {
    pub(crate) docker_node_id: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct Tail {
    #[serde(default = "default_tail", alias = "Tail")]
    pub(crate) tail: u16,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteImagesInput {
    pub(crate) platform_id: Uuid,
    pub(crate) ids: Vec<String>,
    #[serde(default)]
    pub(crate) force: bool,
    #[serde(default)]
    pub(crate) no_prune: bool,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteInput {
    pub(crate) container_ids: Vec<String>,
    #[serde(flatten)]
    pub(crate) options: DeleteContainerOptions,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContentQuery {
    pub(crate) path: Option<String>,
    pub(crate) docker_node_id: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct Hours {
    #[serde(default = "default_hours", alias = "Hours")]
    pub(crate) hours: u16,
}

const fn default_hours() -> u16 {
    24
}

const fn default_tail() -> u16 {
    100
}

const fn default_swarm_task_limit() -> usize {
    50
}

fn default_prune_historical_swarm_task_containers() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum PlatformType {
    #[default]
    Docker,
    DockerSwarm,
    Kubernetes,
}

impl From<PlatformType> for citadel_platforms::PlatformType {
    fn from(value: PlatformType) -> Self {
        match value {
            PlatformType::Docker => Self::Docker,
            PlatformType::DockerSwarm => Self::DockerSwarm,
            PlatformType::Kubernetes => Self::Kubernetes,
        }
    }
}

impl From<citadel_platforms::PlatformType> for PlatformType {
    fn from(value: citadel_platforms::PlatformType) -> Self {
        match value {
            citadel_platforms::PlatformType::Docker => Self::Docker,
            citadel_platforms::PlatformType::DockerSwarm => Self::DockerSwarm,
            citadel_platforms::PlatformType::Kubernetes => Self::Kubernetes,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum PlatformConnectorType {
    #[default]
    Unknown,
    Local,
    Agent,
    EdgeAgent,
}

impl From<PlatformConnectorType> for citadel_platforms::PlatformConnectorType {
    fn from(value: PlatformConnectorType) -> Self {
        match value {
            PlatformConnectorType::Unknown => Self::Unknown,
            PlatformConnectorType::Local => Self::Local,
            PlatformConnectorType::Agent => Self::Agent,
            PlatformConnectorType::EdgeAgent => Self::EdgeAgent,
        }
    }
}

impl From<citadel_platforms::PlatformConnectorType> for PlatformConnectorType {
    fn from(value: citadel_platforms::PlatformConnectorType) -> Self {
        match value {
            citadel_platforms::PlatformConnectorType::Unknown => Self::Unknown,
            citadel_platforms::PlatformConnectorType::Local => Self::Local,
            citadel_platforms::PlatformConnectorType::Agent => Self::Agent,
            citadel_platforms::PlatformConnectorType::EdgeAgent => Self::EdgeAgent,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreatePlatformInput {
    pub name: String,
    pub address: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "type")]
    #[serde(default)]
    pub platform_type: PlatformType,
    #[serde(default)]
    pub connector_type: PlatformConnectorType,
    #[serde(default = "default_prune_historical_swarm_task_containers")]
    pub prune_historical_swarm_task_containers: bool,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
}

impl From<CreatePlatformInput> for citadel_platforms::CreatePlatformInput {
    fn from(value: CreatePlatformInput) -> Self {
        Self {
            name: value.name,
            address: value.address,
            description: value.description,
            platform_type: value.platform_type.into(),
            connector_type: value.connector_type.into(),
            prune_historical_swarm_task_containers: value.prune_historical_swarm_task_containers,
            tag_ids: value.tag_ids,
        }
    }
}

impl From<citadel_platforms::CreatePlatformInput> for CreatePlatformInput {
    fn from(value: citadel_platforms::CreatePlatformInput) -> Self {
        Self {
            name: value.name,
            address: value.address,
            description: value.description,
            platform_type: value.platform_type.into(),
            connector_type: value.connector_type.into(),
            prune_historical_swarm_task_containers: value.prune_historical_swarm_task_containers,
            tag_ids: value.tag_ids,
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteContainerOptions {
    #[serde(default)]
    pub v: bool,
    #[serde(default)]
    pub force: bool,
    #[serde(default)]
    pub link: bool,
}

impl From<DeleteContainerOptions> for citadel_platforms::containers::DeleteContainerOptions {
    fn from(value: DeleteContainerOptions) -> Self {
        Self {
            v: value.v,
            force: value.force,
            link: value.link,
        }
    }
}

impl From<citadel_platforms::containers::DeleteContainerOptions> for DeleteContainerOptions {
    fn from(value: citadel_platforms::containers::DeleteContainerOptions) -> Self {
        Self {
            v: value.v,
            force: value.force,
            link: value.link,
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RenamePlatformInput {
    pub id: Uuid,
    pub name: String,
}

impl From<RenamePlatformInput> for citadel_platforms::management::RenamePlatformInput {
    fn from(value: RenamePlatformInput) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

impl From<citadel_platforms::management::RenamePlatformInput> for RenamePlatformInput {
    fn from(value: citadel_platforms::management::RenamePlatformInput) -> Self {
        Self {
            id: value.id,
            name: value.name,
        }
    }
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DeletePlatformsInput {
    pub ids: Vec<Uuid>,
}

impl From<DeletePlatformsInput> for citadel_platforms::deletion::DeletePlatformsInput {
    fn from(value: DeletePlatformsInput) -> Self {
        Self { ids: value.ids }
    }
}

impl From<citadel_platforms::deletion::DeletePlatformsInput> for DeletePlatformsInput {
    fn from(value: citadel_platforms::deletion::DeletePlatformsInput) -> Self {
        Self { ids: value.ids }
    }
}

#[derive(Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSwarmNodeInput {
    pub version_index: i64,
    pub availability: String,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
}

impl From<UpdateSwarmNodeInput> for citadel_platforms::swarm_mutations::UpdateSwarmNodeInput {
    fn from(value: UpdateSwarmNodeInput) -> Self {
        Self {
            version_index: value.version_index,
            availability: value.availability,
            labels: value.labels,
        }
    }
}

impl From<citadel_platforms::swarm_mutations::UpdateSwarmNodeInput> for UpdateSwarmNodeInput {
    fn from(value: citadel_platforms::swarm_mutations::UpdateSwarmNodeInput) -> Self {
        Self {
            version_index: value.version_index,
            availability: value.availability,
            labels: value.labels,
        }
    }
}

#[derive(Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmNodeAvailabilityTarget {
    pub node_id: String,
    pub version_index: i64,
}

impl From<SwarmNodeAvailabilityTarget>
    for citadel_platforms::swarm_mutations::SwarmNodeAvailabilityTarget
{
    fn from(value: SwarmNodeAvailabilityTarget) -> Self {
        Self {
            node_id: value.node_id,
            version_index: value.version_index,
        }
    }
}

impl From<citadel_platforms::swarm_mutations::SwarmNodeAvailabilityTarget>
    for SwarmNodeAvailabilityTarget
{
    fn from(value: citadel_platforms::swarm_mutations::SwarmNodeAvailabilityTarget) -> Self {
        Self {
            node_id: value.node_id,
            version_index: value.version_index,
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSwarmNodesAvailabilityInput {
    #[serde(default)]
    pub nodes: Vec<SwarmNodeAvailabilityTarget>,
    pub availability: String,
}

impl From<UpdateSwarmNodesAvailabilityInput>
    for citadel_platforms::swarm_mutations::UpdateSwarmNodesAvailabilityInput
{
    fn from(value: UpdateSwarmNodesAvailabilityInput) -> Self {
        Self {
            nodes: value.nodes.into_iter().map(|item| item.into()).collect(),
            availability: value.availability,
        }
    }
}

impl From<citadel_platforms::swarm_mutations::UpdateSwarmNodesAvailabilityInput>
    for UpdateSwarmNodesAvailabilityInput
{
    fn from(value: citadel_platforms::swarm_mutations::UpdateSwarmNodesAvailabilityInput) -> Self {
        Self {
            nodes: value.nodes.into_iter().map(|item| item.into()).collect(),
            availability: value.availability,
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSwarmResourceLabelsInput {
    pub version_index: i64,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
}

impl From<UpdateSwarmResourceLabelsInput>
    for citadel_platforms::swarm_mutations::UpdateSwarmResourceLabelsInput
{
    fn from(value: UpdateSwarmResourceLabelsInput) -> Self {
        Self {
            version_index: value.version_index,
            labels: value.labels,
        }
    }
}

impl From<citadel_platforms::swarm_mutations::UpdateSwarmResourceLabelsInput>
    for UpdateSwarmResourceLabelsInput
{
    fn from(value: citadel_platforms::swarm_mutations::UpdateSwarmResourceLabelsInput) -> Self {
        Self {
            version_index: value.version_index,
            labels: value.labels,
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
pub struct DeleteSwarmResourcesInput {
    #[serde(default)]
    pub ids: Vec<String>,
}

impl From<DeleteSwarmResourcesInput>
    for citadel_platforms::swarm_mutations::DeleteSwarmResourcesInput
{
    fn from(value: DeleteSwarmResourcesInput) -> Self {
        Self { ids: value.ids }
    }
}

impl From<citadel_platforms::swarm_mutations::DeleteSwarmResourcesInput>
    for DeleteSwarmResourcesInput
{
    fn from(value: citadel_platforms::swarm_mutations::DeleteSwarmResourcesInput) -> Self {
        Self { ids: value.ids }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum PruneResource {
    All,
    Volume,
    Network,
    Image,
    Build,
}

impl From<PruneResource> for citadel_platforms::prune::PruneResource {
    fn from(value: PruneResource) -> Self {
        match value {
            PruneResource::All => Self::All,
            PruneResource::Volume => Self::Volume,
            PruneResource::Network => Self::Network,
            PruneResource::Image => Self::Image,
            PruneResource::Build => Self::Build,
        }
    }
}

impl From<citadel_platforms::prune::PruneResource> for PruneResource {
    fn from(value: citadel_platforms::prune::PruneResource) -> Self {
        match value {
            citadel_platforms::prune::PruneResource::All => Self::All,
            citadel_platforms::prune::PruneResource::Volume => Self::Volume,
            citadel_platforms::prune::PruneResource::Network => Self::Network,
            citadel_platforms::prune::PruneResource::Image => Self::Image,
            citadel_platforms::prune::PruneResource::Build => Self::Build,
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PrunePlatformInput {
    pub resource: PruneResource,
}

impl From<PrunePlatformInput> for citadel_platforms::prune::PrunePlatformInput {
    fn from(value: PrunePlatformInput) -> Self {
        Self {
            resource: value.resource.into(),
        }
    }
}

impl From<citadel_platforms::prune::PrunePlatformInput> for PrunePlatformInput {
    fn from(value: citadel_platforms::prune::PrunePlatformInput) -> Self {
        Self {
            resource: value.resource.into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateRuntimeNetwork {
    pub name: String,
    pub driver: String,
    pub scope: String,
    pub internal: Option<bool>,
    pub attachable: Option<bool>,
    pub ingress: Option<bool>,
    #[serde(rename = "enableIPv6", alias = "enableIpv6")]
    pub enable_ipv6: Option<bool>,
    #[serde(rename = "enableIPv4", alias = "enableIpv4")]
    pub enable_ipv4: Option<bool>,
    pub config_only: Option<bool>,
    pub ipam: Option<RuntimeIpam>,
    pub config_from: Option<RuntimeConfigFrom>,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    #[serde(default)]
    pub options: BTreeMap<String, String>,
}

impl From<CreateRuntimeNetwork> for citadel_platforms::CreateRuntimeNetwork {
    fn from(value: CreateRuntimeNetwork) -> Self {
        Self {
            name: value.name,
            driver: value.driver,
            scope: value.scope,
            internal: value.internal,
            attachable: value.attachable,
            ingress: value.ingress,
            enable_ipv6: value.enable_ipv6,
            enable_ipv4: value.enable_ipv4,
            config_only: value.config_only,
            ipam: value.ipam.map(|item| item.into()),
            config_from: value.config_from.map(|item| item.into()),
            labels: value.labels,
            options: value.options,
        }
    }
}

impl From<citadel_platforms::CreateRuntimeNetwork> for CreateRuntimeNetwork {
    fn from(value: citadel_platforms::CreateRuntimeNetwork) -> Self {
        Self {
            name: value.name,
            driver: value.driver,
            scope: value.scope,
            internal: value.internal,
            attachable: value.attachable,
            ingress: value.ingress,
            enable_ipv6: value.enable_ipv6,
            enable_ipv4: value.enable_ipv4,
            config_only: value.config_only,
            ipam: value.ipam.map(|item| item.into()),
            config_from: value.config_from.map(|item| item.into()),
            labels: value.labels,
            options: value.options,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeIpam {
    pub driver: String,
    #[serde(default)]
    pub config: Vec<RuntimeIpamConfig>,
    #[serde(default)]
    pub options: BTreeMap<String, String>,
}

impl From<RuntimeIpam> for citadel_platforms::RuntimeIpam {
    fn from(value: RuntimeIpam) -> Self {
        Self {
            driver: value.driver,
            config: value.config.into_iter().map(|item| item.into()).collect(),
            options: value.options,
        }
    }
}

impl From<citadel_platforms::RuntimeIpam> for RuntimeIpam {
    fn from(value: citadel_platforms::RuntimeIpam) -> Self {
        Self {
            driver: value.driver,
            config: value.config.into_iter().map(|item| item.into()).collect(),
            options: value.options,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeIpamConfig {
    pub subnet: Option<String>,
    pub ip_range: Option<String>,
    pub gateway: Option<String>,
}

impl From<RuntimeIpamConfig> for citadel_platforms::RuntimeIpamConfig {
    fn from(value: RuntimeIpamConfig) -> Self {
        Self {
            subnet: value.subnet,
            ip_range: value.ip_range,
            gateway: value.gateway,
        }
    }
}

impl From<citadel_platforms::RuntimeIpamConfig> for RuntimeIpamConfig {
    fn from(value: citadel_platforms::RuntimeIpamConfig) -> Self {
        Self {
            subnet: value.subnet,
            ip_range: value.ip_range,
            gateway: value.gateway,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfigFrom {
    pub network: String,
}

impl From<RuntimeConfigFrom> for citadel_platforms::RuntimeConfigFrom {
    fn from(value: RuntimeConfigFrom) -> Self {
        Self {
            network: value.network,
        }
    }
}

impl From<citadel_platforms::RuntimeConfigFrom> for RuntimeConfigFrom {
    fn from(value: citadel_platforms::RuntimeConfigFrom) -> Self {
        Self {
            network: value.network,
        }
    }
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateRuntimeVolume {
    pub name: String,
    pub driver: String,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    #[serde(default)]
    pub options: BTreeMap<String, String>,
}

impl From<CreateRuntimeVolume> for citadel_platforms::CreateRuntimeVolume {
    fn from(value: CreateRuntimeVolume) -> Self {
        Self {
            name: value.name,
            driver: value.driver,
            labels: value.labels,
            options: value.options,
        }
    }
}

impl From<citadel_platforms::CreateRuntimeVolume> for CreateRuntimeVolume {
    fn from(value: citadel_platforms::CreateRuntimeVolume) -> Self {
        Self {
            name: value.name,
            driver: value.driver,
            labels: value.labels,
            options: value.options,
        }
    }
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PullImageInput {
    pub platform_id: Uuid,
    pub registry_id: Uuid,
    pub image_tag: String,
}

impl From<PullImageInput> for citadel_platforms::image_pull::PullImageInput {
    fn from(value: PullImageInput) -> Self {
        Self {
            platform_id: value.platform_id,
            registry_id: value.registry_id,
            image_tag: value.image_tag,
        }
    }
}

impl From<citadel_platforms::image_pull::PullImageInput> for PullImageInput {
    fn from(value: citadel_platforms::image_pull::PullImageInput) -> Self {
        Self {
            platform_id: value.platform_id,
            registry_id: value.registry_id,
            image_tag: value.image_tag,
        }
    }
}
