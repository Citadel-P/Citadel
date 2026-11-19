mod repository;
mod rows;
use crate::persistence::postgres::authorization::AUTHORIZED_BY_LEVELS_CTE as AUTHORIZED_CTE;
use rows::*;
mod recovery;
pub(crate) use recovery::{recover, recover_with_mode};
mod activity;
use activity::*;
mod admission;
mod agent_pools;
mod cleanup;
mod deferral;
mod projects;
mod runs;
use chrono::{DateTime, Utc};

use citadel_builds::{
    BuildAgentPool, BuildAgentPoolConfiguration, BuildClaim, BuildError, BuildExecutionResult,
    BuildProject, BuildProjectConfiguration, BuildRepository, BuildRun,
};

use citadel_primitives::{ActorId, ResourceType};

use futures_util::future::BoxFuture;

use sqlx::{AssertSqlSafe, PgPool, Postgres, Row, Transaction};

use uuid::Uuid;

use crate::persistence::postgres::tags::links as resource_tags;

pub mod completion;

pub use repository::PostgresBuildRepository;

pub mod credentials;

pub(crate) mod execution_target;
