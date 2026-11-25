use crate::api::{
    error::{ApiError, HttpResult, api_result, no_store},
    request_validation::{ApiPath, ApiQuery, ValidatedJson},
    resource_access::require_human,
    resources::dashboards::*,
};
use crate::openapi::router::OpenApiRouterExt;
use axum::{
    Json, Router,
    extract::{Extension, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use citadel_dashboards::{
    DashboardContent, DashboardError, DashboardRepository, DashboardViewer, DashboardVisibility,
};
use citadel_identity::{ActorPrincipal, EntitlementService};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct DashboardsHttpState {
    pub backup_coverage:
        Option<Arc<crate::api::resources::backups::coverage::BackupCoverageService>>,
    pub dashboards: Arc<dyn DashboardRepository>,
    pub entitlements: Arc<dyn EntitlementService>,
}

impl From<DashboardError> for ApiError {
    fn from(error: DashboardError) -> Self {
        match error {
            DashboardError::Invalid(message) => Self::Validation(message),
            DashboardError::Unavailable => Self::NotFound,
            DashboardError::Forbidden => Self::Forbidden,
            DashboardError::CustomAccessRequired => Self::LicenseRequired("custom-access-control"),
            DashboardError::Conflict => Self::Conflict(error.to_string()),
            DashboardError::Storage(message) => Self::Storage(message),
        }
    }
}

fn viewer(
    principal: Option<Extension<ActorPrincipal>>,
    headers: &HeaderMap,
) -> HttpResult<DashboardViewer> {
    let actor = api_result(require_human(principal), headers)?;
    Ok(DashboardViewer {
        actor: actor.actor_id,
        administrator: actor.is_administrator(),
    })
}

pub fn router(state: DashboardsHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<DashboardsHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(sources))
        .normalized_routes(utoipa_axum::routes!(workloads))
        .normalized_routes(utoipa_axum::routes!(list))
        .normalized_routes(utoipa_axum::routes!(get))
        .normalized_routes(utoipa_axum::routes!(create))
        .normalized_routes(utoipa_axum::routes!(update))
        .normalized_routes(utoipa_axum::routes!(set_visibility))
        .normalized_routes(utoipa_axum::routes!(delete))
        .normalized_routes(utoipa_axum::routes!(get_default))
        .normalized_routes(utoipa_axum::routes!(set_personal_default))
        .normalized_routes(utoipa_axum::routes!(set_instance_default))
}

#[utoipa::path(get, path = "/api/v1/dashboards", operation_id = "listDashboards", tag = "Dashboards", summary = "List visible dashboards",
    params(DashboardListQuery),
    responses((status = 200, description = "Success", body = DashboardList, content_type = "application/json"), crate::openapi::errors::AccessErrors), security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list(
    State(state): State<DashboardsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ApiQuery(query): ApiQuery<DashboardListQuery>,
) -> HttpResult {
    let actor = viewer(principal, &headers)?;
    let rows = api_result(
        state
            .dashboards
            .list(
                actor,
                query.search.as_deref().unwrap_or(""),
                query.limit.unwrap_or(50),
                query.offset.unwrap_or(0),
            )
            .await,
        &headers,
    )?;
    Ok(no_store(
        Json(DashboardList {
            dashboards: rows.into_iter().map(Into::into).collect(),
        })
        .into_response(),
    ))
}

#[utoipa::path(get, path = "/api/v1/dashboards/{id}", operation_id = "getDashboard", tag = "Dashboards", summary = "Read a saved dashboard",
    params(("id" = Uuid, Path)),
    responses((status = 200, description = "Success", body = SavedDashboardView, content_type = "application/json"), crate::openapi::errors::AccessErrors), security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get(
    State(state): State<DashboardsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ApiPath(id): ApiPath<Uuid>,
) -> HttpResult {
    let actor = viewer(principal, &headers)?;
    let saved = api_result(state.dashboards.get(actor, id).await, &headers)?;
    Ok(no_store(
        Json(SavedDashboardView::from(saved)).into_response(),
    ))
}

#[utoipa::path(post, path = "/api/v1/dashboards", operation_id = "createDashboard", tag = "Dashboards", summary = "Create a dashboard",
    request_body = CreateDashboard,
    responses((status = 200, description = "Success", body = SavedDashboardView, content_type = "application/json"), crate::openapi::errors::ResourceMutationErrors), security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create(
    State(state): State<DashboardsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(input): ValidatedJson<CreateDashboard>,
) -> HttpResult {
    let actor = viewer(principal, &headers)?;
    if input.visibility != DashboardVisibility::Private && !actor.administrator {
        return api_result(Err(ApiError::Forbidden), &headers);
    }
    if input.visibility == DashboardVisibility::Restricted
        && !api_result(
            state.entitlements.custom_access_control_enabled().await,
            &headers,
        )?
    {
        return api_result(
            Err(ApiError::LicenseRequired("custom-access-control")),
            &headers,
        );
    }
    let content = DashboardContent {
        name: input.name,
        configuration: input.configuration,
    };
    let saved = api_result(
        state
            .dashboards
            .create(actor, input.id, input.visibility, content)
            .await,
        &headers,
    )?;
    Ok(no_store(
        Json(SavedDashboardView::from(saved)).into_response(),
    ))
}

#[utoipa::path(put, path = "/api/v1/dashboards/{id}", operation_id = "updateDashboard", tag = "Dashboards", summary = "Save dashboard content at its current revision",
    params(("id" = Uuid, Path)),
    request_body = UpdateDashboard,
    responses((status = 200, description = "Success", body = SavedDashboardView, content_type = "application/json"), crate::openapi::errors::ResourceMutationErrors), security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update(
    State(state): State<DashboardsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ApiPath(id): ApiPath<Uuid>,
    ValidatedJson(input): ValidatedJson<UpdateDashboard>,
) -> HttpResult {
    let actor = viewer(principal, &headers)?;
    let saved = api_result(
        state
            .dashboards
            .update(actor, id, input.revision, input.into())
            .await,
        &headers,
    )?;
    Ok(no_store(
        Json(SavedDashboardView::from(saved)).into_response(),
    ))
}

#[utoipa::path(put, path = "/api/v1/dashboards/{id}/visibility", operation_id = "setDashboardVisibility", tag = "Dashboards", summary = "Change a dashboard’s sharing mode",
    params(("id" = Uuid, Path)), request_body = SetDashboardVisibility,
    responses((status = 200, description = "Saved", body = DashboardSummaryView, content_type = "application/json"), crate::openapi::errors::ResourceMutationErrors),
    security(("Bearer" = [])), extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn set_visibility(
    State(state): State<DashboardsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ApiPath(id): ApiPath<Uuid>,
    ValidatedJson(input): ValidatedJson<SetDashboardVisibility>,
) -> HttpResult {
    let actor = viewer(principal, &headers)?;
    if !actor.administrator {
        return api_result(Err(ApiError::Forbidden), &headers);
    }
    let custom_access = input.visibility == citadel_dashboards::DashboardSharingMode::Restricted
        && api_result(
            state.entitlements.custom_access_control_enabled().await,
            &headers,
        )?;
    let board = api_result(
        state
            .dashboards
            .set_visibility(actor, id, input.revision, input.visibility, custom_access)
            .await,
        &headers,
    )?;
    Ok(no_store(
        Json(DashboardSummaryView::from(board)).into_response(),
    ))
}

#[utoipa::path(delete, path = "/api/v1/dashboards/{id}", operation_id = "deleteDashboard", tag = "Dashboards", summary = "Delete a saved dashboard",
    params(("id" = Uuid, Path), DashboardRevision),
    responses((status = 204, description = "Saved"), crate::openapi::errors::ResourceMutationErrors), security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn delete(
    State(state): State<DashboardsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ApiPath(id): ApiPath<Uuid>,
    ApiQuery(query): ApiQuery<DashboardRevision>,
) -> HttpResult {
    let actor = viewer(principal, &headers)?;
    api_result(
        state.dashboards.delete(actor, id, query.revision).await,
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(get, path = "/api/v1/profile/dashboard-default", operation_id = "getDashboardDefault", tag = "Dashboards", summary = "Resolve the current user’s default dashboard",
    responses((status = 200, description = "Success", body = DashboardDefault, content_type = "application/json"), crate::openapi::errors::AccessErrors), security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_default(
    State(state): State<DashboardsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> HttpResult {
    let actor = viewer(principal, &headers)?;
    let dashboard_id = api_result(state.dashboards.default_for(actor).await, &headers)?;
    Ok(no_store(
        Json(DashboardDefault { dashboard_id }).into_response(),
    ))
}

#[utoipa::path(put, path = "/api/v1/profile/dashboard-default", operation_id = "setPersonalDashboardDefault", tag = "Dashboards", summary = "Set or clear the current user’s default dashboard",
    request_body = DashboardDefault,
    responses((status = 204, description = "Saved"), crate::openapi::errors::ResourceMutationErrors), security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("human")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn set_personal_default(
    State(state): State<DashboardsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(input): ValidatedJson<DashboardDefault>,
) -> HttpResult {
    let actor = viewer(principal, &headers)?;
    api_result(
        state
            .dashboards
            .set_default(actor, false, input.dashboard_id)
            .await,
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(put, path = "/api/v1/instance/dashboard-default", operation_id = "setInstanceDashboardDefault", tag = "Dashboards", summary = "Set or clear the instance default dashboard",
    request_body = DashboardDefault,
    responses((status = 204, description = "Saved"), crate::openapi::errors::ResourceMutationErrors), security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn set_instance_default(
    State(state): State<DashboardsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(input): ValidatedJson<DashboardDefault>,
) -> HttpResult {
    let actor = viewer(principal, &headers)?;
    api_result(
        state
            .dashboards
            .set_default(actor, true, input.dashboard_id)
            .await,
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(get, path="/api/v1/dashboards/sources", operation_id="listDashboardSources", tag="Dashboards",
    summary="Find readable monitoring sources and check pinned resource tag membership",
    params(DashboardSourceQuery, ("tags"=Option<Vec<String>>, Query, description="Match all resource tag names")),
    responses((status=200,description="Bounded resource metadata",body=Vec<DashboardSourceView>),crate::openapi::errors::AccessErrors),
    security(("Bearer"=[])), extensions(("x-citadel-principal"=json!("human")),("x-citadel-public"=json!(false)),("x-citadel-setup-exempt"=json!(false))))]
async fn sources(
    State(state): State<DashboardsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ApiQuery(query): ApiQuery<DashboardSourceQuery>,
    axum::extract::RawQuery(raw): axum::extract::RawQuery,
) -> HttpResult {
    let actor = viewer(principal, &headers)?;
    let tags = api_result(
        crate::api::request_validation::parse_tag_filters(raw.as_deref()),
        &headers,
    )?;
    let ids = query
        .ids
        .as_deref()
        .into_iter()
        .flat_map(|ids| ids.split(','))
        .take(9)
        .map(Uuid::parse_str)
        .collect::<Result<Vec<_>, _>>();
    let ids = api_result(
        ids.map_err(|_| ApiError::Validation("Invalid pinned resource ID.".into())),
        &headers,
    )?;
    let search = query.search.unwrap_or_default();
    let limit = query.limit.unwrap_or(10);
    if ids.len() > 8 || search.chars().count() > 128 || !(1..=25).contains(&limit) {
        return api_result(
            Err(ApiError::Validation(
                "Search up to 128 characters, select up to eight IDs, and request 1–25 results."
                    .into(),
            )),
            &headers,
        );
    }
    let filter = citadel_dashboards::SourceFilter {
        kind: query.kind,
        search,
        ids,
        tags,
        limit: i64::from(limit),
    };
    let rows = api_result(state.dashboards.sources(actor, filter).await, &headers)?;
    Ok(no_store(
        Json(
            rows.into_iter()
                .map(DashboardSourceView::from)
                .collect::<Vec<_>>(),
        )
        .into_response(),
    ))
}

#[utoipa::path(get, path="/api/v1/dashboards/workloads", operation_id="listDashboardWorkloads", tag="Dashboards",
    summary="List readable workloads with persisted monitoring and backup coverage",
    params(DashboardWorkloadQuery, ("tags"=Option<Vec<String>>, Query, description="Match all resource tag names")),
    responses((status=200,description="Bounded workload rows",body=Vec<DashboardWorkloadView>),crate::openapi::errors::AccessErrors),
    security(("Bearer"=[])), extensions(("x-citadel-principal"=json!("human")),("x-citadel-public"=json!(false)),("x-citadel-setup-exempt"=json!(false))))]
async fn workloads(
    State(state): State<DashboardsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ApiQuery(query): ApiQuery<DashboardWorkloadQuery>,
    axum::extract::RawQuery(raw): axum::extract::RawQuery,
) -> HttpResult {
    let principal = api_result(require_human(principal), &headers)?;
    let tags = api_result(
        crate::api::request_validation::parse_tag_filters(raw.as_deref()),
        &headers,
    )?;
    let limit = query.limit.unwrap_or(5);
    if !(1..=10).contains(&limit) {
        return api_result(
            Err(ApiError::Validation(
                "Request between one and ten workloads.".into(),
            )),
            &headers,
        );
    }
    let viewer = DashboardViewer {
        actor: principal.actor_id,
        administrator: principal.is_administrator(),
    };
    let rows = api_result(
        state
            .dashboards
            .workloads(viewer, query.kind, tags, limit.into())
            .await,
        &headers,
    )?;
    let mut statuses = match &state.backup_coverage {
        Some(service) => {
            service
                .dashboard_statuses(&principal, query.kind, &rows)
                .await
        }
        None => Default::default(),
    };
    Ok(no_store(
        Json(
            rows.into_iter()
                .map(|r| DashboardWorkloadView {
                    id: r.id,
                    name: r.name,
                    platform_id: r.platform_id,
                    status: r.status,
                    processing: r.processing,
                    observed_probes: r.probes,
                    more_probes: r.more_probes,
                    backup_status: statuses.remove(&r.id),
                    can_view_logs: r.can_view_logs,
                })
                .collect::<Vec<_>>(),
        )
        .into_response(),
    ))
}
