use super::{
    capabilities::{granted, pool_capabilities, project_capabilities},
    views::{AuthorizedPool, AuthorizedProject, BuildAgentPoolView, BuildProjectView},
};
use citadel_builds::BuildError;
use citadel_identity::ActorPrincipal;
use citadel_primitives::{EffectivePermission, PermissionLevel};
pub(crate) async fn authorized_pools(
    store: &dyn citadel_builds::BuildRepository,
    principal: &ActorPrincipal,
    values: Vec<citadel_builds::BuildAgentPool>,
) -> Result<Vec<AuthorizedPool>, BuildError> {
    let permissions = if principal.is_administrator() || values.is_empty() {
        Default::default()
    } else {
        let ids = values.iter().map(|pool| pool.id).collect::<Vec<_>>();
        store.pool_permissions(principal.actor_id, &ids).await?
    };
    values
        .into_iter()
        .map(|pool| {
            Ok(AuthorizedPool {
                capabilities: pool_capabilities(if principal.is_administrator() {
                    EffectivePermission::Administrator
                } else {
                    granted(
                        permissions
                            .get(&pool.id)
                            .copied()
                            .unwrap_or(PermissionLevel::None),
                    )
                }),
                pool: BuildAgentPoolView::try_from(pool)
                    .map_err(|e| BuildError::Storage(e.to_string()))?,
            })
        })
        .collect()
}
pub(crate) async fn authorized_projects(
    store: &dyn citadel_builds::BuildRepository,
    principal: &ActorPrincipal,
    projects: Vec<citadel_builds::BuildProject>,
) -> Result<Vec<AuthorizedProject>, BuildError> {
    let ids: Vec<_> = projects.iter().map(|project| project.id).collect();
    let permissions = if principal.is_administrator() {
        Default::default()
    } else {
        store.project_permissions(principal.actor_id, &ids).await?
    };
    projects
        .into_iter()
        .map(|project| {
            let level = if principal.is_administrator() {
                EffectivePermission::Administrator
            } else {
                granted(
                    permissions
                        .get(&project.id)
                        .copied()
                        .unwrap_or(PermissionLevel::None),
                )
            };
            Ok(AuthorizedProject {
                project: BuildProjectView::try_from(project)
                    .map_err(|e| BuildError::Storage(e.to_string()))?,
                capabilities: project_capabilities(level),
            })
        })
        .collect()
}
