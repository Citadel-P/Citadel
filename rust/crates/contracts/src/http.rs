#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteAuthentication {
    Anonymous,
    Actor,
    Human,
    Administrator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteContract {
    pub method: &'static str,
    pub path: &'static str,
    pub operation_id: &'static str,
    pub summary: &'static str,
    pub public: bool,
    pub setup_exempt: bool,
    pub authentication: RouteAuthentication,
    pub request_schema: Option<&'static str>,
    pub response_schema: Option<&'static str>,
    pub success_status: u16,
}

macro_rules! route {
    ($method:literal, $path:literal, $operation:literal, $summary:literal,
     $public:literal, $setup:literal, $auth:ident, $request:expr, $response:expr, $status:literal) => {
        RouteContract {
            method: $method,
            path: $path,
            operation_id: $operation,
            summary: $summary,
            public: $public,
            setup_exempt: $setup,
            authentication: RouteAuthentication::$auth,
            request_schema: $request,
            response_schema: $response,
            success_status: $status,
        }
    };
}

pub const ROUTES: &[RouteContract] = &[
    route!(
        "get",
        "/health",
        "getHealth",
        "Process liveness",
        true,
        true,
        Anonymous,
        None,
        Some("HealthResponse"),
        200
    ),
    route!(
        "get",
        "/ready",
        "getReadiness",
        "Dependency readiness",
        false,
        true,
        Anonymous,
        None,
        Some("ReadinessResponse"),
        200
    ),
    route!(
        "get",
        "/metrics",
        "getMetrics",
        "OpenMetrics diagnostics",
        false,
        true,
        Anonymous,
        None,
        None,
        200
    ),
    route!(
        "get",
        "/api/v1/setup/status",
        "getSetupStatus",
        "Get setup status",
        false,
        true,
        Anonymous,
        None,
        Some("SetupStatusResponse"),
        200
    ),
    route!(
        "post",
        "/api/v1/setup/initialize",
        "initializeCitadel",
        "Initialize Citadel",
        false,
        true,
        Anonymous,
        Some("InitializeCitadelRequest"),
        Some("LoginResponse"),
        200
    ),
    route!(
        "post",
        "/api/v1/authentication/login",
        "login",
        "Sign in",
        false,
        false,
        Anonymous,
        Some("LoginRequest"),
        Some("LoginResponse"),
        200
    ),
    route!(
        "get",
        "/api/v1/authentication/refresh",
        "refreshToken",
        "Refresh browser access",
        false,
        false,
        Anonymous,
        None,
        Some("AccessTokenResponse"),
        200
    ),
    route!(
        "post",
        "/api/v1/authentication/logout",
        "logout",
        "End browser session",
        false,
        false,
        Human,
        None,
        None,
        204
    ),
    route!(
        "get",
        "/api/v1/roles/permissions/matrix",
        "getPermissionMatrix",
        "Get permission matrix",
        true,
        false,
        Anonymous,
        None,
        Some("PermissionMatrixResponse"),
        200
    ),
    route!(
        "get",
        "/api/v1/profile",
        "getCurrentProfile",
        "Get current profile",
        false,
        false,
        Human,
        None,
        Some("CurrentProfileView"),
        200
    ),
    route!(
        "patch",
        "/api/v1/profile",
        "updateCurrentProfile",
        "Update current profile",
        false,
        false,
        Human,
        Some("UpdateCurrentProfileRequest"),
        Some("CurrentProfileView"),
        200
    ),
    route!(
        "get",
        "/api/v1/profile/preferences",
        "getProfilePreferences",
        "Get current profile preferences",
        false,
        false,
        Human,
        None,
        Some("UserPreferencesView"),
        200
    ),
    route!(
        "patch",
        "/api/v1/profile/preferences",
        "patchProfilePreferences",
        "Update current profile preferences",
        false,
        false,
        Human,
        Some("PatchUserPreferencesRequest"),
        Some("UserPreferencesView"),
        200
    ),
    route!(
        "post",
        "/api/v1/profile/change-password",
        "changeCurrentPassword",
        "Change current profile password",
        false,
        false,
        Human,
        Some("ChangeCurrentPasswordRequest"),
        None,
        204
    ),
    route!(
        "get",
        "/api/v1/profile/sessions",
        "listProfileSessions",
        "List current profile sessions",
        false,
        false,
        Human,
        None,
        Some("UserSessionsView"),
        200
    ),
    route!(
        "delete",
        "/api/v1/profile/sessions/{sessionId}",
        "revokeProfileSession",
        "Revoke a profile session",
        false,
        false,
        Human,
        None,
        None,
        204
    ),
    route!(
        "delete",
        "/api/v1/profile/sessions",
        "revokeOtherProfileSessions",
        "Revoke other profile sessions",
        false,
        false,
        Human,
        None,
        Some("RevokeOtherProfileSessionsView"),
        200
    ),
    route!(
        "get",
        "/api/v1/serviceAccounts",
        "listServiceAccounts",
        "List Service Accounts",
        true,
        false,
        Actor,
        None,
        Some("ServiceAccountsResponse"),
        200
    ),
    route!(
        "post",
        "/api/v1/serviceAccounts",
        "createServiceAccount",
        "Create a Service Account",
        true,
        false,
        Administrator,
        Some("CreateServiceAccountRequest"),
        Some("ServiceAccountView"),
        200
    ),
    route!(
        "delete",
        "/api/v1/serviceAccounts",
        "archiveServiceAccounts",
        "Archive Service Accounts",
        true,
        false,
        Administrator,
        Some("ArchiveServiceAccountsRequest"),
        None,
        204
    ),
    route!(
        "get",
        "/api/v1/serviceAccounts/limits",
        "getServiceAccountLimits",
        "Get Service Account limits",
        true,
        false,
        Actor,
        None,
        Some("ServiceAccountLimitsView"),
        200
    ),
    route!(
        "get",
        "/api/v1/serviceAccounts/{id}",
        "getServiceAccount",
        "Get a Service Account",
        true,
        false,
        Actor,
        None,
        Some("ServiceAccountDetailResponse"),
        200
    ),
    route!(
        "patch",
        "/api/v1/serviceAccounts/{id}",
        "updateServiceAccount",
        "Update a Service Account",
        true,
        false,
        Administrator,
        Some("UpdateServiceAccountRequest"),
        Some("ServiceAccountView"),
        200
    ),
    route!(
        "post",
        "/api/v1/serviceAccounts/rename",
        "renameServiceAccount",
        "Rename a Service Account",
        true,
        false,
        Administrator,
        Some("RenameServiceAccountRequest"),
        Some("ServiceAccountView"),
        200
    ),
    route!(
        "post",
        "/api/v1/serviceAccounts/{id}/roles",
        "addServiceAccountRole",
        "Assign a Role to a Service Account",
        true,
        false,
        Administrator,
        Some("AddServiceAccountRoleRequest"),
        Some("ServiceAccountView"),
        200
    ),
    route!(
        "delete",
        "/api/v1/serviceAccounts/{id}/roles/{roleId}",
        "removeServiceAccountRole",
        "Remove a Role from a Service Account",
        true,
        false,
        Administrator,
        None,
        Some("ServiceAccountView"),
        200
    ),
    route!(
        "post",
        "/api/v1/serviceAccounts/{id}/resource-accesses",
        "addServiceAccountResourceAccess",
        "Add a resource override to a Service Account",
        true,
        false,
        Administrator,
        Some("AddServiceAccountResourceAccessRequest"),
        Some("ServiceAccountView"),
        200
    ),
    route!(
        "delete",
        "/api/v1/serviceAccounts/{id}/resource-accesses/{resourceAccessId}",
        "removeServiceAccountResourceAccess",
        "Remove a resource override from a Service Account",
        true,
        false,
        Administrator,
        None,
        Some("ServiceAccountView"),
        200
    ),
    route!(
        "get",
        "/api/v1/serviceAccounts/{id}/tokens",
        "listServiceAccountTokens",
        "List Service Account tokens",
        true,
        false,
        Actor,
        None,
        Some("ServiceAccountTokensResponse"),
        200
    ),
    route!(
        "post",
        "/api/v1/serviceAccounts/{id}/tokens",
        "createServiceAccountToken",
        "Create a Service Account token",
        true,
        false,
        Human,
        Some("CreateServiceAccountTokenRequest"),
        Some("CreatedServiceAccountTokenView"),
        201
    ),
    route!(
        "delete",
        "/api/v1/serviceAccounts/{id}/tokens/{tokenId}",
        "revokeServiceAccountToken",
        "Revoke a Service Account token",
        true,
        false,
        Human,
        None,
        None,
        204
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn operation_ids_and_method_paths_are_unique() {
        let mut operations = HashSet::new();
        let mut endpoints = HashSet::new();
        for route in ROUTES {
            assert!(operations.insert(route.operation_id));
            assert!(endpoints.insert((route.method, route.path)));
        }
    }

    #[test]
    fn browser_identity_routes_are_not_in_the_public_document() {
        assert!(ROUTES.iter().filter(|route| route.public).all(|route| {
            !route.path.starts_with("/api/v1/setup")
                && !route.path.starts_with("/api/v1/authentication")
                && !route.path.starts_with("/api/v1/profile")
        }));
    }
}
