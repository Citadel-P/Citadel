use crate::request_validation::ApiPath;
use crate::request_validation::ApiQuery;
use crate::request_validation::ValidatedJson;
use std::sync::Arc;

use axum::extract::{Extension, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_alerts::{AlertChannelInput, AlertDelivery, AlertError, AlertRuleInput, AlertStore};
use citadel_domain::{PermissionLevel, ResourceType};
use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::identity_http::{IdentityHttpResult, identity_result, no_store};
use crate::openapi::router::OpenApiRouterExt;

#[derive(Clone)]
pub struct AlertsHttpState {
    pub identity: Arc<IdentityService>,
    pub store: Arc<dyn AlertStore>,
    pub delivery: Arc<dyn AlertDelivery>,
}

pub fn router(state: AlertsHttpState) -> Router {
    crate::realtime::notify_mutations(
        documented_routes().split_for_parts().0.with_state(state),
        "Alert",
    )
}

#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct Channels {
    channels: Vec<citadel_alerts::AlertChannelView>,
}
#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct Rules {
    alert_rules: Vec<citadel_alerts::AlertRuleListItem>,
}
#[derive(Serialize, utoipa::ToSchema)]
#[schema(as = server::alerts_http::Events)]
#[serde(rename_all = "camelCase")]
struct Events {
    paged_result: citadel_alerts::AlertEventPage,
}
#[derive(Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct Count {
    count: i64,
}
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct Ids {
    ids: Vec<Uuid>,
}
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct ResolveInput {
    ids: Vec<Uuid>,
    resolution_note: Option<String>,
}
#[derive(Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
struct VerifyInput {
    name: String,
    alert_destination: String,
    url: String,
}
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct EventFilter {
    #[serde(alias = "ResourceId")]
    resource_id: Option<Uuid>,
    #[serde(alias = "AlertType")]
    alert_type: Option<String>,
    #[serde(alias = "ResourceType")]
    resource_type: Option<String>,
    #[serde(alias = "UnresolvedOnly")]
    unresolved_only: Option<bool>,
    #[serde(alias = "Page")]
    page: Option<i32>,
    #[serde(alias = "PageSize")]
    page_size: Option<i32>,
}

