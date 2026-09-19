use super::*;

#[utoipa::path(
    post,
    path = "/api/v1/stacks",
    operation_id = "createStack",
    tag = "Stacks",
    summary = "Create a Stack",
    request_body = CreateStackInput,
    responses(
        (status = 200, description = "Success", body = StackView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn create(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<CreateStackInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    identity_result(
        state
            .identity
            .require_scope::<policy::CreateStack>(&principal)
            .await,
        &headers,
    )?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    authorize_git_source(&state, &principal, &input.spec, &headers).await?;
    let input = identity_result(
        citadel_stacks::CreateStack::try_from(input).map_err(stack_error),
        &headers,
    )?;
    let value = identity_result(
        state
            .stacks
            .create(principal.actor_id, principal.is_administrator(), input)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(StackView::from(value)).into_response()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/stacks/{id}",
    operation_id = "updateStack",
    tag = "Stacks",
    summary = "Update Stack configuration",
    request_body(content(
        (PatchStackInput = "application/merge-patch+json"),
        (PatchStackInput = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = StackView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn update(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<PatchStackInput>, JsonRejection>,
) -> IdentityHttpResult {
    let (principal, id) =
        actor_and_id::<policy::WriteStack>(&state, principal, path, &headers).await?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    authorize_git_patch_source(&state, &principal, input.spec.as_ref(), &headers).await?;
    let value = identity_result(
        state
            .stacks
            .update(
                principal.actor_id,
                principal.is_administrator(),
                id,
                input.into(),
            )
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(StackView::from(value)).into_response()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/stacks/{id}/_metadata",
    operation_id = "updateStackMetadata",
    tag = "Stacks",
    summary = "Update Stack metadata",
    request_body(content(
        (ref("#/components/schemas/PatchResourceMetadata") = "application/merge-patch+json"),
        (ref("#/components/schemas/PatchResourceMetadata") = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = StackView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn update_metadata(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<Uuid>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<Value>, JsonRejection>,
) -> IdentityHttpResult {
    let (principal, id) =
        actor_and_id::<policy::WriteStack>(&state, principal, path, &headers).await?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    let description = match input.as_object().and_then(|value| value.get("description")) {
        Some(Value::String(value)) => Some(value.clone()),
        Some(Value::Null) => None,
        Some(_) => {
            return Err(crate::identity_http::IdentityHttpError::from_parts(
                IdentityError::Validation("Description must be a string or null.".to_owned()),
                &headers,
            ));
        }
        None => {
            return super::read::get(
                State(state),
                Some(Extension(principal)),
                Ok(Path(id)),
                headers,
            )
            .await;
        }
    };
    let value = identity_result(
        state
            .stacks
            .update_metadata(
                principal.actor_id,
                principal.is_administrator(),
                id,
                description,
            )
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(StackView::from(value)).into_response()))
}

#[utoipa::path(
    post,
    path = "/api/v1/stacks/rename",
    operation_id = "renameStack",
    tag = "Stacks",
    summary = "Rename a Stack",
    request_body = RenameStackInput,
    responses(
        (status = 200, description = "Success", body = StackView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn rename(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<RenameStackInput>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    authorize::<policy::WriteStack>(&state, &principal, input.id, &headers).await?;
    let value = identity_result(
        state
            .stacks
            .rename(
                principal.actor_id,
                principal.is_administrator(),
                input.id,
                input.name,
            )
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(StackView::from(value)).into_response()))
}

#[utoipa::path(
    delete,
    path = "/api/v1/stacks",
    operation_id = "deleteStacks",
    tag = "Stacks",
    summary = "Delete Stacks",
    request_body = Vec<Uuid>,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::UnavailableResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn delete(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<Vec<Uuid>>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Json(ids) = identity_result(input.map_err(invalid_json), &headers)?;
    for id in &ids {
        authorize::<policy::DeleteStack>(&state, &principal, *id, &headers).await?;
    }
    identity_result(
        state
            .stacks
            .delete(principal.actor_id, principal.is_administrator(), &ids)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
