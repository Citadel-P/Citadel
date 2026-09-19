use super::*;

#[utoipa::path(
    get,
    path = "/api/v1/alertEvents",
    operation_id = "listAlertEvents",
    tag = "AlertEvents",
    summary = "List Alert Events",
    responses(
        (status = 200, description = "Success", body = Events, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    params(("ResourceId" = Option<uuid::Uuid>, Query), ("AlertType" = Option<String>, Query), ("ResourceType" = Option<String>, Query), ("UnresolvedOnly" = Option<bool>, Query), ("Page" = Option<i32>, Query, minimum = 1, extensions(("x-citadel-default" = json!(1)))), ("PageSize" = Option<i32>, Query, minimum = 1, maximum = 1000, extensions(("x-citadel-default" = json!(50))))),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn list_events(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiQuery(f): ApiQuery<EventFilter>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let filter = citadel_alerts::AlertEventFilter {
        resource_id: f.resource_id,
        alert_type: f.alert_type,
        resource_type: f.resource_type,
        unresolved_only: f.unresolved_only.unwrap_or(false),
        page: f.page.unwrap_or(1),
        page_size: f.page_size.unwrap_or(50),
    };
    let paged_result = result(
        s.store
            .list_events(p.actor_id, p.is_administrator(), &filter)
            .await,
        &h,
    )?;
    Ok(no_store(
        Json(Events {
            paged_result: paged_result.into(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/alertEvents/{id}",
    operation_id = "getAlertEvent",
    tag = "AlertEvents",
    summary = "Get an Alert Event",
    responses(
        (status = 200, description = "Success", body = AlertEventView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn get_event(
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
        Json(AlertEventView::from(result(
            s.store.get_event(id).await,
            &h,
        )?))
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/alertEvents/unresolved-count",
    operation_id = "getUnresolvedAlertEventsCount",
    tag = "AlertEvents",
    summary = "Count unresolved Alert Events",
    responses(
        (status = 200, description = "Success", body = Count, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn unresolved_count(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    Ok(no_store(
        Json(Count {
            count: result(
                s.store
                    .unresolved_count(p.actor_id, p.is_administrator())
                    .await,
                &h,
            )?,
        })
        .into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/alertEvents/acknowledge",
    operation_id = "acknowledgeAlertEvents",
    tag = "AlertEvents",
    summary = "Acknowledge Alert Events",
    request_body = Ids,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn acknowledge(
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
    result(s.store.acknowledge(p.actor_id, &i.ids).await, &h)?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[utoipa::path(
    post,
    path = "/api/v1/alertEvents/resolve",
    operation_id = "resolveAlertEvents",
    tag = "AlertEvents",
    summary = "Resolve Alert Events",
    request_body = ResolveInput,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
pub(super) async fn resolve(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(i): ValidatedJson<ResolveInput>,
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
    result(
        s.store
            .resolve(p.actor_id, &i.ids, i.resolution_note.as_deref())
            .await,
        &h,
    )?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
