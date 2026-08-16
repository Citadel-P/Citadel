use std::sync::Arc;

use axum::Json;
use axum::Router;
use axum::extract::rejection::QueryRejection;
use axum::extract::{Extension, Path, Query, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use citadel_application::{
    ActivityFilter, ActivityRecord, ActivityService, IdentityError, PagedActivityRecords,
};
use citadel_contracts::http::routes;
use citadel_domain::{
    ActivityEventType, ActivityResourceType, ActivityStatus, ActorPrincipal, ActorType,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

use crate::contract_router::ContractRouterExt;
use crate::identity_http::{identity_error_response, no_store};

#[derive(Clone)]
pub struct ActivitiesHttpState {
    pub activities: Arc<ActivityService>,
}

pub fn router(state: ActivitiesHttpState) -> Router {
    Router::new()
        .contract_route(routes::LIST_ACTIVITIES, list)
        .contract_route(routes::GET_ACTIVITY, get_by_id)
        .with_state(state)
}

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivityFilterQuery {
    #[serde(alias = "ResourceId")]
    resource_id: Option<Uuid>,
    #[serde(alias = "ResourceType")]
    resource_type: Option<ActivityResourceType>,
    #[serde(alias = "EventType")]
    event_type: Option<ActivityEventType>,
    #[serde(alias = "Page")]
    page: Option<i32>,
    #[serde(alias = "PageSize")]
    page_size: Option<i32>,
}

impl From<ActivityFilterQuery> for ActivityFilter {
    fn from(value: ActivityFilterQuery) -> Self {
        Self {
            resource_id: value.resource_id,
            resource_type: value.resource_type,
            event_type: value.event_type,
            page: value.page,
            page_size: value.page_size,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivityView {
    id: Uuid,
    platform_id: Option<Uuid>,
    resource_id: Option<Uuid>,
    platform_name: String,
    resource_name: String,
    platform_status: String,
    resource_type: ActivityResourceType,
    event_type: ActivityEventType,
    status: ActivityStatus,
    created_at: chrono::DateTime<chrono::Utc>,
    info: Value,
    actor_id: Uuid,
    actor_name: String,
    actor_type: ActorType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PagedActivityView {
    items: Vec<ActivityView>,
    total_count: i64,
    page: i32,
    page_size: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivitiesView {
    paged_result: PagedActivityView,
}

async fn get_by_id(
    State(state): State<ActivitiesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    headers: HeaderMap,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return identity_error_response(IdentityError::Unauthenticated, &headers);
    };
    match state.activities.get(&principal, id).await {
        Ok(activity) => match map_activity(activity) {
            Ok(activity) => no_store(Json(activity).into_response()),
            Err(error) => identity_error_response(error, &headers),
        },
        Err(error) => identity_error_response(error, &headers),
    }
}

async fn list(
    State(state): State<ActivitiesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    query: Result<Query<ActivityFilterQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return identity_error_response(IdentityError::Unauthenticated, &headers);
    };
    let Ok(Query(query)) = query else {
        return identity_error_response(
            IdentityError::Validation("Activity filters are invalid.".to_owned()),
            &headers,
        );
    };
    match state.activities.list(&principal, query.into()).await {
        Ok(activities) => match map_activities(activities) {
            Ok(activities) => no_store(Json(activities).into_response()),
            Err(error) => identity_error_response(error, &headers),
        },
        Err(error) => identity_error_response(error, &headers),
    }
}

fn map_activities(records: PagedActivityRecords) -> Result<ActivitiesView, IdentityError> {
    Ok(ActivitiesView {
        paged_result: PagedActivityView {
            items: records
                .items
                .into_iter()
                .map(map_activity)
                .collect::<Result<Vec<_>, _>>()?,
            total_count: records.total_count,
            page: records.page,
            page_size: records.page_size,
        },
    })
}

fn map_activity(record: ActivityRecord) -> Result<ActivityView, IdentityError> {
    let info = serde_json::from_str::<Value>(&record.info_json)
        .map_err(|error| IdentityError::Storage(error.to_string()))?;
    Ok(ActivityView {
        id: record.id,
        platform_id: record.platform_id,
        resource_id: record.resource_id,
        platform_name: record.platform_name,
        resource_name: record.resource_name,
        platform_status: record.platform_status,
        resource_type: record.resource_type,
        event_type: record.event_type,
        status: record.status,
        created_at: record.created_at,
        info: camel_case_json_keys(info),
        actor_id: record.actor_id,
        actor_name: record.actor_name,
        actor_type: record.actor_type,
    })
}

fn camel_case_json_keys(value: Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| (camel_case_key(key), camel_case_json_keys(value)))
                .collect::<Map<_, _>>(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(camel_case_json_keys).collect()),
        value => value,
    }
}

fn camel_case_key(mut key: String) -> String {
    if key.starts_with('$') {
        return key;
    }
    let Some(first) = key.get_mut(0..1) else {
        return key;
    };
    first.make_ascii_lowercase();
    key
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn dotnet_persistence_info_is_mapped_to_the_frontend_contract() {
        let value = json!({
            "$type": "UserProfileUpdated",
            "Changes": [{
                "Name": "DisplayName",
                "OldValue": "Old",
                "NewValue": "New"
            }]
        });

        assert_eq!(
            camel_case_json_keys(value),
            json!({
                "$type": "UserProfileUpdated",
                "changes": [{
                    "name": "DisplayName",
                    "oldValue": "Old",
                    "newValue": "New"
                }]
            })
        );
    }
}
