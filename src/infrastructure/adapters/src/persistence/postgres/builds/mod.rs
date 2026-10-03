mod repository;
mod rows;
use rows::*;
mod recovery;
pub(crate) use recovery::{recover, recover_with_mode};
mod activity;
use activity::*;
mod agent_pools;
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

const AUTHORIZED_CTE: &str = r#"
WITH actor_scope AS (
    SELECT actor.id AS actorid
    FROM actors actor
    WHERE actor.id = $1 AND actor.isenabled
    UNION
    SELECT team.actorid
    FROM actorteammemberships membership
    JOIN teams team ON team.id = membership.teamid
    JOIN actors team_actor ON team_actor.id = team.actorid AND team_actor.isenabled
    WHERE membership.memberactorid = $1
), global_access AS (
    SELECT EXISTS (
        SELECT 1 FROM actor_scope scope
        JOIN actorroles assignment ON assignment.actorid = scope.actorid
        JOIN permissions permission ON permission.roleid = assignment.roleid
        WHERE permission.resourcetype = $2
          AND permission.permissionlevel = ANY($3)
    ) AS allowed
)
"#;

pub mod completion;

pub use repository::PostgresBuildRepository;

pub mod credentials;
