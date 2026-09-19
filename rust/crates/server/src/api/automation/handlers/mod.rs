use super::capabilities::{capabilities, granted};
mod actions;
pub(crate) use actions::authorized_actions;
use actions::*;
mod runs;
use super::patch::{UpdateAutomationActionInput, UpdateAutomationActionMetadata};
use runs::*;

use super::{requests::*, views::*};

use crate::request_validation::ApiPath;

use crate::request_validation::ApiQuery;

use crate::request_validation::ValidatedJson;

use std::sync::Arc;

use axum::body::Body;

use axum::extract::rejection::JsonRejection;

use axum::extract::{Extension, RawQuery, State};

use axum::http::{HeaderMap, StatusCode};

use axum::response::IntoResponse;

use axum::{Json, Router};

use citadel_automation::{AutomationError, AutomationService};

use citadel_domain::{PermissionLevel, ResourceType};

use citadel_identity::{ActorPrincipal, IdentityError, IdentityService};

use serde_json::Value;

use uuid::Uuid;

use crate::identity_http::{IdentityHttpResult, identity_result, no_store};

use crate::openapi::router::OpenApiRouterExt;

#[derive(Clone)]
pub struct AutomationHttpState {
    pub identity: Arc<IdentityService>,
    pub automation: Arc<AutomationService>,
}

pub fn router(state: AutomationHttpState) -> Router {
    crate::realtime::notify_mutations(
        documented_routes().split_for_parts().0.with_state(state),
        "AutomationAction",
    )
}

fn actor(
    value: Option<Extension<ActorPrincipal>>,
    headers: &HeaderMap,
) -> IdentityHttpResult<ActorPrincipal> {
    identity_result(
        value
            .map(|Extension(value)| value)
            .ok_or(IdentityError::Unauthenticated),
        headers,
    )
}

async fn authorize_global(
    state: &AutomationHttpState,
    principal: &ActorPrincipal,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    identity_result(
        match level {
            PermissionLevel::Read => {
                state
                    .identity
                    .require_scope::<citadel_automation::permissions::ReadAutomationAction>(
                        principal,
                    )
                    .await
            }
            PermissionLevel::Write => {
                state
                    .identity
                    .require_scope::<citadel_automation::permissions::WriteAutomationAction>(
                        principal,
                    )
                    .await
            }
            PermissionLevel::Execute => {
                state
                    .identity
                    .require_scope::<citadel_automation::permissions::ExecuteAutomationAction>(
                        principal,
                    )
                    .await
            }
            _ => Err(IdentityError::Forbidden),
        },
        headers,
    )?;
    Ok(())
}

async fn authorize(
    state: &AutomationHttpState,
    principal: &ActorPrincipal,
    id: Uuid,
    level: PermissionLevel,
    headers: &HeaderMap,
) -> IdentityHttpResult<()> {
    identity_result(
        match level {
            PermissionLevel::Read => {
                state
                    .identity
                    .require_resource::<citadel_automation::permissions::ReadAutomationAction>(
                        principal, id,
                    )
                    .await
            }
            PermissionLevel::Write => {
                state
                    .identity
                    .require_resource::<citadel_automation::permissions::WriteAutomationAction>(
                        principal, id,
                    )
                    .await
            }
            PermissionLevel::Execute => {
                state
                    .identity
                    .require_resource::<citadel_automation::permissions::ExecuteAutomationAction>(
                        principal, id,
                    )
                    .await
            }
            _ => Err(IdentityError::Forbidden),
        },
        headers,
    )?;
    Ok(())
}

fn map_error(error: AutomationError) -> IdentityError {
    match error {
        AutomationError::LicenseRequired => IdentityError::LicenseRequired("automated-operations"),
        AutomationError::Validation(message) => IdentityError::Validation(message),
        AutomationError::NotFound => IdentityError::NotFound,
        AutomationError::Conflict(message) => IdentityError::Conflict(message),
        AutomationError::Storage(message) => IdentityError::Storage(message),
        AutomationError::External(message) => IdentityError::External(message),
    }
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<AutomationHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(list))
        .normalized_routes(utoipa_axum::routes!(create))
        .normalized_routes(utoipa_axum::routes!(get_one))
        .normalized_routes(utoipa_axum::routes!(rename))
        .normalized_routes(utoipa_axum::routes!(update))
        .normalized_routes(utoipa_axum::routes!(update_metadata))
        .normalized_routes(utoipa_axum::routes!(remove))
        .normalized_routes(utoipa_axum::routes!(run_action))
        .normalized_routes(utoipa_axum::routes!(test_action))
        .normalized_routes(utoipa_axum::routes!(list_runs))
        .normalized_routes(utoipa_axum::routes!(get_run))
        .normalized_routes(utoipa_axum::routes!(run_logs))
        .normalized_routes(utoipa_axum::routes!(cancel_run))
}
