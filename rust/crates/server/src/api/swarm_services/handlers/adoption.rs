use super::*;

pub(super) async fn authorize_adoption(
    state: &SwarmServicesHttpState,
    principal: &ActorPrincipal,
    platform: Uuid,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    if !principal.is_administrator() {
        identity_result(
            state
                .identity
                .require_scope::<policy::CreateSwarmService>(principal)
                .await,
            headers,
        )?;
        identity_result(
            state
                .identity
                .authorize_resource(
                    principal,
                    ResourceType::Platform,
                    platform,
                    PermissionLevel::Read,
                    Some(SpecificPermission::Inspect),
                )
                .await,
            headers,
        )?;
    }
    Ok(())
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/swarm/services/{resourceId}/adoption-draft",
    operation_id = "getSwarmServiceAdoptionDraft",
    tag = "Platforms",
    summary = "Review an unmanaged Docker Service for adoption",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/SwarmServiceAdoptionDraftView"), content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn adoption_draft(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((platform, id)) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize_adoption(&state, &principal, platform, &headers).await?;
    let cancel = tokio_util::sync::CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let draft = identity_result(
        state
            .services
            .adoption_draft(
                principal.actor_id,
                principal.is_administrator(),
                platform,
                &id,
                &cancel,
            )
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(Json(serde_json::json!({
        "draft": {"name":draft.name,"platformId":draft.source.platform_id,"description":draft.description,
            "spec":SwarmServiceSpec::from(draft.spec),"tagIds":null,"duplicateSource":null},
        "source":SwarmServiceAdoptionSource::from(draft.source),"issues":draft.issues.into_iter().map(SwarmServiceAdoptionIssue::from).collect::<Vec<_>>(),"previewFingerprint":draft.preview_fingerprint
    })).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/platforms/{platformId}/swarm/services/{resourceId}/adopt",
    operation_id = "adoptSwarmService",
    tag = "Platforms",
    summary = "Adopt an existing Docker Service without changing Docker",
    request_body = AdoptSwarmServiceInput,
    responses(
        (status = 200, description = "Success", body = ManagedSwarmServiceView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("resourceId" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn adopt(
    State(state): State<SwarmServicesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
    body: Result<Json<AdoptSwarmServiceInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((platform, id)) = identity_result(path.map_err(invalid_path), &headers)?;
    authorize_adoption(&state, &principal, platform, &headers).await?;
    let Json(input) = identity_result(body.map_err(invalid_json), &headers)?;
    let cancel = tokio_util::sync::CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let service = identity_result(
        state
            .services
            .adopt(
                principal.actor_id,
                principal.is_administrator(),
                platform,
                &id,
                input.into(),
                &cancel,
            )
            .await
            .map_err(service_error),
        &headers,
    )?;
    Ok(no_store(
        Json(ManagedSwarmServiceView::from(service)).into_response(),
    ))
}
