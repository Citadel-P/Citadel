use crate::activity_store::insert_activity as insert_typed_activity;
use crate::postgres::authorization::{READ_MASK, authorized_catalog_query};
use crate::postgres::tags::TAG_SUMMARIES;
use chrono::Utc;
use citadel_activities::{ActivityEvent, ActivityEventInfo, RegistryActivitySnapshot};
use citadel_primitives::{ActorId, ResourceType};
use citadel_registries::{
    NewRegistry, RegistryDetails, RegistryError, RegistryMutationKind, RegistryPatch,
    RegistryRepository, RegistryStatus, registry_type, validate_description,
    validate_name_identifier,
};
use citadel_tags::TaggableResourceType;
use futures_util::future::BoxFuture;
use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;
mod tag_links;
use tag_links::*;
mod queries;
use queries::*;
mod rows;
use rows::*;
mod activity;
use activity::*;
mod repository;
pub use repository::PostgresRegistryRepository;
