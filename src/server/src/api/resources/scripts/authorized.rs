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
    let levels = levels(store, principal, &ids).await?;
    scripts
        .into_iter()
        .map(|script| {
            let level = levels(script.id);
            Ok(AuthorizedScript {
                script: ScriptView::try_from(script).map_err(ApiError::internal)?,
                capabilities: capabilities(level),
            })
        })
        .collect()
}
async fn levels(
    store: &dyn citadel_scripts::ScriptRepository,
    principal: &ActorPrincipal,
    ids: &[uuid::Uuid],
) -> Result<impl Fn(uuid::Uuid) -> citadel_primitives::EffectivePermission + use<>, ApiError> {
    let admin = principal.is_administrator();
    let permissions = if admin {
        Default::default()
    } else {
        store
            .permissions(principal.actor_id, ids)
            .await
            .map_err(map_error)?
    };
    Ok(move |id| {
        if admin {
            citadel_primitives::EffectivePermission::Administrator
        } else {
            granted(
                permissions
                    .get(&id)
                    .copied()
                    .unwrap_or(PermissionLevel::None),
            )
        }
    })
}

pub(crate) async fn script_summaries(
    store: &dyn citadel_scripts::ScriptRepository,
    principal: &ActorPrincipal,
    scripts: Vec<citadel_scripts::ScriptSummary>,
) -> Result<Vec<super::views::ScriptSummaryView>, ApiError> {
    let ids = scripts.iter().map(|s| s.id).collect::<Vec<_>>();
    let levels = levels(store, principal, &ids).await?;
    scripts
        .into_iter()
        .map(|s| {
            Ok(super::views::ScriptSummaryView {
                id: s.id,
                name: s.name,
                description: s.description,
                enabled: s.enabled,
                control_state: s.control_state,
                schedule_enabled: s.schedule_enabled,
                schedule_cron: s.schedule_cron,
                tags: s.tags.into_iter().map(Into::into).collect(),
                latest_run: s
                    .latest_run
                    .map(super::views::ScriptRunView::try_from)
                    .transpose()
                    .map_err(ApiError::internal)?,
                capabilities: capabilities(levels(s.id)),
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
