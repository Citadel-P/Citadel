use super::{UpdateHttpState, instance::actor};
use crate::api::resources::updates::{
    AgentOperationView, AgentReviewView, AgentScheduleView, AgentUpdatesView, ApproveUpdateInput,
    ReviewAgentsInput,
};
use crate::{
    api::{
        error::{ApiError, error_response, no_store},
        resource_access::require_human_administrator,
    },
    openapi::router::OpenApiRouterExt,
};
use axum::{
    Json,
    extract::{Extension, Path, State},
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use citadel_identity::ActorPrincipal;
use citadel_updates::AgentUpdateService;
use tokio_util::sync::CancellationToken;

pub(super) fn routes() -> utoipa_axum::router::OpenApiRouter<UpdateHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(status))
        .normalized_routes(utoipa_axum::routes!(operation))
        .normalized_routes(utoipa_axum::routes!(review))
        .normalized_routes(utoipa_axum::routes!(approve))
        .normalized_routes(utoipa_axum::routes!(schedule))
}
fn service(state: &UpdateHttpState) -> Result<&AgentUpdateService, ApiError> {
    state
        .agents
        .as_ref()
        .ok_or_else(|| ApiError::Conflict("Agent updates are not configured.".into()))
}
fn response(result: Result<impl IntoResponse, ApiError>, headers: &HeaderMap) -> Response {
    match result {
        Ok(view) => no_store(view.into_response()),
        Err(error) => error_response(error, headers),
    }
}

#[utoipa::path(get,path="/api/v1/updates/agents",operation_id="getAgentUpdates",tag="Updates",summary="Get Agent availability, schedule and durable progress",
responses((status=200,description="Success",body=AgentUpdatesView),crate::openapi::errors::AccessErrors),security(("Bearer"=[])),
extensions(("x-citadel-principal"=json!("administrator")),("x-citadel-public"=json!(false)),("x-citadel-setup-exempt"=json!(false))))]
async fn status(
    State(state): State<UpdateHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> Response {
    response(
        async {
            require_human_administrator(principal)?;
            let (state, targets) = service(&state)?.status(&CancellationToken::new()).await?;
            Ok(AgentUpdatesView::new(state, targets))
        }
        .await
        .map(Json),
        &headers,
    )
}
#[utoipa::path(post,path="/api/v1/updates/agents/review",operation_id="reviewAgentUpdates",tag="Updates",summary="Review selected Agent updates",request_body=ReviewAgentsInput,
responses((status=200,description="Success",body=AgentReviewView),crate::openapi::errors::AccessErrors),security(("Bearer"=[])),
extensions(("x-citadel-principal"=json!("administrator")),("x-citadel-public"=json!(false)),("x-citadel-setup-exempt"=json!(false))))]
async fn review(
    State(state): State<UpdateHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(input): Json<ReviewAgentsInput>,
) -> Response {
    response(
        async {
            let actor = actor(require_human_administrator(principal)?);
            if !state.identity.installation_policy().update_checks_allowed {
                return Err(ApiError::Forbidden);
            }
            service(&state)?
                .review(actor, &input.targets, &CancellationToken::new())
                .await
                .map(AgentReviewView::from)
                .map_err(ApiError::from)
        }
        .await
        .map(Json),
        &headers,
    )
}
#[utoipa::path(post,path="/api/v1/updates/agents/approve",operation_id="approveAgentUpdates",tag="Updates",summary="Start a reviewed Agent selection",request_body=ApproveUpdateInput,
responses((status=200,description="Success",body=AgentOperationView),crate::openapi::errors::AccessErrors),security(("Bearer"=[])),
extensions(("x-citadel-principal"=json!("administrator")),("x-citadel-public"=json!(false)),("x-citadel-setup-exempt"=json!(false))))]
async fn approve(
    State(state): State<UpdateHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(input): Json<ApproveUpdateInput>,
) -> Response {
    response(
        async {
            let actor = actor(require_human_administrator(principal)?);
            if !state.identity.installation_policy().update_checks_allowed {
                return Err(ApiError::Forbidden);
            }
            service(&state)?
                .approve(actor, input.review_id, input.request_id)
                .await
                .map(AgentOperationView::from)
                .map_err(ApiError::from)
        }
        .await
        .map(Json),
        &headers,
    )
}
#[utoipa::path(put,path="/api/v1/updates/agents/schedule",operation_id="updateAgentSchedule",tag="Updates",summary="Save the Team Agent update schedule",request_body=AgentScheduleView,
responses((status=200,description="Success",body=AgentScheduleView),crate::openapi::errors::AccessErrors),security(("Bearer"=[])),
extensions(("x-citadel-principal"=json!("administrator")),("x-citadel-public"=json!(false)),("x-citadel-setup-exempt"=json!(false))))]
async fn schedule(
    State(state): State<UpdateHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Json(input): Json<AgentScheduleView>,
) -> Response {
    response(
        async {
            let actor = actor(require_human_administrator(principal)?);
            let schedule: citadel_updates::AgentUpdateSchedule = input.into();
            service(&state)?
                .save_schedule(actor, schedule.clone(), &CancellationToken::new())
                .await?;
            Ok(AgentScheduleView::from(schedule))
        }
        .await
        .map(Json),
        &headers,
    )
}

#[utoipa::path(get,path="/api/v1/updates/agents/operations/{id}",operation_id="getAgentUpdateOperation",tag="Updates",summary="Read durable Agent update progress",params(("id"=uuid::Uuid,Path)),
responses((status=200,description="Success",body=AgentOperationView),crate::openapi::errors::AccessErrors),security(("Bearer"=[])),
extensions(("x-citadel-principal"=json!("administrator")),("x-citadel-public"=json!(false)),("x-citadel-setup-exempt"=json!(false))))]
async fn operation(
    State(state): State<UpdateHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    Path(id): Path<uuid::Uuid>,
) -> Response {
    response(
        async {
            require_human_administrator(principal)?;
            service(&state)?
                .operation(id)
                .await
                .map(AgentOperationView::from)
                .map_err(ApiError::from)
        }
        .await
        .map(Json),
        &headers,
    )
}
