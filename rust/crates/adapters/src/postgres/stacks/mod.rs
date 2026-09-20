use chrono::Utc;
use citadel_activities::{
    ActivityEvent, ActivityEventInfo, ActivityResourceType, ActivitySourceResource, ActivityStatus,
    StackActivitySnapshot, StackResultActivitySnapshot,
};
use citadel_primitives::ActorId;
use citadel_primitives::{
    EffectivePermission, PermissionLevel, PermissionPolicy, PermissionRequirement,
    SpecificPermissions,
};
use citadel_stacks::permissions as policy;
use citadel_stacks::{
    CreateStack, ImportComposeProject, ResourceBindingSnapshot, StackDeletionClaim, StackDetails,
    StackDriftPolicy, StackError, StackFilter, StackImportClaim, StackOperationClaim,
    StackReleaseDetails, StackReleaseSource, StackReleaseStatus, StackRepository,
    StackRuntimeResult, StackSource, StackSpec, StackStateClaim, StackUpdateState, TagSummary,
    UpdateStack, normalize_project_name,
};
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::activity_store::insert_activity;
mod drift;
mod update_checks;
mod webhooks;

const READ: i32 = 1;
mod repository;
pub use repository::PostgresStackRepository;

mod queries;
use queries::*;

mod mutations;
use mutations::*;

mod apply;

mod delete;

mod state;

mod rows;
use rows::*;

mod authorization;
use authorization::*;

mod activity;
use activity::*;

pub mod bindings;

pub mod release_resources;
