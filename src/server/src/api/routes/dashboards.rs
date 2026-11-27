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
use citadel_identity::{ActorPrincipal, EntitlementService, IdentityService};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct DashboardsHttpState {
    pub dashboards: Arc<dyn DashboardRepository>,
    pub entitlements: Arc<dyn EntitlementService>,
    pub identity: Arc<IdentityService>,
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
    state: &DashboardsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    headers: &HeaderMap,
) -> HttpResult<DashboardViewer> {
    let actor = api_result(require_human(principal), headers)?;
    Ok(dashboard_viewer(&state.identity, &actor))
}

fn writer(
    state: &DashboardsHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    headers: &HeaderMap,
) -> HttpResult<DashboardViewer> {
    let actor = viewer(state, principal, headers)?;
    if actor.read_only {
        return api_result(Err(ApiError::Forbidden), headers);
    }
    Ok(actor)
}

pub fn router(state: DashboardsHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<DashboardsHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(fleet))
        .normalized_routes(utoipa_axum::routes!(sources))
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
    let actor = viewer(&state, principal, &headers)?;
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
    let actor = viewer(&state, principal, &headers)?;
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
    let actor = writer(&state, principal, &headers)?;
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
    let actor = writer(&state, principal, &headers)?;
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
    let actor = writer(&state, principal, &headers)?;
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
    let actor = writer(&state, principal, &headers)?;
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
    let actor = viewer(&state, principal, &headers)?;
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
    let actor = writer(&state, principal, &headers)?;
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
    let actor = writer(&state, principal, &headers)?;
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
    let actor = viewer(&state, principal, &headers)?;
    let tags = api_result(
        crate::api::request_validation::parse_tag_filters(raw.as_deref()),
        &headers,
    )?;
    let filter = api_result(
        query
            .filter(tags)
            .map_err(|e| ApiError::Validation(e.into())),
        &headers,
    )?;
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

#[utoipa::path(get,path="/api/v1/dashboards/fleet",operation_id="getDashboardFleet",tag="Dashboards",
 summary="Count or investigate readable resources in the selected scope",
 params(DashboardFleetQuery, ("tags"=Option<Vec<String>>,Query,description="Match all owner tags")),
 responses((status=200,description="Scoped counts and bounded resource rows",body=crate::api::resources::schema_models::dashboards::DashboardFleetPageSchema),crate::openapi::errors::AccessErrors),
 security(("Bearer"=[])),extensions(("x-citadel-principal"=json!("human")),("x-citadel-public"=json!(false)),("x-citadel-setup-exempt"=json!(false))))]
async fn fleet(
    State(state): State<DashboardsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ApiQuery(q): ApiQuery<DashboardFleetQuery>,
    axum::extract::RawQuery(raw): axum::extract::RawQuery,
) -> HttpResult {
    let actor = viewer(&state, principal, &headers)?;
    let tags = api_result(
        crate::api::request_validation::parse_tag_filters(raw.as_deref()),
        &headers,
    )?;
    let filter = api_result(
        q.filter(tags).map_err(|e| ApiError::Validation(e.into())),
        &headers,
    )?;
    let page = api_result(state.dashboards.fleet(actor, filter).await, &headers)?;
    Ok(no_store(Json(page).into_response()))
}
