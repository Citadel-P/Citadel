use chrono::{DateTime, Utc};
use citadel_activities::{
    ActivityEvent, ActivityEventInfo, ActivityResourceType, ActivitySourceResource, ActivityStatus,
    DeploymentActivitySnapshot, DeploymentResultActivitySnapshot,
};
use citadel_deployments::permissions::{
    ApplyDeployment, DeleteDeployment, ReadDeployment, ReadDeploymentBindings, WriteDeployment,
};
use citadel_deployments::{
    ApplyClaim, AutoUpdateState, CreateDeployment, DeletionClaim, Deployment,
    DeploymentBindingSnapshot, DeploymentDetails, DeploymentDraft, DeploymentDuplicateDraft,
    DeploymentError, DeploymentFilter, DeploymentImageInfo, DeploymentRepository, DeploymentSpec,
    DuplicateSource, DuplicateWarning, FieldPatch, RuntimeContainerState, RuntimeDeploymentResult,
    TagSummary, UpdateDeploymentMetadata,
};
use citadel_primitives::{
    ActorId, EffectivePermission, PermissionLevel, PermissionPolicy, PermissionRequirement,
    SpecificPermissions,
};
use futures_util::future::BoxFuture;
use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::activity_store::insert_activity;

mod activity;
mod adoption;
mod authorization;
pub mod bindings;
mod claims;
mod mutations;
mod queries;
mod repository;
mod rows;
#[cfg(test)]
mod tests;
mod updates;
use activity::*;
pub use adoption::PostgresContainerAdoption;
use authorization::*;
use mutations::*;
use queries::*;
pub use repository::PostgresDeploymentRepository;
use rows::*;
const DEPLOYMENT_RESOURCE_TYPE: i32 = 1;
const PLATFORM_RESOURCE_TYPE: i32 = 0;
