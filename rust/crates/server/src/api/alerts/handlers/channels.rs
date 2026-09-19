use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/alertRules/channels",
    operation_id = "listAlertChannels",
    tag = "AlertRules",
    summary = "List Alert Channels",
    responses(
        (status = 200, description = "Success", body = Channels, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn list_channels(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let channels = result(
        s.store
            .list_channels(p.actor_id, p.is_administrator())
            .await,
        &h,
    )?;
    Ok(no_store(
        Json(Channels {
            channels: channels.into_iter().map(Into::into).collect(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/alertRules/channels",
    operation_id = "createAlertChannel",
    tag = "AlertRules",
    summary = "Create an Alert Channel",
    request_body = AlertChannelInput,
    responses(
        (status = 200, description = "Success", body = AlertChannelView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn create_channel(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(i): ValidatedJson<AlertChannelInput>,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::AlertChannel,
        PermissionLevel::Write,
        None,
        &h,
    )
    .await?;
    let mut i: citadel_alerts::AlertChannelConfiguration = i.into();
    result(i.validate(), &h)?;
    let v = result(s.store.create_channel(p.actor_id, &i).await, &h)?;
    Ok(no_store(Json(AlertChannelView::from(v)).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/alertRules/channels/{id}",
    operation_id = "getAlertChannel",
    tag = "AlertRules",
    summary = "Get an Alert Channel",
    responses(
        (status = 200, description = "Success", body = AlertChannelView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_channel(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::AlertChannel,
        PermissionLevel::Read,
        Some(id),
        &h,
    )
    .await?;
    Ok(no_store(
        Json(AlertChannelView::from(result(
            s.store.get_channel(id).await,
            &h,
        )?))
        .into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/alertRules/channels/{id}",
    operation_id = "updateAlertChannel",
    tag = "AlertRules",
    summary = "Update an Alert Channel",
    request_body(content(
        (AlertChannelInput = "application/merge-patch+json"),
        (AlertChannelInput = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = AlertChannelView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn update_channel(
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
        ResourceType::AlertChannel,
        PermissionLevel::Write,
        Some(id),
        &h,
    )
    .await?;
    Ok(no_store(
        Json(AlertChannelView::from(result(
            s.store.update_channel(id, &patch).await,
            &h,
        )?))
        .into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/alertRules/channels",
    operation_id = "deleteAlertChannels",
    tag = "AlertRules",
    summary = "Delete Alert Channels",
    request_body = Ids,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn delete_channels(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(i): ValidatedJson<Ids>,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::AlertChannel,
        PermissionLevel::Write,
        None,
        &h,
    )
    .await?;
    result(s.store.delete_channels(&i.ids).await, &h)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    post,
    path = "/api/v1/alertRules/channels/verify",
    operation_id = "verifyAlertChannel",
    tag = "AlertRules",
    summary = "Verify an Alert Channel",
    request_body = VerifyInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ExternalAccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn verify_channel(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(i): ValidatedJson<VerifyInput>,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    auth(
        &s,
        &p,
        ResourceType::AlertChannel,
        PermissionLevel::Execute,
        None,
        &h,
    )
    .await?;
    let mut i = citadel_alerts::AlertChannelConfiguration {
        name: i.name,
        alert_destination: i.alert_destination,
        url: i.url,
        is_active: true,
    };
    result(i.validate(), &h)?;
    let channel = citadel_alerts::AlertChannel {
        id: Uuid::nil(),
        name: i.name,
        alert_destination: i.alert_destination,
        url: i.url,
        is_active: true,
        created_by_actor_id: p.actor_id.value(),
        created_at: chrono::Utc::now(),
    };
    let event = test_event();
    result(
        s.delivery
            .send(&channel, &event, &CancellationToken::new())
            .await,
        &h,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

pub(super) fn test_event() -> citadel_alerts::AlertEvent {
    let now = chrono::Utc::now();
    citadel_alerts::AlertEvent {
        id: Uuid::nil(),
        alert_rule_id: Uuid::nil(),
        alert_type: "Verification".into(),
        severity: "Information".into(),
        status: "Active".into(),
        message: "Citadel Alert Channel verification succeeded.".into(),
        info: json!({"humanMessage":"Citadel Alert Channel verification succeeded."}),
        resource_id: None,
        resource_name: "Citadel".into(),
        resource_type: "System".into(),
        acknowledged_by_actor_id: None,
        acknowledged_at: None,
        resolved_by_actor_id: None,
        resolved_at: None,
        resolution_note: None,
        created_at: now,
        updated_at: now,
    }
}
