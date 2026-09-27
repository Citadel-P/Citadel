pub use repository::PostgresAlertRepository;
mod repository;
mod rows;
use rows::*;
mod activity;
use activity::*;
mod queries;
use queries::*;
mod channels;
mod delivery;
mod events;
mod rules;
use std::collections::HashMap;

use std::sync::Arc;

use chrono::{DateTime, Utc};

use citadel_alerts::{
    AlertChannel, AlertChannelConfiguration, AlertDeliveryClaim, AlertError, AlertEvent,
    AlertEventFilter, AlertEventPage, AlertObservation, AlertRepository, AlertRule,
    AlertRuleConfiguration, AlertRuleListItem, NewAlertEvent, is_in_quiet_hours,
};

use citadel_primitives::{ActorId, ResourceType};

use futures_util::future::BoxFuture;

use serde_json::Value;

use sqlx::{AssertSqlSafe, PgPool, Postgres, Row, Transaction};

use uuid::Uuid;

mod validation;
use citadel_alerts::rules::evaluation::{observation_matches, rule_applies};
use delivery::enqueue_deliveries;
use validation::*;

mod configuration;

pub mod observations;
