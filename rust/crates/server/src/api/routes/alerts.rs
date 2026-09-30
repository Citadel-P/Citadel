//! Alerts HTTP routes, authorization and local request handling.
use crate::{
    api::{
        error::{ApiError, HttpResult, api_result, no_store},
        resources::alerts::{patch::*, requests::*, views::*},
    },
    openapi::router::OpenApiRouterExt,
    request_validation::{ApiPath, ApiQuery, ValidatedJson, invalid_json},
};

use axum::{
    Json, Router,
    extract::{Extension, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

use citadel_alerts::{AlertDelivery, AlertError, AlertRepository};

use citadel_identity::{ActorPrincipal, IdentityService};

use citadel_primitives::{PermissionLevel, ResourceType};

use serde_json::json;

use std::sync::Arc;

use tokio_util::sync::CancellationToken;

use uuid::Uuid;

#[derive(Clone)]
pub struct AlertsHttpState {
    pub identity: Arc<IdentityService>,
    pub store: Arc<dyn AlertRepository>,
    pub delivery: Arc<dyn AlertDelivery>,
}

pub fn router(state: AlertsHttpState) -> Router {
    crate::realtime::notify_mutations(
        documented_routes().split_for_parts().0.with_state(state),
        "Alert",
    )
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<AlertsHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(list_channels))
        .normalized_routes(utoipa_axum::routes!(create_channel))
        .normalized_routes(utoipa_axum::routes!(get_channel))
        .normalized_routes(utoipa_axum::routes!(update_channel))
        .normalized_routes(utoipa_axum::routes!(delete_channels))
        .normalized_routes(utoipa_axum::routes!(verify_channel))
        .normalized_routes(utoipa_axum::routes!(list_rules))
        .normalized_routes(utoipa_axum::routes!(create_rule))
        .normalized_routes(utoipa_axum::routes!(get_rule))
        .normalized_routes(utoipa_axum::routes!(get_rule_config))
        .normalized_routes(utoipa_axum::routes!(rename_rule))
        .normalized_routes(utoipa_axum::routes!(update_rule_metadata))
        .normalized_routes(utoipa_axum::routes!(update_rule))
        .normalized_routes(utoipa_axum::routes!(delete_rules))
        .normalized_routes(utoipa_axum::routes!(list_events))
        .normalized_routes(utoipa_axum::routes!(get_event))
        .normalized_routes(utoipa_axum::routes!(unresolved_count))
        .normalized_routes(utoipa_axum::routes!(acknowledge))
        .normalized_routes(utoipa_axum::routes!(resolve))
}

fn actor(v: Option<Extension<ActorPrincipal>>, h: &HeaderMap) -> HttpResult<ActorPrincipal> {
    api_result(v.map(|Extension(v)| v).ok_or(ApiError::Unauthenticated), h)
}

async fn auth(
    s: &AlertsHttpState,
    p: &ActorPrincipal,
    t: ResourceType,
    l: PermissionLevel,
    id: Option<Uuid>,
    h: &HeaderMap,
) -> HttpResult<()> {
    let r = match (t, l, id) {
        (ResourceType::AlertChannel, PermissionLevel::Read, Some(id)) => {
            s.identity
                .require_resource::<citadel_alerts::permissions::ReadAlertChannel>(p, id)
                .await
        }
        (ResourceType::AlertChannel, PermissionLevel::Read, None) => {
            s.identity
                .require_scope::<citadel_alerts::permissions::ReadAlertChannel>(p)
                .await
        }
        (ResourceType::AlertChannel, PermissionLevel::Write, Some(id)) => {
            s.identity
                .require_resource::<citadel_alerts::permissions::WriteAlertChannel>(p, id)
                .await
        }
        (ResourceType::AlertChannel, PermissionLevel::Write, None) => {
            s.identity
                .require_scope::<citadel_alerts::permissions::WriteAlertChannel>(p)
                .await
        }
        (ResourceType::AlertChannel, PermissionLevel::Execute, Some(id)) => {
            s.identity
                .require_resource::<citadel_alerts::permissions::ExecuteAlertChannel>(p, id)
                .await
        }
        (ResourceType::AlertChannel, PermissionLevel::Execute, None) => {
            s.identity
                .require_scope::<citadel_alerts::permissions::ExecuteAlertChannel>(p)
                .await
        }
        (ResourceType::Alert, PermissionLevel::Read, Some(id)) => {
            s.identity
                .require_resource::<citadel_alerts::permissions::ReadAlertRule>(p, id)
                .await
        }
        (ResourceType::Alert, PermissionLevel::Read, None) => {
            s.identity
                .require_scope::<citadel_alerts::permissions::ReadAlertRule>(p)
                .await
        }
        (ResourceType::Alert, PermissionLevel::Write, Some(id)) => {
            s.identity
                .require_resource::<citadel_alerts::permissions::WriteAlertRule>(p, id)
                .await
        }
        (ResourceType::Alert, PermissionLevel::Write, None) => {
            s.identity
                .require_scope::<citadel_alerts::permissions::WriteAlertRule>(p)
                .await
        }
        (ResourceType::Alert, PermissionLevel::Execute, Some(id)) => {
            s.identity
                .require_resource::<citadel_alerts::permissions::ExecuteAlertRule>(p, id)
                .await
        }
        (ResourceType::Alert, PermissionLevel::Execute, None) => {
            s.identity
                .require_scope::<citadel_alerts::permissions::ExecuteAlertRule>(p)
                .await
        }
        _ => Err(citadel_identity::IdentityError::Forbidden),
    };
    api_result(r, h)?;
    Ok(())
}

fn result<T>(r: Result<T, AlertError>, h: &HeaderMap) -> HttpResult<T> {
    api_result(
        r.map_err(|e| match e {
            AlertError::FieldValidation(fields) => ApiError::FieldValidation(fields),
            AlertError::InvalidCooldown => ApiError::FieldValidation(
                [("0".into(), vec![AlertError::InvalidCooldown.to_string()])].into(),
            ),
            AlertError::RuleNotFound => {
                ApiError::ResourceNotFound("The provided alert rule does not exist")
            }
            AlertError::LicenseRequired => ApiError::LicenseRequired("advanced-alerting"),
            AlertError::Validation(m) => crate::request_validation::validation_error(m),
            AlertError::NotFound => ApiError::NotFound,
            AlertError::Conflict(m) => ApiError::Conflict(m),
            source @ (AlertError::Storage(_) | AlertError::Delivery(_)) => {
                ApiError::internal(source)
            }
        }),
        h,
    )
}

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
async fn list_channels(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    let records = result(
        s.store
            .list_channels(p.actor_id, p.is_administrator())
            .await,
        &h,
    )?;
    let ids = records.iter().map(|item| item.id).collect::<Vec<_>>();
    let permissions = api_result(
        s.identity
            .permissions_for_resources(&p, ResourceType::AlertChannel, &ids)
            .await,
        &h,
    )?;
    let items = records
        .into_iter()
        .map(|item| {
            let id = item.id;
            let mut view = result(AlertChannelView::try_from(item), &h)?;
            view.capabilities = Some(crate::api::resource_access::capabilities_from_permission(
                permissions.get(&id).copied().flatten(),
            ));
            Ok(view)
        })
        .collect::<HttpResult<Vec<_>>>()?;
    let permission = api_result(
        s.identity
            .global_permission(&p, ResourceType::AlertChannel)
            .await,
        &h,
    )?;
    Ok(no_store(
        Json(Channels {
            channels: items,
            capabilities: crate::api::resource_access::capabilities_from_permission(permission),
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
async fn create_channel(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(i): ValidatedJson<AlertChannelInput>,
) -> HttpResult {
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
    Ok(no_store(
        Json(result(AlertChannelView::try_from(v), &h)?).into_response(),
    ))
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
async fn get_channel(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> HttpResult {
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
    let mut view = result(
        AlertChannelView::try_from(result(s.store.get_channel(id).await, &h)?),
        &h,
    )?;
    let permission = api_result(
        s.identity
            .permission_for_resource(&p, ResourceType::AlertChannel, id)
            .await,
        &h,
    )?;
    view.capabilities = Some(crate::api::resource_access::capabilities_from_permission(
        permission,
    ));
    Ok(no_store(Json(view).into_response()))
}

#[utoipa::path(
    patch,
    path = "/api/v1/alertRules/channels/{id}",
    operation_id = "updateAlertChannel",
    tag = "AlertRules",
    summary = "Update an Alert Channel",
    request_body(content(
        (PatchAlertChannelInput = "application/merge-patch+json"),
        (PatchAlertChannelInput = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = AlertChannelView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_channel(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
    patch: Result<Json<PatchAlertChannelInput>, JsonRejection>,
) -> HttpResult {
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
    let Json(patch) = api_result(patch.map_err(invalid_json), &h)?;
    let patch = patch.into_value();
    Ok(no_store(
        Json(result(
            AlertChannelView::try_from(result(s.store.update_channel(id, &patch).await, &h)?),
            &h,
        )?)
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
async fn delete_channels(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(i): ValidatedJson<Ids>,
) -> HttpResult {
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
async fn verify_channel(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(i): ValidatedJson<VerifyInput>,
) -> HttpResult {
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
        alert_destination: i.alert_destination.as_str().to_owned(),
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
        audit: citadel_primitives::AuditMetadata {
            created_by_actor_id: p.actor_id,
            created_at: chrono::Utc::now(),
        },
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

fn test_event() -> citadel_alerts::AlertEvent {
    let now = chrono::Utc::now();
    citadel_alerts::AlertEvent {
        id: Uuid::nil(),
        alert_rule_id: Uuid::nil(),
        alert_type: "Verification".into(),
        severity: citadel_alerts::AlertSeverity::Info,
        status: citadel_alerts::AlertEventStatus::Active,
        message: "Citadel Alert Channel verification succeeded.".into(),
        info: json!({"humanMessage":"Citadel Alert Channel verification succeeded."}),
        resource_id: None,
        resource_name: "Citadel".into(),
        resource_type: "System".into(),
        acknowledged_by_actor_id: None,
        acknowledged_at: None,
        resolved_by_actor_id: None,
        resolved_at: None,
        actor_id: None,
        actor_name: None,
        actor_type: None,
        resolution_note: None,
        created_at: now,
        updated_at: now,
    }
}

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
async fn list_events(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiQuery(f): ApiQuery<EventFilter>,
    h: HeaderMap,
) -> HttpResult {
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
            paged_result: result(paged_result.try_into(), &h)?,
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
async fn get_event(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> HttpResult {
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
        Json(result(
            AlertEventView::try_from(result(s.store.get_event(id).await, &h)?),
            &h,
        )?)
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
async fn unresolved_count(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
) -> HttpResult {
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
async fn acknowledge(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(i): ValidatedJson<Ids>,
) -> HttpResult {
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
async fn resolve(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(i): ValidatedJson<ResolveInput>,
) -> HttpResult {
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
async fn list_rules(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
) -> HttpResult {
    let p = actor(p, &h)?;
    let records = result(
        s.store.list_rules(p.actor_id, p.is_administrator()).await,
        &h,
    )?;
    let ids = records.iter().map(|item| item.rule.id).collect::<Vec<_>>();
    let permissions = api_result(
        s.identity
            .permissions_for_resources(&p, ResourceType::Alert, &ids)
            .await,
        &h,
    )?;
    let items = records
        .into_iter()
        .map(|item| {
            let id = item.rule.id;
            let mut view = result(AlertRuleListItem::try_from(item), &h)?;
            view.rule.capabilities =
                Some(crate::api::resource_access::capabilities_from_permission(
                    permissions.get(&id).copied().flatten(),
                ));
            Ok(view)
        })
        .collect::<HttpResult<Vec<_>>>()?;
    let permission = api_result(
        s.identity.global_permission(&p, ResourceType::Alert).await,
        &h,
    )?;
    Ok(no_store(
        Json(Rules {
            alert_rules: items,
            capabilities: crate::api::resource_access::capabilities_from_permission(permission),
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
async fn create_rule(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(i): ValidatedJson<AlertRuleInput>,
) -> HttpResult {
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
        Json(result(
            AlertRuleView::try_from(result(s.store.create_rule(p.actor_id, &i).await, &h)?),
            &h,
        )?)
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
async fn get_rule(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> HttpResult {
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
    let mut view = result(
        AlertRuleView::try_from(result(s.store.get_rule(id).await, &h)?),
        &h,
    )?;
    let permission = api_result(
        s.identity
            .permission_for_resource(&p, ResourceType::Alert, id)
            .await,
        &h,
    )?;
    view.capabilities = Some(crate::api::resource_access::capabilities_from_permission(
        permission,
    ));
    Ok(no_store(Json(view).into_response()))
}

#[utoipa::path(
    get,
    path = "/api/v1/alertRules/{id}/_cfg",
    operation_id = "getAlertRuleConfig",
    tag = "AlertRules",
    summary = "Get Alert Rule configuration",
    responses(
        (status = 200, description = "Success", body = AlertRuleConfig, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_rule_config(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
) -> HttpResult {
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
        Json(result(AlertRuleConfig::try_from(rule), &h)?).into_response(),
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
async fn rename_rule(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(input): ValidatedJson<RenameAlertRuleInput>,
) -> HttpResult {
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
        Json(result(
            AlertRuleView::try_from(result(s.store.rename_rule(p.actor_id, &input).await, &h)?),
            &h,
        )?)
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
        (PatchAlertRuleMetadata = "application/merge-patch+json"),
        (PatchAlertRuleMetadata = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = AlertRuleView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_rule_metadata(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
    patch: Result<Json<PatchAlertRuleMetadata>, JsonRejection>,
) -> HttpResult {
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
    let Json(patch) = api_result(patch.map_err(invalid_json), &h)?;
    let patch = patch.into_value();
    let description = result(citadel_alerts::description_patch(&patch), &h)?;
    Ok(no_store(
        Json(result(
            AlertRuleView::try_from(result(
                s.store.update_rule_description(id, description).await,
                &h,
            )?),
            &h,
        )?)
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
        (PatchAlertRuleInput = "application/merge-patch+json"),
        (PatchAlertRuleInput = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = AlertRuleView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn update_rule(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    h: HeaderMap,
    patch: Result<Json<PatchAlertRuleInput>, JsonRejection>,
) -> HttpResult {
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
    let Json(patch) = api_result(patch.map_err(invalid_json), &h)?;
    let patch = patch.into_value();
    Ok(no_store(
        Json(result(
            AlertRuleView::try_from(result(
                s.store.update_rule(p.actor_id, id, &patch).await,
                &h,
            )?),
            &h,
        )?)
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
async fn delete_rules(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(i): ValidatedJson<Ids>,
) -> HttpResult {
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
