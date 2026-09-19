use super::*;

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub(super) struct SwarmPreflightInput {
    compose_files: Vec<String>,
    #[serde(default)]
    build_image_bindings: Vec<StackBuildImageBinding>,
}

#[utoipa::path(
    post,
    path = "/api/v1/stacks/preflight/swarm",
    operation_id = "preflightSwarmStack",
    tag = "Stacks",
    summary = "Validate Docker Swarm Stack compatibility",
    request_body = SwarmPreflightInput,
    responses(
        (status = 200, description = "Success", body = SwarmStackCompatibilityReport, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn preflight(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    input: Result<Json<SwarmPreflightInput>, JsonRejection>,
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
    let value = identity_result(
        state
            .stacks
            .preflight_swarm(
                &input.compose_files,
                &input
                    .build_image_bindings
                    .into_iter()
                    .map(Into::into)
                    .collect::<Vec<_>>(),
            )
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(
        Json(SwarmStackCompatibilityReport::from(value)).into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/platforms/{platformId}/unmanaged-compose-projects/{projectName}",
    operation_id = "getComposeProjectImportDraft",
    tag = "Platforms",
    summary = "Get a Compose or Swarm Stack import draft",
    responses(
        (status = 200, description = "Success", body = ComposeProjectImportDraftView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("projectName" = String, Path), ("importKind" = Option<StackImportKind>, Query)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn import_draft(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    query: Result<Query<ImportDraftQuery>, QueryRejection>,
    headers: HeaderMap,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((platform_id, project_name)) = identity_result(path.map_err(invalid_path), &headers)?;
    let Query(query) = identity_result(query.map_err(invalid_query), &headers)?;
    identity_result(
        state
            .identity
            .authorize_resource(
                &principal,
                ResourceType::Platform,
                platform_id,
                PermissionLevel::Read,
                None,
            )
            .await,
        &headers,
    )?;
    let value = identity_result(
        state
            .stacks
            .import_draft(
                platform_id,
                &project_name,
                query.import_kind.map(Into::into),
            )
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(
        Json(ComposeProjectImportDraftView::from(value)).into_response(),
    ))
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ImportDraftQuery {
    import_kind: Option<StackImportKind>,
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ValidateImportRequest {
    name: String,
    stack_source: StackSource,
    spec: StackSpec,
    import_kind: Option<StackImportKind>,
}

#[utoipa::path(
    post,
    path = "/api/v1/platforms/{platformId}/unmanaged-compose-projects/{projectName}/import-draft",
    operation_id = "validateComposeProjectImportDraft",
    tag = "Platforms",
    summary = "Validate a source for an unmanaged Compose project",
    request_body = ValidateImportRequest,
    responses(
        (status = 200, description = "Success", body = ComposeProjectImportValidation, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("projectName" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn validate_import_draft(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<ValidateImportRequest>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((platform_id, project_name)) = identity_result(path.map_err(invalid_path), &headers)?;
    identity_result(
        state
            .identity
            .require_scope::<policy::CreateStack>(&principal)
            .await,
        &headers,
    )?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    authorize_git_source(&state, &principal, &input.spec, &headers).await?;
    let value = identity_result(
        state
            .stacks
            .validate_import(
                platform_id,
                &project_name,
                &input.name,
                input.stack_source.into(),
                &input.spec.into(),
                input.import_kind.map(Into::into),
            )
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(
        Json(ComposeProjectImportValidation::from(value)).into_response(),
    ))
}

#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ImportRequest {
    name: String,
    description: Option<String>,
    stack_source: StackSource,
    spec: StackSpec,
    preview_fingerprint: String,
    #[serde(default)]
    tag_ids: Vec<Uuid>,
    import_kind: Option<StackImportKind>,
    #[serde(default)]
    import_sensitive_environment_as_secrets: bool,
}

#[utoipa::path(
    post,
    path = "/api/v1/platforms/{platformId}/unmanaged-compose-projects/{projectName}/import",
    operation_id = "importComposeProject",
    tag = "Platforms",
    summary = "Import a Compose project or Swarm Stack",
    request_body = ImportRequest,
    responses(
        (status = 200, description = "Success", body = StackView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("platformId" = uuid::Uuid, Path), ("projectName" = String, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn import(
    State(state): State<StacksHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    path: Result<Path<(Uuid, String)>, PathRejection>,
    headers: HeaderMap,
    input: Result<Json<ImportRequest>, JsonRejection>,
) -> IdentityHttpResult {
    let principal = identity_result(require_actor(principal), &headers)?;
    let Path((platform_id, project_name)) = identity_result(path.map_err(invalid_path), &headers)?;
    identity_result(
        state
            .identity
            .require_scope::<policy::CreateStack>(&principal)
            .await,
        &headers,
    )?;
    let Json(input) = identity_result(input.map_err(invalid_json), &headers)?;
    authorize_git_source(&state, &principal, &input.spec, &headers).await?;
    if citadel_stacks::StackSource::from(input.stack_source)
        != citadel_stacks::StackSpec::from(input.spec.clone()).source()
    {
        return Err(crate::identity_http::IdentityHttpError::from_parts(
            IdentityError::Validation("Stack source and specification type must match.".to_owned()),
            &headers,
        ));
    }
    if input.import_sensitive_environment_as_secrets {
        return Err(crate::identity_http::IdentityHttpError::from_parts(
            IdentityError::Validation(
                "Importing newly detected sensitive values requires the Phase 7 Secret execution slice."
                    .to_owned(),
            ),
            &headers,
        ));
    }
    let import_kind = match input.import_kind {
        Some(value) => value.into(),
        None => {
            identity_result(
                state
                    .stacks
                    .import_draft(platform_id, &project_name, None)
                    .await
                    .map_err(stack_error),
                &headers,
            )?
            .import_kind
        }
    };
    let input = ImportComposeProject {
        name: input.name,
        platform_id,
        project_name,
        description: input.description,
        spec: input.spec.into(),
        tag_ids: input.tag_ids,
        import_kind,
        preview_fingerprint: input.preview_fingerprint,
        detected_secret_values: std::collections::BTreeMap::new(),
    };
    let value = identity_result(
        state
            .stacks
            .import(principal.actor_id, principal.is_administrator(), input)
            .await
            .map_err(stack_error),
        &headers,
    )?;
    Ok(no_store(Json(StackView::from(value)).into_response()))
}
