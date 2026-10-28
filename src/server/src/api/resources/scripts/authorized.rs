use super::{
    capabilities::{capabilities, granted},
    views::{AuthorizedScript, ScriptView},
};
use crate::api::error::ApiError;
use citadel_identity::ActorPrincipal;
use citadel_primitives::PermissionLevel;
use citadel_scripts::ScriptError;
pub(crate) async fn authorized_scripts(
    store: &dyn citadel_scripts::ScriptRepository,
    principal: &ActorPrincipal,
    scripts: Vec<citadel_scripts::Script>,
) -> Result<Vec<AuthorizedScript>, ApiError> {
    let ids: Vec<_> = scripts.iter().map(|script| script.id).collect();
    let permissions = if principal.is_administrator() {
        Default::default()
    } else {
        store
            .permissions(principal.actor_id, &ids)
            .await
            .map_err(map_error)?
    };
    scripts
        .into_iter()
        .map(|script| {
            let level = if principal.is_administrator() {
                citadel_primitives::EffectivePermission::Administrator
            } else {
                granted(
                    permissions
                        .get(&script.id)
                        .copied()
                        .unwrap_or(PermissionLevel::None),
                )
            };
            Ok(AuthorizedScript {
                script: ScriptView::try_from(script).map_err(ApiError::internal)?,
                capabilities: capabilities(level),
            })
        })
        .collect()
}
pub(crate) fn map_error(error: ScriptError) -> ApiError {
    match error {
        ScriptError::LicenseRequired => ApiError::LicenseRequired("automated-operations"),
        ScriptError::Validation(message) => ApiError::Validation(message),
        ScriptError::NotFound => ApiError::NotFound,
        ScriptError::Conflict(message) => ApiError::Conflict(message),
        source @ ScriptError::Storage(_) => ApiError::internal(source),
        ScriptError::External(message) => ApiError::External(message),
    }
}
