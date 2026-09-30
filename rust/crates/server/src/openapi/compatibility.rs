//! Frozen schemas at the remaining dynamic JSON compatibility boundary.
//! New contracts use real DTOs with `ToSchema`.
use std::{collections::BTreeMap, sync::LazyLock};
use utoipa::{
    PartialSchema, ToSchema,
    openapi::{RefOr, schema::Schema},
};

fn all() -> &'static BTreeMap<String, RefOr<Schema>> {
    static SCHEMAS: LazyLock<BTreeMap<String, RefOr<Schema>>> = LazyLock::new(|| {
        serde_json::from_str(include_str!("compatibility.json"))
            .expect("compatibility schemas are valid OpenAPI")
    });
    &SCHEMAS
}

pub fn schemas() -> BTreeMap<String, RefOr<Schema>> {
    all().clone()
}

// The stats query is decoded as an integer and validated by StatsWindow, rather
// than a serde enum. Preserve its existing 24/48/72 hour schema at that boundary.
pub struct StatsHours;
impl PartialSchema for StatsHours {
    fn schema() -> RefOr<Schema> {
        all()["StatsHours"].clone()
    }
}
impl ToSchema for StatsHours {}

// Schema descriptors for values serialized at the dynamic JSON boundary.
// Runtime DTOs reference these explicitly instead of exposing untyped JSON.
macro_rules! wire_schema {
    ($($name:ident),* $(,)?) => {$(
        pub struct $name;
        impl PartialSchema for $name {
            fn schema() -> RefOr<Schema> { all()[stringify!($name)].clone() }
        }
        impl ToSchema for $name {}
    )*};
}
wire_schema!(
    ActivityEventInfo,
    ActionRunStatus,
    ActionRunTrigger,
    BackupExecutionLocation,
    BackupRepositorySpec,
    BackupRepositoryStatus,
    BackupRepositoryType,
    BackupRepositoryValidationStatus,
    BackupRestoreStatus,
    BackupRunItemStatus,
    BackupRunStatus,
    BackupRunTrigger,
    BackupSnapshotAvailability,
    BackupSourceSpec,
    BackupWebhookConfig,
    BuildAgentPoolProvider,
    BuildAgentPoolProviderSpec,
    BuildAgentPoolValidationStatus,
    BuildProjectBuilderKind,
    BuildRunStatus,
    BuildRunTrigger,
    BuildWebhookConfig,
    ContainerStateStatus,
    ContainerSystemRole,
    GitReposStatus,
    GlobalSearchCategory,
    HostPortBinding,
    IpAddressManagementConfig,
    PlatformConnectorType,
    PlatformDescriptor,
    RegistryConfiguration,
    RegistryType,
    RepoWebhookConfig,
    SearchStatusTone,
    SwarmQuorumState,
    SwarmServiceOwnership,
    ContainerVolumeResult,
    NetworkConnectedContainer,
    NetworkPeerInfo,
    GlobalSearchResourceType,
    BackupCoverageStatus,
);
