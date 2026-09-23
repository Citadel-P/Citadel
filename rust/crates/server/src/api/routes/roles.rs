use crate::{
    api::{
        error::{HttpResult, api_result, no_store},
        resources::roles::{
            requests::{
                CreateRoleRequest, DeleteRolesRequest, PatchRolePermissionsRequest,
                RenameRoleRequest,
            },
            views::{PermissionMatrixViewItem, RolesResponse},
        },
        routes::authentication::require_human_administrator,
    },
    openapi::router::OpenApiRouterExt,
    request_validation::ApiPath,
};

use axum::{
    Json, Router,
    extract::{Extension, State, rejection::JsonRejection},
    http::HeaderMap,
    response::{IntoResponse, Response},
};

use citadel_identity::{
    ActorPrincipal, IdentityError, IdentityService, PermissionGrant, RoleMutationService,
    RoleReadService, permission_matrix,
};

use citadel_primitives::{PermissionLevel, ResourceType, SpecificPermission};

use std::{collections::BTreeMap, sync::Arc};

use uuid::Uuid;

#[derive(Clone)]
pub struct RolesHttpState {
    pub identity: Arc<IdentityService>,
    pub roles: Arc<RoleReadService>,
    pub mutations: Arc<RoleMutationService>,
}

pub fn router(state: RolesHttpState) -> Router {
    crate::realtime::notify_mutations(
        documented_routes().split_for_parts().0.with_state(state),
        "Role",
    )
}

pub(crate) fn documented_routes() -> utoipa_axum::router::OpenApiRouter<RolesHttpState> {
    utoipa_axum::router::OpenApiRouter::new()
        .normalized_routes(utoipa_axum::routes!(list))
        .normalized_routes(utoipa_axum::routes!(get))
        .normalized_routes(utoipa_axum::routes!(create))
        .normalized_routes(utoipa_axum::routes!(patch_permissions))
        .normalized_routes(utoipa_axum::routes!(rename))
        .normalized_routes(utoipa_axum::routes!(delete))
        .normalized_routes(utoipa_axum::routes!(get_permission_matrix))
}

