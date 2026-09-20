pub use repository::PostgresAutomationRepository;
mod repository;
mod rows;
use rows::*;
mod recovery;
pub(crate) use recovery::{recover_interrupted, recover_interrupted_with_mode};
mod activity;
use activity::*;
mod queries;
use queries::*;
mod actions;
mod runs;
mod scheduling;
use chrono::{DateTime, Utc};

use citadel_automation::{
    AutomationAction, AutomationActionConfiguration, AutomationError, AutomationRepository,
    AutomationRun, AutomationRunClaim, AutomationRunResult, code_hash,
};

use citadel_activities::{ActivityEvent, ActivityEventInfo};
use citadel_primitives::{ActorId, ResourceType};

use citadel_git::repositories::webhooks::RepoWebhookConfig;

use futures_util::future::BoxFuture;

use sqlx::{AssertSqlSafe, PgPool, Postgres, Row, Transaction};

use uuid::Uuid;

enum EnqueueMode<'a> {
    Queue,
    Execute(Option<&'a str>),
    Webhook(&'a RepoWebhookConfig),
}
