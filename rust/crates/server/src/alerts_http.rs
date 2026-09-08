use std::sync::Arc;

use axum::extract::{Extension, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{Json, Router};
use citadel_alerts::{AlertChannelInput, AlertDelivery, AlertError, AlertRuleInput, AlertStore};
use citadel_contracts::http::routes;
use citadel_domain::{PermissionLevel, ResourceType};
use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::contract_router::ContractRouterExt;
use crate::identity_http::{IdentityHttpResult, identity_result, no_store};

#[derive(Clone)]
pub struct AlertsHttpState {
    pub identity: Arc<IdentityService>,
    pub store: Arc<dyn AlertStore>,
    pub delivery: Arc<dyn AlertDelivery>,
}

pub fn router(state: AlertsHttpState) -> Router {
    crate::realtime::notify_mutations(
        Router::new()
            .contract_route(routes::LIST_ALERT_CHANNELS, list_channels)
            .contract_route(routes::CREATE_ALERT_CHANNEL, create_channel)
            .contract_route(routes::GET_ALERT_CHANNEL, get_channel)
            .contract_route(routes::UPDATE_ALERT_CHANNEL, update_channel)
            .contract_route(routes::DELETE_ALERT_CHANNELS, delete_channels)
            .contract_route(routes::VERIFY_ALERT_CHANNEL, verify_channel)
            .contract_route(routes::LIST_ALERT_RULES, list_rules)
            .contract_route(routes::CREATE_ALERT_RULE, create_rule)
            .contract_route(routes::GET_ALERT_RULE, get_rule)
            .contract_route(routes::GET_ALERT_RULE_CONFIG, get_rule_config)
            .contract_route(routes::RENAME_ALERT_RULE, rename_rule)
            .contract_route(routes::UPDATE_ALERT_RULE_METADATA, update_rule_metadata)
            .contract_route(routes::UPDATE_ALERT_RULE, update_rule)
            .contract_route(routes::DELETE_ALERT_RULES, delete_rules)
            .contract_route(routes::LIST_ALERT_EVENTS, list_events)
            .contract_route(routes::GET_ALERT_EVENT, get_event)
            .contract_route(routes::GET_UNRESOLVED_ALERT_COUNT, unresolved_count)
            .contract_route(routes::ACKNOWLEDGE_ALERT_EVENTS, acknowledge)
            .contract_route(routes::RESOLVE_ALERT_EVENTS, resolve)
            .with_state(state),
        "Alert",
    )
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Channels {
    channels: Vec<citadel_alerts::AlertChannelView>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Rules {
    alert_rules: Vec<citadel_alerts::AlertRuleView>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Events {
    paged_result: citadel_alerts::AlertEventPage,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Count {
    count: i64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Ids {
    ids: Vec<Uuid>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResolveInput {
    ids: Vec<Uuid>,
    resolution_note: Option<String>,
}
#[derive(Deserialize)]
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
async fn create_channel(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    Json(mut i): Json<AlertChannelInput>,
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
async fn get_channel(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
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
async fn update_channel(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Json(patch): Json<serde_json::Value>,
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
async fn delete_channels(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    Json(i): Json<Ids>,
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
async fn verify_channel(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    Json(i): Json<VerifyInput>,
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
async fn create_rule(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    Json(mut i): Json<AlertRuleInput>,
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
async fn get_rule(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
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
async fn get_rule_config(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
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

async fn rename_rule(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    Json(mut input): Json<citadel_alerts::RenameAlertRuleInput>,
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

async fn update_rule_metadata(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Json(patch): Json<serde_json::Value>,
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

async fn update_rule(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
    h: HeaderMap,
    Json(patch): Json<serde_json::Value>,
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
async fn delete_rules(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    Json(i): Json<Ids>,
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
async fn list_events(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Query(f): Query<EventFilter>,
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
async fn get_event(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    Path(id): Path<Uuid>,
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
async fn acknowledge(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    Json(i): Json<Ids>,
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
async fn resolve(
    State(s): State<AlertsHttpState>,
    p: Option<Extension<ActorPrincipal>>,
    h: HeaderMap,
    Json(i): Json<ResolveInput>,
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
            AlertError::Validation(m) => IdentityError::Validation(m),
            AlertError::NotFound => IdentityError::NotFound,
            AlertError::Conflict(m) => IdentityError::Conflict(m),
            AlertError::Storage(m) | AlertError::Delivery(m) => IdentityError::Storage(m),
        }),
        h,
    )
}
