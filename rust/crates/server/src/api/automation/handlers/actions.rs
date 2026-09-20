use super::*;

pub(crate) async fn authorized_actions(
    store: &dyn citadel_automation::AutomationRepository,
    principal: &ActorPrincipal,
    actions: Vec<citadel_automation::AutomationAction>,
) -> Result<Vec<AuthorizedAction>, AutomationError> {
    let ids: Vec<_> = actions.iter().map(|action| action.id).collect();
    let permissions = if principal.is_administrator() {
        Default::default()
    } else {
        store.permissions(principal.actor_id, &ids).await?
    };
    Ok(actions
        .into_iter()
        .map(|action| {
            let level = if principal.is_administrator() {
                citadel_primitives::EffectivePermission::Administrator
            } else {
                granted(
                    permissions
                        .get(&action.id)
                        .copied()
                        .unwrap_or(PermissionLevel::None),
                )
            };
            AuthorizedAction {
                action: action.into(),
                capabilities: capabilities(level),
            }
        })
        .collect())
}

pub(super) async fn action_response(
    state: &AutomationHttpState,
    principal: &ActorPrincipal,
    action: citadel_automation::AutomationAction,
    headers: &HeaderMap,
) -> IdentityHttpResult {
    let mut actions = identity_result(
        authorized_actions(state.automation.store().as_ref(), principal, vec![action])
            .await
            .map_err(map_error),
        headers,
    )?;
    Ok(no_store(Json(actions.remove(0)).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/automation/actions",
    operation_id = "listAutomationActions",
    tag = "AutomationActions",
    summary = "List Automation Actions",
    responses(
        (status = 200, description = "Success", body = ActionList, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("tags" = Option<Vec<String>>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn list(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    let tags = identity_result(
        crate::api::tags::handlers::parse_filters(query.as_deref()),
        &headers,
    )?;
    let mut actions = identity_result(
        state
            .automation
            .store()
            .list(principal.actor_id, principal.is_administrator())
            .await
            .map_err(map_error),
        &headers,
    )?;
    actions.retain(|action| crate::api::tags::handlers::matches_filters(&action.tags, &tags));
    let actions = identity_result(
        authorized_actions(state.automation.store().as_ref(), &principal, actions)
            .await
            .map_err(map_error),
        &headers,
    )?;
    let level = if principal.is_administrator() {
        citadel_primitives::EffectivePermission::Administrator
    } else {
        granted(
            identity_result(
                state
                    .identity
                    .global_permission(&principal, ResourceType::AutomationAction)
                    .await,
                &headers,
            )?
            .map_or(PermissionLevel::None, |grant| grant.level),
        )
    };
    Ok(no_store(
        Json(ActionList {
            actions,
            capabilities: capabilities(level),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/automation/actions",
    operation_id = "createAutomationAction",
    tag = "AutomationActions",
    summary = "Create an Automation Action",
    request_body = AutomationActionInput,
    responses(
        (status = 200, description = "Success", body = AutomationActionView, content_type = "application/json"),
        crate::openapi::errors::CreateErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn create(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(input): ValidatedJson<AutomationActionInput>,
) -> IdentityHttpResult {
    let mut input: citadel_automation::AutomationActionConfiguration = input.into();
    let principal = actor(principal, &headers)?;
    authorize_global(&state, &principal, PermissionLevel::Write, &headers).await?;
    identity_result(
        state
            .automation
            .validate_input(&mut input, principal.actor_id)
            .map_err(map_error),
        &headers,
    )?;
    identity_result(
        state
            .identity
            .ensure_run_as_allowed(
                &principal,
                citadel_primitives::ActorId::new(
                    input
                        .run_as_actor_id
                        .expect("validation sets the run-as Actor"),
                ),
            )
            .await,
        &headers,
    )?;
    if citadel_automation::changes_paid_trigger(None, &input) {
        identity_result(
            state
                .automation
                .ensure_paid_trigger()
                .await
                .map_err(map_error),
            &headers,
        )?;
    }
    let action = identity_result(
        state
            .automation
            .store()
            .create(principal.actor_id, &input)
            .await
            .map_err(map_error),
        &headers,
    )?;
    action_response(&state, &principal, action, &headers).await
}

#[utoipa::path(
    get,
    path = "/api/v1/automation/actions/{id}",
    operation_id = "getAutomationAction",
    tag = "AutomationActions",
    summary = "Get an Automation Action",
    responses(
        (status = 200, description = "Success", body = AutomationActionView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_one(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Read, &headers).await?;
    let action = identity_result(
        state.automation.store().get(id).await.map_err(map_error),
        &headers,
    )?;
    action_response(&state, &principal, action, &headers).await
}

#[utoipa::path(
    post,
    path = "/api/v1/automation/actions/rename",
    operation_id = "renameAutomationAction",
    tag = "AutomationActions",
    summary = "Rename an Automation Action",
    request_body = RenameInput,
    responses(
        (status = 200, description = "Success", body = AutomationActionView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn rename(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    ValidatedJson(input): ValidatedJson<RenameInput>,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(
        &state,
        &principal,
        input.id,
        PermissionLevel::Write,
        &headers,
    )
    .await?;
    let action = identity_result(
        state
            .automation
            .store()
            .rename(input.id, &input.name, principal.actor_id)
            .await
            .map_err(map_error),
        &headers,
    )?;
    action_response(&state, &principal, action, &headers).await
}

#[utoipa::path(
    patch,
    path = "/api/v1/automation/actions/{id}",
    operation_id = "updateAutomationAction",
    tag = "AutomationActions",
    summary = "Update an Automation Action",
    request_body(content(
        (UpdateAutomationActionInput = "application/merge-patch+json"),
        (UpdateAutomationActionInput = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = AutomationActionView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn update(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    patch: Result<Json<UpdateAutomationActionInput>, JsonRejection>,
) -> IdentityHttpResult {
    update_action(
        state,
        principal,
        id,
        headers,
        patch.map(|Json(value)| value),
        false,
    )
    .await
}

#[utoipa::path(
    patch,
    path = "/api/v1/automation/actions/{id}/_metadata",
    operation_id = "updateAutomationActionMetadata",
    tag = "AutomationActions",
    summary = "Update Automation Action metadata",
    request_body(content(
        (UpdateAutomationActionMetadata = "application/merge-patch+json"),
        (UpdateAutomationActionMetadata = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = AutomationActionView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn update_metadata(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    patch: Result<Json<UpdateAutomationActionMetadata>, JsonRejection>,
) -> IdentityHttpResult {
    update_action(
        state,
        principal,
        id,
        headers,
        patch.map(|Json(value)| value.into()),
        true,
    )
    .await
}

pub(super) async fn update_action(
    state: AutomationHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    id: Uuid,
    headers: HeaderMap,
    patch: Result<UpdateAutomationActionInput, JsonRejection>,
    metadata_only: bool,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    let patch = identity_result(
        patch.map_err(crate::request_validation::invalid_json),
        &headers,
    )?;
    let current = identity_result(
        state.automation.store().get(id).await.map_err(map_error),
        &headers,
    )?;
    let mut input = patch.apply(current.clone());
    identity_result(
        state
            .automation
            .validate_input(&mut input, principal.actor_id)
            .map_err(map_error),
        &headers,
    )?;
    identity_result(
        state
            .identity
            .ensure_run_as_allowed(
                &principal,
                citadel_primitives::ActorId::new(
                    input
                        .run_as_actor_id
                        .expect("validation sets the run-as Actor"),
                ),
            )
            .await,
        &headers,
    )?;
    if citadel_automation::changes_paid_trigger(Some(&current), &input) {
        identity_result(
            state
                .automation
                .ensure_paid_trigger()
                .await
                .map_err(map_error),
            &headers,
        )?;
    }
    let action = identity_result(
        state
            .automation
            .store()
            .update(&current, &input, principal.actor_id, metadata_only)
            .await
            .map_err(map_error),
        &headers,
    )?;
    action_response(&state, &principal, action, &headers).await
}

#[utoipa::path(
    delete,
    path = "/api/v1/automation/actions/{id}",
    operation_id = "deleteAutomationAction",
    tag = "AutomationActions",
    summary = "Delete an Automation Action",
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn remove(
    State(state): State<AutomationHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = actor(principal, &headers)?;
    authorize(&state, &principal, id, PermissionLevel::Write, &headers).await?;
    identity_result(
        state
            .automation
            .store()
            .delete(id, principal.actor_id)
            .await
            .map_err(map_error),
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
