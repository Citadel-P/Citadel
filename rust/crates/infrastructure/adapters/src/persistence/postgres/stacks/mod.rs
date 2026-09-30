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
    CreateStack, ImportComposeProject, ResourceBindingSnapshot, StackDeletionClaim,
    StackDriftPolicy, StackError, StackFilter, StackImportClaim, StackOperationClaim, StackRelease,
    StackReleaseSource, StackReleaseStatus, StackRepository, StackRuntimeResult, StackSource,
    StackSpec, StackStateClaim, StackUpdateState, UpdateStack, normalize_project_name,
};
use citadel_tags::TagSummary;
use futures_util::future::BoxFuture;
use serde_json::{Value, json};
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::persistence::postgres::activities::store::insert_activity;
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

pub mod build_images;