#[utoipa::path(
    get,
    path = "/api/v1/alertRules/channels",
    operation_id = "listAlertChannels",
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
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    let channels = result(
        s.store
            .list_channels(p.actor_id, p.is_administrator())
            .await,
        &h,
    )?;
    Ok(no_store(Json(Channels { channels }).into_response()))
}
#[utoipa::path(
    post,
    path = "/api/v1/alertRules/channels",
    operation_id = "createAlertChannel",
    summary = "Create an Alert Channel",
    request_body = AlertChannelInput,
    responses(
        (status = 200, description = "Success", body = citadel_alerts::AlertChannelView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_channel(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(mut i): ValidatedJson<AlertChannelInput>,
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
    result(i.validate(), &h)?;
    let v = result(s.store.create_channel(p.actor_id, &i).await, &h)?;
    Ok(no_store(Json(v).into_response()))
}
#[utoipa::path(
    get,
    path = "/api/v1/alertRules/channels/{id}",
    operation_id = "getAlertChannel",
    summary = "Get an Alert Channel",
    responses(
        (status = 200, description = "Success", body = citadel_alerts::AlertChannelView, content_type = "application/json"),
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
        Json(result(s.store.get_channel(id).await, &h)?).into_response(),
    ))
}
#[utoipa::path(
    patch,
    path = "/api/v1/alertRules/channels/{id}",
    operation_id = "updateAlertChannel",
    summary = "Update an Alert Channel",
    request_body = citadel_alerts::AlertChannelInput,
    responses(
        (status = 200, description = "Success", body = citadel_alerts::AlertChannelView, content_type = "application/json"),
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
        Json(result(s.store.update_channel(id, &patch).await, &h)?).into_response(),
    ))
}
#[utoipa::path(
    delete,
    path = "/api/v1/alertRules/channels",
    operation_id = "deleteAlertChannels",
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
    let mut i = AlertChannelInput {
        name: i.name,
        alert_destination: i.alert_destination,
        url: i.url,
        is_active: true,
    };
    result(i.validate(), &h)?;
    let channel = citadel_alerts::AlertChannelView {
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
#[utoipa::path(
    get,
    path = "/api/v1/alertRules",
    operation_id = "listAlertRules",
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
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
    Ok(no_store(
        Json(Rules {
            alert_rules: result(
                s.store.list_rules(p.actor_id, p.is_administrator()).await,
                &h,
            )?,
        })
        .into_response(),
    ))
}
#[utoipa::path(
    post,
    path = "/api/v1/alertRules",
    operation_id = "createAlertRule",
    summary = "Create an Alert Rule",
    request_body = AlertRuleInput,
    responses(
        (status = 200, description = "Success", body = citadel_alerts::AlertRuleView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create_rule(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(mut i): ValidatedJson<AlertRuleInput>,
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
    result(i.validate_create(), &h)?;
    Ok(no_store(
        Json(result(s.store.create_rule(p.actor_id, &i).await, &h)?).into_response(),
    ))
}
#[utoipa::path(
    get,
    path = "/api/v1/alertRules/{id}",
    operation_id = "getAlertRule",
    summary = "Get an Alert Rule",
    responses(
        (status = 200, description = "Success", body = citadel_alerts::AlertRuleView, content_type = "application/json"),
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
        Json(result(s.store.get_rule(id).await, &h)?).into_response(),
    ))
}
#[utoipa::path(
    get,
    path = "/api/v1/alertRules/{id}/_cfg",
    operation_id = "getAlertRuleConfig",
    summary = "Get Alert Rule configuration",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/AlertRuleConfigView"), content_type = "application/json"),
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
    summary = "Rename an Alert Rule",
    request_body = citadel_alerts::RenameAlertRuleInput,
    responses(
        (status = 200, description = "Success", body = citadel_alerts::AlertRuleView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("actor")), ("x-citadel-public" = json!(true)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn rename_rule(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    ValidatedJson(mut input): ValidatedJson<citadel_alerts::RenameAlertRuleInput>,
) -> IdentityHttpResult {
    let p = actor(p, &h)?;
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
        Json(result(s.store.rename_rule(p.actor_id, &input).await, &h)?).into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/alertRules/{id}/_metadata",
    operation_id = "updateAlertRuleMetadata",
    summary = "Update Alert Rule metadata",
    request_body = ref("#/components/schemas/PatchResourceMetadata"),
    responses(
        (status = 200, description = "Success", body = citadel_alerts::AlertRuleView, content_type = "application/json"),
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
        Json(result(
            s.store.update_rule_description(id, description).await,
            &h,
        )?)
        .into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/alertRules/{id}",
    operation_id = "updateAlertRule",
    summary = "Update an Alert Rule",
    request_body = ref("#/components/schemas/PatchAlertRuleInput"),
    responses(
        (status = 200, description = "Success", body = citadel_alerts::AlertRuleView, content_type = "application/json"),
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
        Json(result(
            s.store.update_rule(p.actor_id, id, &patch).await,
            &h,
        )?)
        .into_response(),
    ))
}
#[utoipa::path(
    delete,
    path = "/api/v1/alertRules",
    operation_id = "deleteAlertRules",
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
#[utoipa::path(
    get,
    path = "/api/v1/alertEvents",
    operation_id = "listAlertEvents",
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
    Ok(no_store(Json(Events { paged_result }).into_response()))
}
#[utoipa::path(
    get,
    path = "/api/v1/alertEvents/{id}",
    operation_id = "getAlertEvent",
    summary = "Get an Alert Event",
    responses(
        (status = 200, description = "Success", body = citadel_alerts::AlertEventView, content_type = "application/json"),
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
        Json(result(s.store.get_event(id).await, &h)?).into_response(),
    ))
}
#[utoipa::path(
    get,
    path = "/api/v1/alertEvents/unresolved-count",
    operation_id = "getUnresolvedAlertEventsCount",
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

fn test_event() -> citadel_alerts::AlertEventView {
    let now = chrono::Utc::now();
    citadel_alerts::AlertEventView {
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
fn actor(
    v: Option<Extension<ActorPrincipal>>,
    h: &HeaderMap,
) -> IdentityHttpResult<ActorPrincipal> {
    identity_result(
        v.map(|Extension(v)| v)
            .ok_or(IdentityError::Unauthenticated),
        h,
    )
}
async fn auth(
    s: &AlertsHttpState,
    p: &ActorPrincipal,
    t: ResourceType,
    l: PermissionLevel,
    id: Option<Uuid>,
    h: &HeaderMap,
) -> IdentityHttpResult<()> {
    let r = match id {
        Some(id) => s.identity.authorize_resource(p, t, id, l, None).await,
        None => s.identity.authorize(p, t, l, None).await,
    };
    identity_result(r, h)?;
    Ok(())
}
fn result<T>(r: Result<T, AlertError>, h: &HeaderMap) -> IdentityHttpResult<T> {
    identity_result(
        r.map_err(|e| match e {
            AlertError::FieldValidation(fields) => IdentityError::FieldValidation(fields),
            AlertError::InvalidCooldown => IdentityError::FieldValidation(
                [("0".into(), vec![AlertError::InvalidCooldown.to_string()])].into(),
            ),
            AlertError::RuleNotFound => {
                IdentityError::ResourceNotFound("The provided alert rule does not exist")
            }
            AlertError::LicenseRequired => IdentityError::LicenseRequired("advanced-alerting"),
            AlertError::Validation(m) => crate::request_validation::validation_error(m),
            AlertError::NotFound => IdentityError::NotFound,
            AlertError::Conflict(m) => IdentityError::Conflict(m),
            AlertError::Storage(m) | AlertError::Delivery(m) => IdentityError::Storage(m),
        }),
        h,
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
