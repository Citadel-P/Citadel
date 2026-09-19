use super::adoption_views::ContainerAdoptionDraft;
use super::handlers::*;
use super::requests::AdoptContainerInput;
use super::views::DeploymentView;
use tokio_util::sync::CancellationToken;

#[utoipa::path(
    get,
    path = "/api/v1/containers/{id}/adoption-draft",
    operation_id = "getContainerAdoptionDraft",
    tag = "Containers",
    summary = "Review adoption of an unmanaged Container",
    responses(
        (status = 200, description = "Success", body = ContainerAdoptionDraft, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn draft(
    State(state): State<DeploymentsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    identity_result(
        state
            .identity
            .require_scope::<CreateDeployment>(&principal)
            .await,
        &headers,
    )?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let draft = identity_result(
        tokio::time::timeout(
            std::time::Duration::from_secs(30),
            state.deployments.adoption_draft(
                principal.actor_id,
                principal.is_administrator(),
                id,
                &cancel,
            ),
        )
        .await
        .unwrap_or_else(|_| {
            Err(DeploymentError::Runtime(
                "Container adoption inspection timed out.".into(),
            ))
        })
        .map_err(deployment_error),
        &headers,
    )?;
    Ok(no_store(
        Json(ContainerAdoptionDraft::from(draft)).into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/containers/{id}/adopt",
    operation_id = "adoptContainer",
    tag = "Containers",
    summary = "Adopt a Container without changing Docker",
    request_body = AdoptContainerInput,
    responses(
        (status = 200, description = "Success", body = DeploymentView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn adopt(
    State(state): State<DeploymentsHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<AdoptContainerInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    identity_result(
        state
            .identity
            .require_scope::<CreateDeployment>(&principal)
            .await,
        &headers,
    )?;
    let Path(id) = identity_result(path.map_err(invalid_path), &headers)?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let deployment = identity_result(
        tokio::time::timeout(
            std::time::Duration::from_secs(30),
            state.deployments.adopt_container(
                principal.actor_id,
                principal.is_administrator(),
                id,
                input.into(),
                &cancel,
            ),
        )
        .await
        .unwrap_or_else(|_| {
            Err(DeploymentError::Runtime(
                "Container adoption timed out. Refresh inventory before retrying.".into(),
            ))
        })
        .map_err(deployment_error),
        &headers,
    )?;
    Ok(no_store(
        Json(DeploymentView::from(deployment)).into_response(),
    ))
}
