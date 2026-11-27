use super::activity;
use crate::persistence::postgres::identity::authorization_cache::{Impact, Mutation};
use citadel_activities::ActivityEventInfo;
use citadel_dashboards::{
    DashboardContent, DashboardError as Error, DashboardRepository, DashboardSharingMode,
    DashboardSummary, DashboardViewer, DashboardVisibility, SavedDashboard,
};
use citadel_primitives::ResourceType;
use futures_util::future::BoxFuture;
use sqlx::{PgConnection, PgPool, Row, postgres::PgRow};
use uuid::Uuid;

type ResultFuture<'a, T> = BoxFuture<'a, Result<T, Error>>;
const ACCESS: &str = concat!(
    include_str!("../enabled_actor_scope.sql"),
    ", dashboard_viewer AS (SELECT $2::boolean AS administrator)",
    include_str!("visibility.sql")
);
const _: () = assert!(ResourceType::Dashboard as i32 == 22);

pub struct PostgresDashboardRepository {
    pool: PgPool,
}

impl PostgresDashboardRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn storage(error: sqlx::Error) -> Error {
    Error::Storage(error.to_string())
}

fn dashboard_write_error(error: sqlx::Error) -> Error {
    if error.as_database_error().is_some_and(|db| {
        db.is_unique_violation() && db.constraint() == Some("ix_dashboards_unique_name")
    }) {
        Error::Invalid("That dashboard name is already in use. Choose a different name.".into())
    } else {
        storage(error)
    }
}

fn visibility_name(visibility: DashboardVisibility) -> &'static str {
    match visibility {
        DashboardVisibility::Private => "Private",
        DashboardVisibility::InstanceShared => "InstanceShared",
        DashboardVisibility::Restricted => "Restricted",
    }
}

fn summary(row: &PgRow, viewer: DashboardViewer) -> Result<DashboardSummary, Error> {
    let visibility = match row.try_get::<&str, _>("visibility").map_err(storage)? {
        "Private" => DashboardVisibility::Private,
        "InstanceShared" => DashboardVisibility::InstanceShared,
        "Restricted" => DashboardVisibility::Restricted,
        _ => return Err(Error::Storage("Invalid dashboard visibility".into())),
    };
    Ok(DashboardSummary {
        visibility,
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        revision: row.try_get("revision").map_err(storage)?,
        can_edit: !viewer.read_only && row.try_get("can_edit").map_err(storage)?,
        can_delete: !viewer.read_only && row.try_get("can_delete").map_err(storage)?,
    })
}

