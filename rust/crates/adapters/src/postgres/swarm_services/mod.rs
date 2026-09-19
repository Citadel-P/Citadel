use chrono::{DateTime, Utc};
use citadel_domain::{
    ActivityEvent, ActivityEventInfo, ActivityStatus, ActorId, ResourceType,
    SwarmServiceActivitySnapshot,
};
use citadel_domain::{
    EffectivePermission, PermissionLevel, PermissionPolicy, PermissionRequirement,
    SpecificPermissions,
};
use citadel_swarm_services::permissions as policy;
use citadel_swarm_services::{
    AutoUpdateState, CreateSwarmService, RenameSwarmService, RuntimeServiceResult,
    ServiceDeletionClaim, ServiceOperationClaim, ServiceOperationKind, SwarmServiceDetails,
    SwarmServiceError, SwarmServiceFilter, SwarmServiceOperation, SwarmServiceRepository,
    SwarmServiceSpec, UpdateSwarmService,
};
use futures_util::future::BoxFuture;
use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::activity_store::insert_activity;

mod adoption;
mod updates;
pub use adoption::PostgresSwarmServiceAdoption;

mod repository;
pub use repository::PostgresSwarmServiceRepository;

mod queries;
use queries::*;

mod mutations;
use mutations::*;

mod delete;

mod apply;

mod rows;
use rows::*;

mod authorization;
use authorization::*;

mod activity;
use activity::service_creation_activity;
pub(crate) use activity::{activity_snapshot, insert_swarm_activity};

pub mod bindings;
