use citadel_platforms::{
    AuthorizedReadError, ContainerDeploymentSummary, ContainerDeploymentUpdateState,
    ContainerDetails, ContainerStatSnapshot, EffectivePlatformPermission, ImageDetails,
    NodeResourceProjection, PlatformDetails, PlatformReader, PlatformStatSnapshot,
    RuntimeNetworkSummary, RuntimeVolumeSummary, SwarmConfigSummary, SwarmNetworkSummary,
    SwarmNodeSummary, SwarmSecretSummary, SwarmServiceSummary, SwarmTaskSummary,
    WorkloadStatusCounts,
};
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, PgPool, Row};
use std::collections::BTreeMap;
use uuid::Uuid;
mod reader;
pub use reader::PostgresPlatformReader;
mod rows;
use rows::*;
mod queries;
use queries::*;

mod metadata;

pub use metadata::PostgresPlatformMetadataRepository;
