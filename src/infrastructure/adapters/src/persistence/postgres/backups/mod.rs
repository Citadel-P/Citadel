use crate::persistence::postgres::authorization::AUTHORIZED_BY_LEVELS_CTE as AUTHORIZED_CTE;
use citadel_backups::runs::read_models as summaries;
mod repository;
mod rows;
use rows::*;
pub mod data_admission;
mod data_helpers;
mod leases;
mod recovery;
pub(crate) use recovery::{recover_stale, recover_stale_with_mode};
mod activity;
pub mod coverage;
mod policies;
mod repositories;
mod restores;
mod runs;
use citadel_backups::{
    policies::metadata as policy_metadata, repositories::patch as repository_patch,
};

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use citadel_backups::*;

use citadel_primitives::{ActorId, ResourceType};

use futures_util::future::BoxFuture;

use serde_json::Value;

use sqlx::{AssertSqlSafe, PgPool, Row};

use uuid::Uuid;

use crate::persistence::postgres::tags::links as resource_tags;

pub use repository::PostgresBackupPersistence;

pub mod secrets;

pub mod source_planner;

pub(crate) mod instance_identity;
