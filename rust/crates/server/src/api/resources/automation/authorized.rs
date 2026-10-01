use super::{
    capabilities::{capabilities, granted},
    views::{AuthorizedAction, AutomationActionView},
};
use crate::api::error::ApiError;
use citadel_automation::AutomationError;
use citadel_identity::ActorPrincipal;
use citadel_primitives::PermissionLevel;
pub(crate) async fn authorized_actions(
    store: &dyn citadel_automation::AutomationRepository,
    principal: &ActorPrincipal,
    actions: Vec<citadel_automation::AutomationAction>,
) -> Result<Vec<AuthorizedAction>, ApiError> {
    let ids: Vec<_> = actions.iter().map(|action| action.id).collect();
    let permissions = if principal.is_administrator() {
        Default::default()
    } else {
        store
            .permissions(principal.actor_id, &ids)
            .await
            .map_err(map_error)?
    };
    actions
        .into_iter()
        .map(|action| {
            let level = if principal.is_administrator() {
                citadel_primitives::EffectivePermission::Administrator
            } else {
                granted(
                    permissions
                        .get(&action.id)
                        .copied()
                        .unwrap_or(PermissionLevel::None),
                )
            };
            Ok(AuthorizedAction {
                action: AutomationActionView::try_from(action).map_err(ApiError::internal)?,
                capabilities: capabilities(level),
            })
        })
        .collect()
}
pub(crate) fn map_error(error: AutomationError) -> ApiError {
    match error {
        AutomationError::LicenseRequired => ApiError::LicenseRequired("automated-operations"),
        AutomationError::Validation(message) => ApiError::Validation(message),
        AutomationError::NotFound => ApiError::NotFound,
        AutomationError::Conflict(message) => ApiError::Conflict(message),
        source @ AutomationError::Storage(_) => ApiError::internal(source),
        AutomationError::External(message) => ApiError::External(message),
    }
}
