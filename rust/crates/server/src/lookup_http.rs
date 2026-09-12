use crate::identity_http::{IdentityHttpResult, identity_result, no_store};
use crate::openapi::router::OpenApiRouterExt;
use crate::platforms_http::{PlatformsHttpState, lookup_platform_resources};
use axum::extract::rejection::QueryRejection;
use axum::extract::{Extension, Query, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_domain::LookupResourceType;
use citadel_identity::{ActorPrincipal, EntitlementService, IdentityError};
use citadel_resources::{LookupCaller, LookupError, LookupRequest, LookupResult, LookupStore};
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct LookupHttpState {
    pub store: Arc<dyn LookupStore>,
    pub entitlements: Arc<dyn EntitlementService>,
    pub platforms: PlatformsHttpState,
}

pub fn router(state: LookupHttpState) -> Router {
    documented_routes().split_for_parts().0.with_state(state)
}

#[derive(Deserialize)]
struct LookupQuery {
    #[serde(rename = "TargetResourceType", alias = "targetResourceType")]
    target: String,
    #[serde(rename = "SourceResourceType", alias = "sourceResourceType")]
    source: Option<String>,
    #[serde(rename = "SourceResourceId", alias = "sourceResourceId")]
    source_id: Option<Uuid>,
    #[serde(rename = "PlatformId", alias = "platformId")]
    platform_id: Option<Uuid>,
}

#[utoipa::path(
    get,
    path = "/api/v1/lookup",
    operation_id = "lookup",
    summary = "Look up accessible resources",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/lookupResponse"), content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("TargetResourceType" = citadel_domain::LookupResourceType, Query), ("SourceResourceType" = Option<citadel_domain::LookupResourceType>, Query), ("SourceResourceId" = Option<uuid::Uuid>, Query), ("PlatformId" = Option<uuid::Uuid>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn lookup(
    State(state): State<LookupHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    query: Result<Query<LookupQuery>, QueryRejection>,
) -> IdentityHttpResult {
    let Extension(principal) =
        identity_result(principal.ok_or(IdentityError::Unauthenticated), &headers)?;
    let Query(query) = identity_result(
        query.map_err(|_| IdentityError::Validation("Invalid lookup query parameters.".into())),
        &headers,
    )?;
    let request = LookupRequest {
        target: identity_result(parse_kind(&query.target), &headers)?,
        source: identity_result(
            query.source.as_deref().map(parse_kind).transpose(),
            &headers,
        )?,
        source_id: query.source_id,
        platform_id: query.platform_id,
    };
    identity_result(
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
            && identity_result(
                state.entitlements.custom_access_control_enabled().await,
                &headers,
            )?,
    };
    match identity_result(
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

fn parse_kind(value: &str) -> Result<LookupResourceType, IdentityError> {
    LookupResourceType::ALL
        .iter()
        .copied()
        .find(|kind| format!("{kind:?}").eq_ignore_ascii_case(value))
        .ok_or_else(|| IdentityError::Validation("Unknown lookup resource type.".into()))
}

fn lookup_error(error: LookupError) -> IdentityError {
    match error {
        LookupError::Validation(message) => IdentityError::Validation(message),
        LookupError::Forbidden => IdentityError::Forbidden,
        LookupError::NotFound => IdentityError::NotFound,
        LookupError::Storage(message) => IdentityError::Storage(message),
    }
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<LookupHttpState> {
    utoipa_axum::router::OpenApiRouter::new().normalized_routes(utoipa_axum::routes!(lookup))
}
