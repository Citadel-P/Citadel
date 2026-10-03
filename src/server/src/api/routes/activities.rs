//! Activities HTTP routes, authorization and local request handling.
use crate::api::resources::activities::authorized::activity_access;
use crate::{
    api::{
        error::{ApiError, error_response, no_store},
        resources::activities::{
            requests::ActivityFilterQuery,
            views::{ActivitiesView, PagedActivityView, map_activity},
        },
    },
    openapi::router::OpenApiRouterExt,
    request_validation::ApiPath,
};

use axum::{
    Json, Router,
    extract::{Extension, Query, State, rejection::QueryRejection},
    http::HeaderMap,
    response::{IntoResponse, Response},
};

use citadel_activities::{ActivityError, ActivityService, PagedActivityRecords};

use citadel_identity::ActorPrincipal;

use std::sync::Arc;

use uuid::Uuid;

#[derive(Clone)]
pub struct ActivitiesHttpState {
    pub activities: Arc<ActivityService>,
}

pub fn router(state: ActivitiesHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<ActivitiesHttpState> {
    #[derive(utoipa::OpenApi)]
    #[openapi(components(schemas(crate::api::resources::vocabulary::ActivityEventTypeSchema)))]
    struct ActivitySchemas;

    utoipa_axum::router::OpenApiRouter::with_openapi(
        <ActivitySchemas as utoipa::OpenApi>::openapi(),
    )
        .normalized_routes(utoipa_axum::routes!(list))
        .normalized_routes(utoipa_axum::routes!(get_by_id))
}

#[utoipa::path(
    get,
    path = "/api/v1/activities/{id}",
    operation_id = "getActivity",
    tag = "Activities",
    summary = "Get an authorized activity",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::activities::views::ActivityView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_by_id(
    State(state): State<ActivitiesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return error_response(ApiError::Unauthenticated, &headers);
    };
    match state.activities.get(&activity_access(&principal), id).await {
        Ok(activity) => match map_activity(activity) {
            Ok(activity) => no_store(Json(activity).into_response()),
            Err(error) => error_response(error, &headers),
        },
        Err(error) => activity_error_response(error, &headers),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/activities",
    operation_id = "listActivities",
    tag = "Activities",
    summary = "List authorized activities",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::activities::views::ActivitiesView, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("ResourceId" = Option<uuid::Uuid>, Query), ("ResourceType" = Option<crate::api::resources::vocabulary::ActivityResourceTypeSchema>, Query), ("EventType" = Option<crate::api::resources::vocabulary::ActivityEventTypeSchema>, Query), ("Page" = Option<i32>, Query, minimum = 1, extensions(("x-citadel-default" = json!(1)))), ("PageSize" = Option<i32>, Query, minimum = 1, maximum = 500, extensions(("x-citadel-default" = json!(50))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list(
    State(state): State<ActivitiesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    query: Result<Query<ActivityFilterQuery>, QueryRejection>,
    headers: HeaderMap,
) -> Response {
    let Some(Extension(principal)) = principal else {
        return error_response(ApiError::Unauthenticated, &headers);
    };
    let Ok(Query(query)) = query else {
        return error_response(
            ApiError::Validation("Activity filters are invalid.".to_owned()),
            &headers,
        );
    };
    match state
        .activities
        .list(&activity_access(&principal), query.into())
        .await
    {
        Ok(activities) => match map_activities(activities) {
            Ok(activities) => no_store(Json(activities).into_response()),
            Err(error) => error_response(error, &headers),
        },
        Err(error) => activity_error_response(error, &headers),
    }
}

fn map_activities(records: PagedActivityRecords) -> Result<ActivitiesView, ApiError> {
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

impl From<ActivityError> for ApiError {
    fn from(error: ActivityError) -> Self {
        match error {
            ActivityError::Validation(message) => Self::Validation(message),
            ActivityError::NotFound => Self::NotFound,
            source @ ActivityError::Storage(_) => Self::internal(source),
        }
    }
}

fn activity_error_response(error: ActivityError, headers: &HeaderMap) -> Response {
    error_response(error, headers)
}

#[cfg(test)]
mod tests {
    use crate::api::resources::activities::presentation::public_activity_info;
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
            public_activity_info(value),
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
