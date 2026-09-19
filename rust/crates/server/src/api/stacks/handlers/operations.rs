use super::*;

#[utoipa::path(
    post,
    path = "/api/v1/stacks/{stackId}/check-updates",
    operation_id = "checkStackUpdates",
    tag = "Stacks",
    summary = "Check a Stack source for updates",
    responses(
        (status = 200, description = "Success", body = StackView, content_type = "application/json"),
        crate::openapi::errors::ExternalResourceErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn check_updates(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) =
        actor_and_id::<policy::WriteStack>(&state, principal, path, &headers).await?;
    let cancel = tokio_util::sync::CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let value = identity_result(
        state
            .stacks
            .check_updates(
                principal.actor_id,
                principal.is_administrator(),
                id,
                &cancel,
            )
            .await
            .map_err(|error| match error {
                StackError::Runtime(message) => IdentityError::External(message),
                other => stack_error(other),
            }),
        &headers,
    )?;
    Ok(no_store(Json(StackView::from(value)).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/stacks/apply",
    operation_id = "applyStack",
    tag = "Stacks",
    summary = "Apply a Stack and stream progress",
    request_body = ApplyStackInput,
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/StackStreamItems"), content_type = "application/json"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn apply(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<ApplyStackInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    authorize::<policy::ApplyStack>(&state, &principal, input.id, &headers).await?;
    let receiver = identity_result(
        state
            .stacks
            .apply(
                principal.actor_id,
                principal.is_administrator(),
                input.into(),
            )
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(progress_response(receiver))
}

#[utoipa::path(
    post,
    path = "/api/v1/stacks/rollback",
    operation_id = "rollbackStack",
    tag = "Stacks",
    summary = "Roll back a Stack and stream progress",
    request_body = RollbackStackInput,
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/StackStreamItems"), content_type = "application/json"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn rollback(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<RollbackStackInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    authorize::<policy::RollbackStack>(&state, &principal, input.stack_id, &headers).await?;
    let receiver = identity_result(
        state
            .stacks
            .rollback(
                principal.actor_id,
                principal.is_administrator(),
                input.into(),
            )
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(progress_response(receiver))
}

#[utoipa::path(
    get,
    path = "/api/v1/stacks/{stackId}/drift",
    operation_id = "getStackDrift",
    tag = "Stacks",
    summary = "Get Stack drift",
    responses(
        (status = 200, description = "Success", body = StackDriftReport, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn drift(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) =
        actor_and_id::<policy::ReadStack>(&state, principal, path, &headers).await?;
    let value = identity_result(
        state
            .stacks
            .drift(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(
        Json(StackDriftReport::from(value)).into_response(),
    ))
}

#[derive(Deserialize, Default, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct StackDriftPolicyInput {
    mode: Option<StackDriftMode>,
    alert_on_drift: Option<bool>,
    mark_degraded: Option<bool>,
    auto_start_stopped_containers: Option<bool>,
    auto_resume_paused_containers: Option<bool>,
    remove_extra_containers: Option<bool>,
}

impl From<StackDriftPolicyInput> for citadel_stacks::StackDriftPolicy {
    fn from(value: StackDriftPolicyInput) -> Self {
        let defaults = Self::default();
        Self {
            mode: value.mode.map(Into::into).unwrap_or(defaults.mode),
            alert_on_drift: value.alert_on_drift.unwrap_or(defaults.alert_on_drift),
            mark_degraded: value.mark_degraded.unwrap_or(defaults.mark_degraded),
            auto_start_stopped_containers: value
                .auto_start_stopped_containers
                .unwrap_or(defaults.auto_start_stopped_containers),
            auto_resume_paused_containers: value
                .auto_resume_paused_containers
                .unwrap_or(defaults.auto_resume_paused_containers),
            remove_extra_containers: value
                .remove_extra_containers
                .unwrap_or(defaults.remove_extra_containers),
        }
        .normalized()
    }
}

#[utoipa::path(
    put,
    path = "/api/v1/stacks/{stackId}/drift-policy",
    operation_id = "updateStackDriftPolicy",
    tag = "Stacks",
    summary = "Update Stack drift policy",
    request_body = StackDriftPolicyInput,
    responses(
        (status = 200, description = "Success", body = StackView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn update_drift_policy(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<StackDriftPolicyInput>, JsonRejection>,
) -> IdentityHttpResult {
    let (principal, id) =
        actor_and_id::<policy::WriteStack>(&state, principal, path, &headers).await?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let value = identity_result(
        state
            .stacks
            .update_drift_policy(
                principal.actor_id,
                principal.is_administrator(),
                id,
                citadel_stacks::StackDriftPolicy::from(input),
            )
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(StackView::from(value)).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/stacks/{stackId}/reconcile",
    operation_id = "reconcileStack",
    tag = "Stacks",
    summary = "Reconcile safe Stack drift",
    responses(
        (status = 200, description = "Success", body = StackReconciliationResult, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("stackId" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn reconcile(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let (principal, id) =
        actor_and_id::<policy::ReconcileStack>(&state, principal, path, &headers).await?;
    let value = identity_result(
        state
            .stacks
            .reconcile_drift(principal.actor_id, principal.is_administrator(), id)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(
        Json(StackReconciliationResult::from(value)).into_response(),
    ))
}
