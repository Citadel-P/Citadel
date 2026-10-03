use crate::{
    api::{
        error::{ApiError, HttpResult, api_result, no_store},
        resources::lookup::requests::LookupQuery,
        routes::platforms::{PlatformsHttpState, lookup_platform_resources},
    },
    openapi::router::OpenApiRouterExt,
};

use axum::{
    Json, Router,
    extract::{Extension, Query, State, rejection::QueryRejection},
    http::HeaderMap,
    response::IntoResponse,
};

use citadel_discovery::{
    LookupCaller, LookupError, LookupQuery as ResourceLookupQuery, LookupReader,
    LookupResourceType, LookupResult,
};

use citadel_identity::{ActorPrincipal, EntitlementService};

use std::sync::Arc;

#[derive(Clone)]
pub struct LookupHttpState {
    pub store: Arc<dyn LookupReader>,
    pub entitlements: Arc<dyn EntitlementService>,
    pub platforms: PlatformsHttpState,
}

pub fn router(state: LookupHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<LookupHttpState> {
    utoipa_axum::router::OpenApiRouter::new().normalized_routes(utoipa_axum::routes!(lookup))
}

#[utoipa::path(
    get,
    path = "/api/v1/lookup",
    operation_id = "lookup",
    tag = "Lookup",
    summary = "Look up accessible resources",
    responses(
        (status = 200, description = "Success", body = Vec<crate::api::resources::schema_models::discovery::LookupResourceInfoSchema>, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("TargetResourceType" = crate::api::resources::vocabulary::LookupResourceTypeSchema, Query), ("SourceResourceType" = Option<crate::api::resources::vocabulary::LookupResourceTypeSchema>, Query), ("SourceResourceId" = Option<uuid::Uuid>, Query), ("PlatformId" = Option<uuid::Uuid>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn lookup(
    State(state): State<LookupHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    query: Result<Query<LookupQuery>, QueryRejection>,
) -> HttpResult {
    let Extension(principal) = api_result(principal.ok_or(ApiError::Unauthenticated), &headers)?;
    let Query(query) = api_result(
        query.map_err(|_| ApiError::Validation("Invalid lookup query parameters.".into())),
        &headers,
    )?;
    let request = ResourceLookupQuery {
        target: api_result(parse_kind(&query.target), &headers)?,
        source: api_result(
            query.source.as_deref().map(parse_kind).transpose(),
            &headers,
        )?,
        source_id: query.source_id,
        platform_id: query.platform_id,
    };
    api_result(
        request
            .validate(principal.is_administrator())
            .map_err(lookup_error),
        &headers,
    )?;
    let caller = LookupCaller {
        actor_id: principal.actor_id,
        user_id: principal.subject_id,
        administrator: principal.is_administrator(),
        service_accounts_enabled: request.target == LookupResourceType::RunAsActor
            && api_result(
                state.entitlements.custom_access_control_enabled().await,
                &headers,
            )?,
    };
    match api_result(
        state
            .store
            .lookup(&caller, &request)
            .await
            .map_err(lookup_error),
        &headers,
    )? {
        LookupResult::Rows(rows) => Ok(no_store(Json(rows).into_response())),
        LookupResult::PlatformResources { platform_id, kind } => {
            lookup_platform_resources(&state.platforms, platform_id, kind, &headers).await
        }
    }
}

fn parse_kind(value: &str) -> Result<LookupResourceType, ApiError> {
    LookupResourceType::ALL
        .iter()
        .copied()
        .find(|kind| format!("{kind:?}").eq_ignore_ascii_case(value))
        .ok_or_else(|| ApiError::Validation("Unknown lookup resource type.".into()))
}

fn lookup_error(error: LookupError) -> ApiError {
    match error {
        LookupError::Validation(message) => ApiError::Validation(message),
        LookupError::Forbidden => ApiError::Forbidden,
        LookupError::NotFound => ApiError::NotFound,
        source @ LookupError::Storage(_) => ApiError::internal(source),
    }
}
