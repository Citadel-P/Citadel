use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/alertRules",
    operation_id = "listAlertRules",
    tag = "AlertRules",
    summary = "List Alert Rules",
    responses(
        (status = 200, description = "Success", body = Rules, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn list_rules(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    Ok(no_store(
        Json(Rules {
            alert_rules: result(
                s.store.list_rules(p.actor_id, p.is_administrator()).await,
                &h,
            )?
            .into_iter()
            .map(Into::into)
            .collect(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/alertRules",
    operation_id = "createAlertRule",
    tag = "AlertRules",
    summary = "Create an Alert Rule",
    request_body = AlertRuleInput,
    responses(
        (status = 200, description = "Success", body = AlertRuleView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn create_rule(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(i): ValidatedJson<AlertRuleInput>,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::Alert,
        PermissionLevel::Write,
        None,
        &h,
    )
    .await?;
    let mut i: citadel_alerts::AlertRuleConfiguration = i.into();
    result(i.validate_create(), &h)?;
    Ok(no_store(
        Json(AlertRuleView::from(result(
            s.store.create_rule(p.actor_id, &i).await,
            &h,
        )?))
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/alertRules/{id}",
    operation_id = "getAlertRule",
    tag = "AlertRules",
    summary = "Get an Alert Rule",
    responses(
        (status = 200, description = "Success", body = AlertRuleView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_rule(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::Alert,
        PermissionLevel::Read,
        Some(id),
        &h,
    )
    .await?;
    Ok(no_store(
        Json(AlertRuleView::from(result(s.store.get_rule(id).await, &h)?)).into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/alertRules/{id}/_cfg",
    operation_id = "getAlertRuleConfig",
    tag = "AlertRules",
    summary = "Get Alert Rule configuration",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/AlertRuleConfigView"), content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_rule_config(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::Alert,
        PermissionLevel::Read,
        Some(id),
        &h,
    )
    .await?;
    let rule = result(s.store.get_rule(id).await, &h)?;
    Ok(no_store(
        Json(json!({
            "id":rule.id,"name":rule.name,"description":rule.description,
            "isSystem":rule.created_by_actor_id == Uuid::from_u128(1),
            "type":rule.alert_type,"severity":rule.severity,"cooldownSeconds":rule.cooldown_seconds,
            "requiredMatches":rule.required_matches,"threshold":rule.threshold,"status":rule.status,
            "channelIds":rule.channel_ids,"limitedTo":rule.limited_to,"quietHours":rule.quiet_hours
        }))
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/alertRules/rename",
    operation_id = "renameAlertRule",
    tag = "AlertRules",
    summary = "Rename an Alert Rule",
    request_body = RenameAlertRuleInput,
    responses(
        (status = 200, description = "Success", body = AlertRuleView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn rename_rule(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(input): ValidatedJson<RenameAlertRuleInput>,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let mut input: citadel_alerts::RenameAlertRuleInput = input.into();
    result(input.validate(), &h)?;
    auth(
        &s,
        &p,
        ResourceType::Alert,
        PermissionLevel::Write,
        Some(input.id),
        &h,
    )
    .await?;
    Ok(no_store(
        Json(AlertRuleView::from(result(
            s.store.rename_rule(p.actor_id, &input).await,
            &h,
        )?))
        .into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/alertRules/{id}/_metadata",
    operation_id = "updateAlertRuleMetadata",
    tag = "AlertRules",
    summary = "Update Alert Rule metadata",
    request_body(content(
        (ref("#/components/schemas/PatchResourceMetadata") = "application/merge-patch+json"),
        (ref("#/components/schemas/PatchResourceMetadata") = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = AlertRuleView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn update_rule_metadata(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
    ValidatedJson(patch): ValidatedJson<serde_json::Value>,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::Alert,
        PermissionLevel::Write,
        Some(id),
        &h,
    )
    .await?;
    let description = result(citadel_alerts::description_patch(&patch), &h)?;
    Ok(no_store(
        Json(AlertRuleView::from(result(
            s.store.update_rule_description(id, description).await,
            &h,
        )?))
        .into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/alertRules/{id}",
    operation_id = "updateAlertRule",
    tag = "AlertRules",
    summary = "Update an Alert Rule",
    request_body(content(
        (ref("#/components/schemas/PatchAlertRuleInput") = "application/merge-patch+json"),
        (ref("#/components/schemas/PatchAlertRuleInput") = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = AlertRuleView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn update_rule(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
    ValidatedJson(patch): ValidatedJson<serde_json::Value>,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::Alert,
        PermissionLevel::Write,
        Some(id),
        &h,
    )
    .await?;
    Ok(no_store(
        Json(AlertRuleView::from(result(
            s.store.update_rule(p.actor_id, id, &patch).await,
            &h,
        )?))
        .into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/alertRules",
    operation_id = "deleteAlertRules",
    tag = "AlertRules",
    summary = "Delete Alert Rules",
    request_body = Ids,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn delete_rules(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(i): ValidatedJson<Ids>,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::Alert,
        PermissionLevel::Write,
        None,
        &h,
    )
    .await?;
    result(s.store.delete_rules(&i.ids).await, &h)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