#[utoipa::path(
    get,
    path = "/api/v1/roles",
    operation_id = "listRoles",
    tag = "Roles",
    summary = "Get all Roles",
    responses(
        (status = 200, description = "Success", body = RolesResponse, content_type = "application/json"),
        crate::openapi::errors::AccessErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn list(
    State(state): State<RolesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_human_administrator(principal), &headers)?;
    let permission = api_result(
        role_permission(&state.identity, &principal, PermissionLevel::Read).await,
        &headers,
    )?;
    let roles = api_result(state.roles.list().await, &headers)?;
    Ok(no_store(
        Json(RolesResponse {
            roles: roles.into_iter().map(Into::into).collect(),
            capabilities: permission.into(),
        })
        .into_response(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/roles/{id}",
    operation_id = "getRole",
    tag = "Roles",
    summary = "Get a Role by ID",
    responses(
        (status = 200, description = "Success", body = crate::api::resources::roles::views::RoleView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get(
    State(state): State<RolesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
) -> HttpResult {
    let principal = api_result(require_human_administrator(principal), &headers)?;
    api_result(
        role_permission(&state.identity, &principal, PermissionLevel::Read).await,
        &headers,
    )?;
    let role = api_result(state.roles.get(id).await, &headers)?;
    Ok(no_store(
        Json(crate::api::resources::roles::views::RoleView::from(role)).into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/roles",
    operation_id = "createRole",
    tag = "Roles",
    summary = "Create a Role",
    request_body = CreateRoleRequest,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::roles::views::RoleView, content_type = "application/json"),
        crate::openapi::errors::CreateErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn create(
    State(state): State<RolesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    request: Result<Json<CreateRoleRequest>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(
        authorize(&state, principal, PermissionLevel::Write).await,
        &headers,
    )?;
    let request: CreateRoleRequest = api_result(role_json(request), &headers)?;
    let request: citadel_identity::CreateRole = request.into();
    let role = api_result(
        state.mutations.create(request, principal.actor_id).await,
        &headers,
    )?;
    Ok(no_store(
        Json(crate::api::resources::roles::views::RoleView::from(role)).into_response(),
    ))
}

#[utoipa::path(
    patch,
    path = "/api/v1/roles/{id}/permissions",
    operation_id = "updateRolePermissions",
    tag = "Roles",
    summary = "Update Role permissions",
    request_body(content(
        (PatchRolePermissionsRequest = "application/merge-patch+json"),
        (PatchRolePermissionsRequest = "application/json")
    )),
    responses(
        (status = 200, description = "Success", body = crate::api::resources::roles::views::RoleView, content_type = "application/json"),
        crate::openapi::errors::ResourceErrors
    ),
    params(("id" = uuid::Uuid, Path)),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn patch_permissions(
    State(state): State<RolesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    ApiPath(id): ApiPath<Uuid>,
    headers: HeaderMap,
    request: Result<Json<PatchRolePermissionsRequest>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(
        authorize(&state, principal, PermissionLevel::Write).await,
        &headers,
    )?;
    let request: PatchRolePermissionsRequest = api_result(role_json(request), &headers)?;
    let request: citadel_identity::PatchRolePermissions = request.into();
    let role = api_result(
        state
            .mutations
            .patch_permissions(id, request, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(no_store(
        Json(crate::api::resources::roles::views::RoleView::from(role)).into_response(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/roles/rename",
    operation_id = "renameRole",
    tag = "Roles",
    summary = "Rename a Role",
    request_body = RenameRoleRequest,
    responses(
        (status = 200, description = "Success", body = crate::api::resources::roles::views::RoleView, content_type = "application/json"),
        crate::openapi::errors::ResourceMutationErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn rename(
    State(state): State<RolesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    request: Result<Json<RenameRoleRequest>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(
        authorize(&state, principal, PermissionLevel::Write).await,
        &headers,
    )?;
    let request: RenameRoleRequest = api_result(role_json(request), &headers)?;
    let request: citadel_identity::RenameRole = request.into();
    let role = api_result(
        state
            .mutations
            .rename(request.id, &request.name, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(no_store(
        Json(crate::api::resources::roles::views::RoleView::from(role)).into_response(),
    ))
}

#[utoipa::path(
    delete,
    path = "/api/v1/roles",
    operation_id = "deleteRoles",
    tag = "Roles",
    summary = "Delete Roles",
    request_body = DeleteRolesRequest,
    responses(
        (status = 204, description = "Success"),
        crate::openapi::errors::ResourceErrors
    ),
    security(("Bearer" = [])),
    extensions(("x-citadel-principal" = json!("administrator")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn delete(
    State(state): State<RolesHttpState>,
    principal: Option<Extension<ActorPrincipal>>,
    headers: HeaderMap,
    request: Result<Json<DeleteRolesRequest>, JsonRejection>,
) -> HttpResult {
    let principal = api_result(
        authorize(&state, principal, PermissionLevel::Execute).await,
        &headers,
    )?;
    let request: DeleteRolesRequest = api_result(role_json(request), &headers)?;
    let request: citadel_identity::DeleteRoles = request.into();
    api_result(
        state
            .mutations
            .delete(request.ids, principal.actor_id)
            .await,
        &headers,
    )?;
    Ok(axum::http::StatusCode::NO_CONTENT.into_response())
}

fn role_json<T>(payload: Result<Json<T>, JsonRejection>) -> Result<T, crate::api::error::ApiError> {
    payload
        .map(|Json(value)| value)
        .map_err(crate::request_validation::invalid_json)
}

async fn authorize(
    state: &RolesHttpState,
    principal: Option<Extension<ActorPrincipal>>,
    level: PermissionLevel,
) -> Result<ActorPrincipal, IdentityError> {
    let principal = require_human_administrator(principal)?;
    state
        .identity
        .authorize(&principal, ResourceType::Role, level, None)
        .await?;
    Ok(principal)
}

async fn role_permission(
    identity: &IdentityService,
    principal: &ActorPrincipal,
    level: PermissionLevel,
) -> Result<PermissionGrant, IdentityError> {
    match identity
        .global_permission(principal, ResourceType::Role)
        .await?
    {
        Some(permission) if permission.level.grants(level) => Ok(permission),
        _ => Err(IdentityError::Forbidden),
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/roles/permissions/matrix",
    operation_id = "getPermissionMatrix",
    tag = "Roles",
    summary = "Get permission matrix",
    responses(
        (status = 200, description = "Success", body = ref("#/components/schemas/PermissionMatrixResponse"), content_type = "application/json"),
        crate::openapi::errors::RequestErrors
    ),
    security(),
    extensions(("x-citadel-principal" = json!("anonymous")), ("x-citadel-public" = json!(false)), ("x-citadel-setup-exempt" = json!(false)))
)]
async fn get_permission_matrix() -> Response {
    let matrix = permission_matrix()
        .into_iter()
        .map(|(resource_type, capability)| {
            let specifics = capability
                .specifics
                .iter()
                .map(|(permission, level)| (permission_name(*permission).to_owned(), *level))
                .collect::<BTreeMap<_, _>>();
            let labels = capability
                .specifics
                .iter()
                .map(|(permission, _)| {
                    (
                        permission_name(*permission).to_owned(),
                        specific_permission_label(*permission).to_owned(),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            (
                resource_name(resource_type).to_owned(),
                PermissionMatrixViewItem {
                    maximum_level: capability.maximum_level,
                    specific_permissions: specifics,
                    label: resource_label(resource_type).to_owned(),
                    specific_permission_labels: labels,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    no_store(Json(matrix).into_response())
}

const fn resource_name(resource: ResourceType) -> &'static str {
    match resource {
        ResourceType::Platform => "Platform",
        ResourceType::Deployment => "Deployment",
        ResourceType::Stack => "Stack",
        ResourceType::Registry => "Registry",
        ResourceType::GitRepository => "GitRepository",
        ResourceType::GitAccount => "GitAccount",
        ResourceType::Alert => "Alert",
        ResourceType::AlertChannel => "AlertChannel",
        ResourceType::User => "User",
        ResourceType::Team => "Team",
        ResourceType::Role => "Role",
        ResourceType::Binding => "Binding",
        ResourceType::Tag => "Tag",
        ResourceType::AutomationAction => "AutomationAction",
        ResourceType::License => "License",
        ResourceType::BackupRepository => "BackupRepository",
        ResourceType::BackupPolicy => "BackupPolicy",
        ResourceType::Volume => "Volume",
        ResourceType::Build => "Build",
        ResourceType::BuildAgentPool => "BuildAgentPool",
        ResourceType::SwarmService => "SwarmService",
        ResourceType::ServiceAccount => "ServiceAccount",
    }
}

const fn resource_label(resource: ResourceType) -> &'static str {
    match resource {
        ResourceType::Binding => "Bindings",
        ResourceType::Tag => "Tags",
        ResourceType::Alert => "Alert Rules",
        ResourceType::GitRepository => "Git Repository",
        ResourceType::GitAccount => "Git Account",
        ResourceType::AlertChannel => "Alert Channel",
        ResourceType::AutomationAction => "Automation",
        ResourceType::BackupRepository => "Backup Repositories",
        ResourceType::BackupPolicy => "Backups",
        ResourceType::Build => "Builds",
        ResourceType::BuildAgentPool => "Build Pools",
        ResourceType::SwarmService => "Swarm Services",
        _ => resource_name(resource),
    }
}

const fn permission_name(permission: SpecificPermission) -> &'static str {
    match permission {
        SpecificPermission::Logs => "Logs",
        SpecificPermission::Inspect => "Inspect",
        SpecificPermission::Apply => "Apply",
        SpecificPermission::Pull => "Pull",
        SpecificPermission::Terminal => "Terminal",
        SpecificPermission::ResourceBindings => "ResourceBindings",
        SpecificPermission::Releases => "Releases",
        SpecificPermission::Restore => "Restore",
        SpecificPermission::Browse => "Browse",
        SpecificPermission::Download => "Download",
        SpecificPermission::ManageNodeAgents => "ManageNodeAgents",
        SpecificPermission::Use => "Use",
        SpecificPermission::ManageCredentials => "ManageCredentials",
    }
}

const fn specific_permission_label(permission: SpecificPermission) -> &'static str {
    match permission {
        SpecificPermission::ResourceBindings => "Resource Bindings",
        _ => permission_name(permission),
    }
}
