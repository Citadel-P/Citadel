pub use repository::PostgresScriptRepository;
pub(crate) mod executors;
mod repository;
mod rows;
use rows::*;
mod recovery;
pub(crate) use recovery::{recover_interrupted, recover_interrupted_with_mode};
mod activity;
use activity::*;
mod queries;
use queries::*;
mod definitions;
mod runs;
mod scheduling;
use chrono::{DateTime, Utc};

use citadel_scripts::{
    Script, ScriptClaimResult, ScriptConfiguration, ScriptDraft, ScriptError, ScriptRepository,
    ScriptRun, ScriptRunClaim, ScriptRunResult, code_hash,
};

use citadel_activities::{ActivityEvent, ActivityEventInfo};
use citadel_primitives::{ActorId, ResourceType};

use citadel_primitives::WebhookConfig;

use futures_util::future::BoxFuture;

use sqlx::{AssertSqlSafe, PgPool, Postgres, Row, Transaction};

use uuid::Uuid;

enum EnqueueMode<'a> {
    Queue,
    Draft(Option<&'a ScriptDraft>),
    Webhook(&'a WebhookConfig),
}
