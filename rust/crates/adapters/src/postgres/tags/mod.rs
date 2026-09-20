use crate::postgres::authorization::{AUTHORIZED_CTE, READ_MASK};
use chrono::Utc;
use citadel_primitives::{ActorId, ResourceType};
use citadel_tags::{
    NewTag, Tag, TagError, TagPatch, TagRepository, TagSummary, TaggableResourceType,
};
use futures_util::future::BoxFuture;
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;
mod queries;
use queries::*;
mod rows;
use rows::*;
mod repository;
pub use repository::PostgresTagRepository;

pub(crate) use queries::{
    TAG_SUMMARIES, insert_resource_tags, replace_resource_tags_tx, validate_tag_ids,
};
