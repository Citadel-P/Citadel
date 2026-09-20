use citadel_discovery::GlobalSearchMatch;
use citadel_discovery::GlobalSearchParent;
use citadel_discovery::GlobalSearchReader;
use citadel_discovery::SearchError;
use citadel_discovery::ValidatedSearch;
use citadel_primitives::{ActorId, ResourceType};
use futures_util::future::BoxFuture;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct PostgresGlobalSearchStore {
    pool: PgPool,
}
impl PostgresGlobalSearchStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl GlobalSearchReader for PostgresGlobalSearchStore {
    fn search<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        query: &'a ValidatedSearch,
    ) -> BoxFuture<'a, Result<Vec<GlobalSearchMatch>, SearchError>> {
        Box::pin(async move {
            let escaped = query
                .query
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            let types: Vec<i32> = query
                .resource_types
                .iter()
                .map(|kind| *kind as i32)
                .collect();
            let rows = sqlx::query(include_str!("global_search.sql"))
                .bind(actor.value())
                .bind(administrator)
                .bind(types)
                .bind(&query.query)
                .bind(format!("{escaped}%"))
                .bind(format!("%{escaped}%"))
                .bind(query.limit_per_type)
                .bind(40_i64)
                .bind(7_i32)
                .bind(ResourceType::Platform as i32)
                .bind(ResourceType::Stack as i32)
                .bind(ResourceType::Deployment as i32)
                .bind(ResourceType::GitRepository as i32)
                .bind(ResourceType::Registry as i32)
                .bind(ResourceType::AutomationAction as i32)
                .bind(ResourceType::BackupPolicy as i32)
                .bind(ResourceType::BackupRepository as i32)
                .bind(ResourceType::Build as i32)
                .bind(ResourceType::BuildAgentPool as i32)
                .bind(ResourceType::SwarmService as i32)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?;
            rows.into_iter()
                .map(|row| {
                    let parent_id: Option<Uuid> = row.try_get("parentid").map_err(storage)?;
                    let parent_kind: Option<i32> =
                        row.try_get("parentresourcetype").map_err(storage)?;
                    let parent_name: Option<String> = row.try_get("parentname").map_err(storage)?;
                    let parent = match (parent_id, parent_kind, parent_name) {
                        (Some(id), Some(kind), Some(name)) => Some(GlobalSearchParent {
                            id,
                            resource_type: resource_type(kind)?,
                            name,
                        }),
                        _ => None,
                    };
                    Ok(GlobalSearchMatch {
                        id: row.try_get("id").map_err(storage)?,
                        resource_type: resource_type(
                            row.try_get("resourcetype").map_err(storage)?,
                        )?,
                        name: row.try_get("name").map_err(storage)?,
                        secondary_text: row.try_get("secondarytext").map_err(storage)?,
                        status: row.try_get("status").map_err(storage)?,
                        parent,
                    })
                })
                .collect()
        })
    }
}
fn storage(error: sqlx::Error) -> SearchError {
    SearchError::Storage(error.to_string())
}
fn resource_type(kind: i32) -> Result<ResourceType, SearchError> {
    ResourceType::from_i32(kind)
        .ok_or_else(|| SearchError::Storage("Invalid search resource type.".into()))
}
