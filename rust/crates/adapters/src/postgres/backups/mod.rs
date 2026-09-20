use citadel_backups::runs::read_models as summaries;
mod repository;
mod rows;
use rows::*;
mod recovery;
pub(crate) use recovery::{recover_stale, recover_stale_with_mode};
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

use crate::resource_tags;

pub(crate) const AUTHORIZED_CTE: &str = r#"
WITH actor_scope AS (
    SELECT actor.id AS actorid FROM actors actor
    WHERE actor.id=$1 AND actor.isenabled
    UNION
    SELECT team.actorid
    FROM actorteammemberships membership
    JOIN teams team ON team.id=membership.teamid
    JOIN actors team_actor ON team_actor.id=team.actorid AND team_actor.isenabled
    WHERE membership.memberactorid=$1
), global_access AS (
    SELECT EXISTS (
        SELECT 1 FROM actor_scope scope
        JOIN actorroles assignment ON assignment.actorid=scope.actorid
        JOIN permissions permission ON permission.roleid=assignment.roleid
        WHERE permission.resourcetype=$2
          AND permission.permissionlevel = ANY($3)
    ) AS allowed
)
"#;

pub use repository::PostgresBackupPersistence;
