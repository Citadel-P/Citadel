#![forbid(unsafe_code)]
mod capabilities;
pub use capabilities::{
    ContainerInventoryPort, ContainerStatsPort, ImageInventoryPort, NetworkInventoryPort,
    NetworkMutationPort, NetworkObservationPort, PlatformInfoPort, PlatformStatsPort,
    SwarmInventoryPort, VolumeInventoryPort, VolumeMutationPort, VolumeObservationPort,
};
pub mod model;
pub use model::{ConnectorKind, PlatformKind};
pub mod agent_setup;
pub mod containers;
pub mod deletion;
pub mod image_pull;
pub mod images;
mod inventory;
pub mod jobs;
pub mod logs;
pub mod management;
mod mutations;
pub mod node_agents;
pub mod prune;
pub mod read_models;
mod registration;
pub mod repository;
pub mod service;
mod statistics;
pub mod swarm_mutations;
pub mod swarm_overview;
pub mod terminal;
pub mod volume_content;
pub use inventory::{
    ContainerStatsStore, InventoryProjectionChange, InventoryProjectionStore,
    PlatformInventoryPort, RuntimeContainerStat, RuntimeContainerStatsStream, RuntimeImageSummary,
    RuntimeInventorySnapshot, RuntimeNetworkSummary, RuntimeSwarmConfig, RuntimeSwarmInventory,
    RuntimeSwarmNode, RuntimeSwarmResourceMount, RuntimeSwarmSecret, RuntimeSwarmService,
    RuntimeSwarmTask, RuntimeVolumeSummary,
};
pub use mutations::{
    CreateRuntimeNetwork, CreateRuntimeVolume, CreatedRuntimeNetwork, PlatformResourceMutationPort,
    RuntimeConfigFrom, RuntimeIpam, RuntimeIpamConfig,
};
pub use read_models::{
    ContainerDeploymentSummary, ContainerDeploymentUpdateState, ContainerDetails,
    ContainerIdentity, ContainerStatSnapshot, EffectivePlatformPermission, ImageDetails,
    ImageIdentity, NetworkDetails, NodeResourceProjection, PlatformDetails, PlatformStatSnapshot,
    PlatformTelemetryContext, SwarmConfigSummary, SwarmNetworkSummary, SwarmNodeSummary,
    SwarmSecretSummary, SwarmServiceSummary, SwarmTaskSummary, VolumeDetails,
    VolumeUsageDataSummary, WorkloadStatusCounts,
};
pub use registration::{
    CreatePlatformInput, LOCAL_DOCKER_ADDRESS, PlatformConnectorType, PlatformRegistration,
    PlatformRegistrationError, PlatformRegistrationRepository, PlatformRegistrationRuntime,
    PlatformRegistrationService, PlatformType,
};
pub use repository::PlatformReader;
pub use service::PlatformReadService;
pub use statistics::{
    ServiceStatIdentity, ServiceStatistics, ServiceTaskSample, StatisticsContainer,
    StatisticsReader, StatisticsWorkload, StatsWindow, SwarmTaskRuntimePort, validate_running_task,
};
pub mod runtime;
pub use runtime::{
    AuthorizedPlatformReader, AuthorizedReadError, HostDiskUsage, PlatformHealthPort,
    PlatformRuntimePort, PlatformSummary, RuntimeCapabilityError, RuntimeContainerSummary,
    RuntimeErrorKind, RuntimePlatformInfo, RuntimePlatformStats, RuntimeStatsStream,
    RuntimeSwarmInfo, RuntimeSwarmPeer,
};

mod metadata;
pub use metadata::{PlatformMetadataError, PlatformMetadataRepository};

pub mod stats_ingestion;

mod descriptor;
pub use descriptor::{PlatformDescriptor, PlatformRoutingMetadata};

pub mod resource_mutations;

pub mod edge_management;

pub mod image_mutations;

pub mod runtime_provider;
