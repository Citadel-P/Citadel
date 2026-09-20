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

pub mod containers;

pub mod deletion;

pub mod edge;

pub mod inventory;

pub mod local_target;

pub mod management;

pub mod node_agents;

pub mod nodes;

pub mod registration;

pub mod statistics;

pub mod status;

pub mod swarm;

mod authorized_reader;
pub use authorized_reader::PostgresAuthorizedPlatformReader;
pub mod classification;