async fn read(
    conn: &mut PgConnection,
    viewer: DashboardViewer,
    id: Uuid,
    lock: bool,
) -> Result<SavedDashboard, Error> {
    // Take the row lock in a separate statement so an overlapping writer's committed
    // revision is visible to the authorization and revision check under READ COMMITTED.
    if lock {
        sqlx::query("SELECT id FROM dashboards WHERE id=$1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut *conn)
            .await
            .map_err(storage)?;
    }
    let sql = format!(
        "{ACCESS} SELECT a.*, d.configuration FROM dashboard_access a JOIN dashboards d USING(id) WHERE a.id=$3 AND a.can_read"
    );
    let row = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
        .bind(viewer.actor.value())
        .bind(viewer.administrator)
        .bind(id)
        .fetch_optional(conn)
        .await
        .map_err(storage)?
        .ok_or(Error::Unavailable)?;
    Ok(SavedDashboard {
        summary: summary(&row, viewer)?,
        configuration: row.try_get("configuration").map_err(storage)?,
    })
}

impl DashboardRepository for PostgresDashboardRepository {
    fn fleet(
        &self,
        viewer: DashboardViewer,
        filter: citadel_dashboards::FleetFilter,
    ) -> ResultFuture<'_, citadel_dashboards::FleetPage> {
        Box::pin(super::fleet::read(&self.pool, viewer, filter))
    }
    fn sources(
        &self,
        viewer: DashboardViewer,
        filter: citadel_dashboards::SourceFilter,
    ) -> ResultFuture<'_, Vec<citadel_dashboards::DashboardSource>> {
        Box::pin(super::sources::read(&self.pool, viewer, filter))
    }
    fn list<'a>(
        &'a self,
        viewer: DashboardViewer,
        search: &'a str,
        limit: i64,
        offset: i64,
    ) -> ResultFuture<'a, Vec<DashboardSummary>> {
        Box::pin(async move {
            let sql = format!(
                "{ACCESS} SELECT * FROM dashboard_access WHERE can_read AND strpos(lower(name),lower($3))>0 ORDER BY lower(name),id LIMIT $4 OFFSET $5"
            );
            sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
                .bind(viewer.actor.value())
                .bind(viewer.administrator)
                .bind(search)
                .bind(limit.clamp(1, 100))
                .bind(offset.max(0))
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .iter()
                .map(|row| summary(row, viewer))
                .collect()
        })
    }
    fn get(&self, viewer: DashboardViewer, id: Uuid) -> ResultFuture<'_, SavedDashboard> {
        Box::pin(async move {
            let mut conn = self.pool.acquire().await.map_err(storage)?;
            read(&mut conn, viewer, id, false).await
        })
    }
    fn create(
        &self,
        viewer: DashboardViewer,
        id: Uuid,
        visibility: DashboardVisibility,
        mut content: DashboardContent,
    ) -> ResultFuture<'_, SavedDashboard> {
        Box::pin(async move {
            content.validate()?;
            if id.is_nil() {
                return Err(Error::Invalid("A dashboard ID is required.".into()));
            }
            if visibility != DashboardVisibility::Private && !viewer.administrator {
                return Err(Error::Forbidden);
            }
            let mode = visibility_name(visibility);
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let config = serde_json::to_value(content.configuration)
                .map_err(|e| Error::Invalid(e.to_string()))?;
            let inserted=sqlx::query("INSERT INTO dashboards(id,owneractorid,name,visibility,configuration) VALUES($1,$2,$3,$4,$5) ON CONFLICT(id) DO NOTHING")
                .bind(id).bind(viewer.actor.value()).bind(&content.name).bind(mode).bind(&config).execute(&mut *tx).await.map_err(dashboard_write_error)?.rows_affected();
            let saved = read(&mut tx, viewer, id, false).await?;
            if inserted == 0 {
                let owned: bool = sqlx::query_scalar(
                    "SELECT owneractorid IS NOT DISTINCT FROM $2 FROM dashboards WHERE id=$1",
                )
                .bind(id)
                .bind(viewer.actor.value())
                .fetch_one(&mut *tx)
                .await
                .map_err(storage)?;
                if !owned
                    || saved.summary.visibility != visibility
                    || saved.summary.revision != 1
                    || saved.summary.name != content.name
                    || saved.configuration != config
                {
                    return Err(Error::Conflict);
                }
            } else {
                // Retained activity and grants must never attach to a different board with a recycled ID.
                let retired: bool = sqlx::query_scalar("SELECT EXISTS(SELECT FROM activityevents WHERE resourcetype='Dashboard' AND resourceid=$1)")
                    .bind(id).fetch_one(&mut *tx).await.map_err(storage)?;
                if retired {
                    return Err(Error::Conflict);
                }
                activity::record(
                    &mut tx,
                    viewer,
                    id,
                    &saved.summary.name,
                    ActivityEventInfo::DashboardCreated {
                        visibility: mode.into(),
                        configuration: Some(activity::configuration_snapshot(
                            &saved.summary.name,
                            &saved.configuration,
                        )),
                    },
                )
                .await?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(saved)
        })
    }
    fn update(
        &self,
        viewer: DashboardViewer,
        id: Uuid,
        revision: i64,
        mut content: DashboardContent,
    ) -> ResultFuture<'_, SavedDashboard> {
        Box::pin(async move {
            content.validate()?;
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let saved = read(&mut tx, viewer, id, true).await?;
            if !saved.summary.can_edit {
                return Err(Error::Forbidden);
            }
            if revision != saved.summary.revision {
                return Err(Error::Conflict);
            }
            let previous = serde_json::from_value::<citadel_dashboards::DashboardConfiguration>(
                saved.configuration.clone(),
            )
            .ok()
            .filter(|old| old.schema_version == content.configuration.schema_version)
            .ok_or_else(|| {
                Error::Invalid("This dashboard contains unsupported settings.".into())
            })?;
            let changes = activity::changes(
                saved.summary.name != content.name,
                &previous,
                &content.configuration,
            );
            if changes.is_empty() {
                tx.commit().await.map_err(storage)?;
                return Ok(saved);
            }
            let config = serde_json::to_value(content.configuration)
                .map_err(|e| Error::Invalid(e.to_string()))?;
            let previous_snapshot =
                activity::configuration_snapshot(&saved.summary.name, &saved.configuration);
            let snapshot = activity::configuration_snapshot(&content.name, &config);
            let has_diff = previous_snapshot != snapshot;
            sqlx::query("UPDATE dashboards SET name=$2, configuration=$3, updatedat=now(), revision=revision+1 WHERE id=$1")
                .bind(id).bind(&content.name).bind(config).execute(&mut *tx).await.map_err(dashboard_write_error)?;
            activity::record(
                &mut tx,
                viewer,
                id,
                &content.name,
                ActivityEventInfo::DashboardUpdated {
                    changes,
                    previous_configuration: has_diff.then_some(previous_snapshot),
                    configuration: has_diff.then_some(snapshot),
                },
            )
            .await?;
            let saved = read(&mut tx, viewer, id, false).await?;
            tx.commit().await.map_err(storage)?;
            Ok(saved)
        })
    }
    fn set_visibility(
        &self,
        viewer: DashboardViewer,
        id: Uuid,
        revision: i64,
        mode: DashboardSharingMode,
        custom_access: bool,
    ) -> ResultFuture<'_, DashboardSummary> {
        Box::pin(async move {
            if !viewer.administrator {
                return Err(Error::Forbidden);
            }
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let row = sqlx::query(
                "SELECT id,name,revision,visibility,true AS can_edit,true AS can_delete
                FROM dashboards WHERE id=$1 AND (visibility<>'Private' OR owneractorid=$2)
                AND EXISTS (SELECT FROM actors WHERE id=$2 AND isenabled) FOR UPDATE",
            )
            .bind(id)
            .bind(viewer.actor.value())
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?
            .ok_or(Error::Unavailable)?;
            let mut board = summary(&row, viewer)?;
            if board.visibility == DashboardVisibility::Private {
                return Err(Error::Invalid(
                    "Personal dashboards cannot change sharing mode.".into(),
                ));
            }
            if board.revision != revision {
                return Err(Error::Conflict);
            }
            let visibility = DashboardVisibility::from(mode);
            if board.visibility != visibility {
                if visibility == DashboardVisibility::Restricted && !custom_access {
                    return Err(Error::CustomAccessRequired);
                }
                board.revision = sqlx::query_scalar("UPDATE dashboards SET visibility=$2, revision=revision+1, updatedat=now() WHERE id=$1 RETURNING revision")
                    .bind(id).bind(visibility_name(visibility)).fetch_one(&mut *tx).await.map_err(storage)?;
                board.visibility = visibility;
                activity::record(
                    &mut tx,
                    viewer,
                    id,
                    &board.name,
                    ActivityEventInfo::DashboardSharingChanged {
                        visibility: visibility_name(visibility).into(),
                    },
                )
                .await?;
            }
            tx.commit().await.map_err(storage)?;
            Ok(board)
        })
    }
    fn delete(&self, viewer: DashboardViewer, id: Uuid, revision: i64) -> ResultFuture<'_, ()> {
        Box::pin(async move {
            let mut authorization = Mutation::enter(&self.pool).await;
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let saved = read(&mut tx, viewer, id, true).await?;
            if !saved.summary.can_delete {
                return Err(Error::Forbidden);
            }
            if revision != saved.summary.revision {
                return Err(Error::Conflict);
            }
            let impact = Impact::Resources(ResourceType::Dashboard as i32, vec![id]);
            authorization
                .capture(&mut tx, &impact)
                .await
                .map_err(storage)?;
            sqlx::query("DELETE FROM resourceaccesses WHERE resourcetype=$1 AND resourceid=$2")
                .bind(ResourceType::Dashboard as i32)
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            let cleared_home =
                sqlx::query("DELETE FROM instancedashboarddefault WHERE dashboardid=$1")
                    .bind(id)
                    .execute(&mut *tx)
                    .await
                    .map_err(storage)?
                    .rows_affected()
                    != 0;
            if cleared_home {
                activity::instance_home_changed(&mut tx, viewer, None).await?;
            }
            activity::record(
                &mut tx,
                viewer,
                id,
                &saved.summary.name,
                ActivityEventInfo::DashboardDeleted {
                    visibility: visibility_name(saved.summary.visibility).into(),
                    configuration: activity::deleted_configuration_snapshot(&saved),
                },
            )
            .await?;
            sqlx::query("DELETE FROM dashboards WHERE id=$1")
                .bind(id)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            authorization.commit(tx, impact).await.map_err(storage)
        })
    }
    fn default_for(&self, viewer: DashboardViewer) -> ResultFuture<'_, Option<Uuid>> {
        Box::pin(async move {
            let sql = format!(
                "{ACCESS} SELECT a.id FROM (SELECT dashboardid,1 AS priority FROM personaldashboarddefaults WHERE actorid=$1 UNION ALL SELECT dashboardid,2 FROM instancedashboarddefault) p JOIN dashboard_access a ON a.id=p.dashboardid WHERE a.can_read ORDER BY priority LIMIT 1"
            );
            sqlx::query_scalar(sqlx::AssertSqlSafe(sql.as_str()))
                .bind(viewer.actor.value())
                .bind(viewer.administrator)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)
        })
    }
    fn set_default(
        &self,
        viewer: DashboardViewer,
        instance: bool,
        id: Option<Uuid>,
    ) -> ResultFuture<'_, ()> {
        Box::pin(async move {
            if instance && !viewer.administrator {
                return Err(Error::Forbidden);
            }
            let mut tx = self.pool.begin().await.map_err(storage)?;
            let mut dashboard_name = None;
            if let Some(id) = id {
                let sql = format!("{ACCESS} SELECT a.visibility, a.name FROM dashboard_access a JOIN dashboards d USING(id)
                    WHERE a.id=$3 AND a.can_read FOR SHARE OF d");
                let board = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()))
                    .bind(viewer.actor.value())
                    .bind(viewer.administrator)
                    .bind(id)
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(storage)?
                    .ok_or(Error::Unavailable)?;
                dashboard_name = Some(board.try_get("name").map_err(storage)?);
                if instance && board.try_get::<&str, _>("visibility").map_err(storage)? == "Private"
                {
                    return Err(Error::Invalid(
                        "Choose a shared dashboard as the instance default.".into(),
                    ));
                }
            }
            let write = match (instance, id) {
                (true, Some(id)) => sqlx::query("INSERT INTO instancedashboarddefault(dashboardid) VALUES($1) ON CONFLICT(singleton) DO UPDATE SET dashboardid=excluded.dashboardid WHERE instancedashboarddefault.dashboardid IS DISTINCT FROM excluded.dashboardid")
                    .bind(id),
                (true, None) => sqlx::query("DELETE FROM instancedashboarddefault"),
                (false, Some(id)) => sqlx::query("INSERT INTO personaldashboarddefaults(actorid,dashboardid) VALUES($1,$2) ON CONFLICT(actorid) DO UPDATE SET dashboardid=excluded.dashboardid")
                    .bind(viewer.actor.value()).bind(id),
                (false, None) => sqlx::query("DELETE FROM personaldashboarddefaults WHERE actorid=$1")
                    .bind(viewer.actor.value()),
            };
            let changed = write
                .execute(&mut *tx)
                .await
                .map_err(storage)?
                .rows_affected()
                != 0;
            if instance && changed {
                activity::instance_home_changed(&mut tx, viewer, dashboard_name).await?;
            }
            tx.commit().await.map_err(storage)
        })
    }
}
