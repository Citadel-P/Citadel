mod channels;
use channels::*;
mod rules;
use rules::*;
mod events;
use super::{requests::*, views::*};
use events::*;

use crate::request_validation::ApiPath;

use crate::request_validation::ApiQuery;

use crate::request_validation::ValidatedJson;

use std::sync::Arc;

use axum::extract::{Extension, State};

use axum::http::{HeaderMap, StatusCode};

use axum::response::IntoResponse;

use axum::{Json, Router};

use citadel_alerts::{AlertDelivery, AlertError, AlertRepository};

use citadel_primitives::{PermissionLevel, ResourceType};

use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};

use serde_json::json;

use tokio_util::sync::CancellationToken;

use uuid::Uuid;

use crate::identity_http::{IdentityHttpResult, identity_result, no_store};

use crate::openapi::router::OpenApiRouterExt;

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
        _ => Err(IdentityError::Forbidden),
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
