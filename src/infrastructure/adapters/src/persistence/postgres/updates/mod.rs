use super::{
    activities::store::insert_activity,
    identity::{
        authorization_cache::AuthorizationCache, users::repository::lock_identity_mutations,
    },
};
use chrono::{DateTime, Utc};
use citadel_activities::{ActivityEvent, ActivityEventInfo, ActivityStatus};
use citadel_identity::{ADMIN_ROLE_ID, SYSTEM_ACTOR_ID};
use citadel_primitives::ActorId;
use citadel_updates::{
    CheckState, CompletionReceipt, CoreIdentity, ReleaseCheckRepository, ReleaseNotice,
    UpdateActor, UpdateError, UpdateJournal, UpdateOperation, UpdatePhase, UpdatePlan,
    UpdateRepository, release_version,
};
use futures_util::future::BoxFuture;
use sqlx::{PgPool, Postgres, Row, Transaction};
use std::sync::Arc;
use uuid::Uuid;

mod activity;
mod authorization;
mod checkpoint;
mod checks;
pub(crate) use checkpoint::{CHECKPOINT_PROBE_SQL, validate_checkpoint_database};
mod claims;
mod drain;
mod preflight;
mod queries;
mod recovery;
mod repository;
mod reviews;
mod startup;
pub use preflight::read_update_database_state;

use activity::{activity, operation_activity};
use authorization::{authorize, instance};
pub use queries::read_instance_identity;
pub use repository::PostgresUpdateRepository;
use repository::storage;
pub use startup::check_startup_updates;
