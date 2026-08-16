#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
    Patch,
    Delete,
}

impl HttpMethod {
    #[must_use]
    pub const fn as_openapi_str(self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::Post => "post",
            Self::Patch => "patch",
            Self::Delete => "delete",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteAuthentication {
    Anonymous,
    Actor,
    Human,
    Administrator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ParameterLocation {
    Path,
    Query,
}

impl ParameterLocation {
    #[must_use]
    pub const fn as_openapi_str(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Query => "query",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegerFormat {
    Int32,
    Int64,
}

impl IntegerFormat {
    #[must_use]
    pub const fn as_openapi_str(self) -> &'static str {
        match self {
            Self::Int32 => "int32",
            Self::Int64 => "int64",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterSchema {
    String,
    Boolean {
        default: Option<bool>,
    },
    Uuid,
    Integer {
        format: IntegerFormat,
        minimum: Option<i64>,
        maximum: Option<i64>,
        default: Option<i64>,
    },
    Reference(&'static str),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParameterContract {
    pub name: &'static str,
    pub location: ParameterLocation,
    pub required: bool,
    pub schema: ParameterSchema,
}

impl ParameterContract {
    #[must_use]
    pub const fn path_uuid(name: &'static str) -> Self {
        Self {
            name,
            location: ParameterLocation::Path,
            required: true,
            schema: ParameterSchema::Uuid,
        }
    }

    #[must_use]
    pub const fn query(name: &'static str, schema: ParameterSchema) -> Self {
        Self {
            name,
            location: ParameterLocation::Query,
            required: false,
            schema,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteContract {
    pub method: HttpMethod,
    pub path: &'static str,
    pub operation_id: &'static str,
    pub summary: &'static str,
    pub public: bool,
    pub setup_exempt: bool,
    pub authentication: RouteAuthentication,
    pub request_schema: Option<&'static str>,
    pub response_schema: Option<&'static str>,
    pub success_status: u16,
    pub parameters: &'static [ParameterContract],
}

const ACTIVITY_FILTER_PARAMETERS: &[ParameterContract] = &[
    ParameterContract::query("ResourceId", ParameterSchema::Uuid),
    ParameterContract::query(
        "ResourceType",
        ParameterSchema::Reference("ActivityResourceType"),
    ),
    ParameterContract::query("EventType", ParameterSchema::Reference("ActivityEventType")),
    ParameterContract::query(
        "Page",
        ParameterSchema::Integer {
            format: IntegerFormat::Int32,
            minimum: Some(1),
            maximum: None,
            default: Some(1),
        },
    ),
    ParameterContract::query(
        "PageSize",
        ParameterSchema::Integer {
            format: IntegerFormat::Int32,
            minimum: Some(1),
            maximum: Some(500),
            default: Some(50),
        },
    ),
];

const USER_FILTER_PARAMETERS: &[ParameterContract] = &[
    ParameterContract::query("Name", ParameterSchema::String),
    ParameterContract::query(
        "Page",
        ParameterSchema::Integer {
            format: IntegerFormat::Int32,
            minimum: Some(1),
            maximum: None,
            default: Some(1),
        },
    ),
    ParameterContract::query(
        "PageSize",
        ParameterSchema::Integer {
            format: IntegerFormat::Int32,
            minimum: Some(1),
            maximum: Some(500),
            default: Some(50),
        },
    ),
];

const USER_SEARCH_PARAMETERS: &[ParameterContract] = &[
    ParameterContract::query("Query", ParameterSchema::String),
    ParameterContract::query(
        "Limit",
        ParameterSchema::Integer {
            format: IntegerFormat::Int32,
            minimum: Some(1),
            maximum: Some(50),
            default: Some(20),
        },
    ),
];

const SERVICE_ACCOUNT_FILTER_PARAMETERS: &[ParameterContract] = &[
    ParameterContract::query(
        "Page",
        ParameterSchema::Integer {
            format: IntegerFormat::Int64,
            minimum: Some(1),
            maximum: None,
            default: Some(1),
        },
    ),
    ParameterContract::query(
        "PageSize",
        ParameterSchema::Integer {
            format: IntegerFormat::Int64,
            minimum: Some(1),
            maximum: None,
            default: Some(50),
        },
    ),
    ParameterContract::query("Name", ParameterSchema::String),
    ParameterContract::query(
        "IncludeArchived",
        ParameterSchema::Boolean {
            default: Some(false),
        },
    ),
];

const TOKEN_FILTER_PARAMETERS: &[ParameterContract] = &[
    ParameterContract::path_uuid("id"),
    ParameterContract::query(
        "Page",
        ParameterSchema::Integer {
            format: IntegerFormat::Int64,
            minimum: Some(1),
            maximum: None,
            default: Some(1),
        },
    ),
    ParameterContract::query(
        "PageSize",
        ParameterSchema::Integer {
            format: IntegerFormat::Int64,
            minimum: Some(1),
            maximum: None,
            default: Some(50),
        },
    ),
];

macro_rules! route_catalog {
    ($(
        $name:ident => {
            method: $method:ident,
            path: $path:literal,
            operation_id: $operation_id:literal,
            summary: $summary:literal,
            public: $public:literal,
            setup_exempt: $setup_exempt:literal,
            authentication: $authentication:ident,
            request_schema: $request_schema:expr,
            response_schema: $response_schema:expr,
            success_status: $success_status:literal,
            parameters: $parameters:expr
        }
    ),+ $(,)?) => {
        pub mod routes {
            use super::*;

            $(
                pub const $name: RouteContract = RouteContract {
                    method: HttpMethod::$method,
                    path: $path,
                    operation_id: $operation_id,
                    summary: $summary,
                    public: $public,
                    setup_exempt: $setup_exempt,
                    authentication: RouteAuthentication::$authentication,
                    request_schema: $request_schema,
                    response_schema: $response_schema,
                    success_status: $success_status,
                    parameters: $parameters,
                };
            )+
        }

        pub const ROUTES: &[RouteContract] = &[$(routes::$name),+];
    };
}

route_catalog! {
    GET_HEALTH => {
        method: Get, path: "/health", operation_id: "getHealth", summary: "Process liveness",
        public: true, setup_exempt: true, authentication: Anonymous,
        request_schema: None, response_schema: Some("HealthResponse"), success_status: 200,
        parameters: &[]
    },
    GET_READINESS => {
        method: Get, path: "/ready", operation_id: "getReadiness", summary: "Dependency readiness",
        public: false, setup_exempt: true, authentication: Anonymous,
        request_schema: None, response_schema: Some("ReadinessResponse"), success_status: 200,
        parameters: &[]
    },
    GET_METRICS => {
        method: Get, path: "/metrics", operation_id: "getMetrics", summary: "OpenMetrics diagnostics",
        public: false, setup_exempt: true, authentication: Anonymous,
        request_schema: None, response_schema: None, success_status: 200,
        parameters: &[]
    },
    GET_SETUP_STATUS => {
        method: Get, path: "/api/v1/setup/status", operation_id: "getSetupStatus", summary: "Get setup status",
        public: false, setup_exempt: true, authentication: Anonymous,
        request_schema: None, response_schema: Some("SetupStatusResponse"), success_status: 200,
        parameters: &[]
    },
    INITIALIZE_CITADEL => {
        method: Post, path: "/api/v1/setup/initialize", operation_id: "initializeCitadel", summary: "Initialize Citadel",
        public: false, setup_exempt: true, authentication: Anonymous,
        request_schema: Some("InitializeCitadelRequest"), response_schema: Some("LoginResponse"), success_status: 200,
        parameters: &[]
    },
    LOGIN => {
        method: Post, path: "/api/v1/authentication/login", operation_id: "login", summary: "Sign in",
        public: false, setup_exempt: false, authentication: Anonymous,
        request_schema: Some("LoginRequest"), response_schema: Some("LoginResponse"), success_status: 200,
        parameters: &[]
    },
    REFRESH_TOKEN => {
        method: Get, path: "/api/v1/authentication/refresh", operation_id: "refreshToken", summary: "Refresh browser access",
        public: false, setup_exempt: false, authentication: Anonymous,
        request_schema: None, response_schema: Some("AccessTokenResponse"), success_status: 200,
        parameters: &[]
    },
    LOGOUT => {
        method: Post, path: "/api/v1/authentication/logout", operation_id: "logout", summary: "End browser session",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: None, response_schema: None, success_status: 204,
        parameters: &[]
    },
    GET_PERMISSION_MATRIX => {
        method: Get, path: "/api/v1/roles/permissions/matrix", operation_id: "getPermissionMatrix", summary: "Get permission matrix",
        public: true, setup_exempt: false, authentication: Anonymous,
        request_schema: None, response_schema: Some("PermissionMatrixResponse"), success_status: 200,
        parameters: &[]
    },
    GET_APPLICATION_INFO => {
        method: Get, path: "/api/v1/application/info", operation_id: "getApplicationInfo", summary: "Get application information",
        public: false, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ApplicationInfoView"), success_status: 200,
        parameters: &[]
    },
    GET_CURRENT_PROFILE => {
        method: Get, path: "/api/v1/profile", operation_id: "getCurrentProfile", summary: "Get current profile",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: None, response_schema: Some("CurrentProfileView"), success_status: 200,
        parameters: &[]
    },
    UPDATE_CURRENT_PROFILE => {
        method: Patch, path: "/api/v1/profile", operation_id: "updateCurrentProfile", summary: "Update current profile",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: Some("UpdateCurrentProfileRequest"), response_schema: Some("CurrentProfileView"), success_status: 200,
        parameters: &[]
    },
    GET_PROFILE_PREFERENCES => {
        method: Get, path: "/api/v1/profile/preferences", operation_id: "getProfilePreferences", summary: "Get current profile preferences",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: None, response_schema: Some("UserPreferencesView"), success_status: 200,
        parameters: &[]
    },
    PATCH_PROFILE_PREFERENCES => {
        method: Patch, path: "/api/v1/profile/preferences", operation_id: "patchProfilePreferences", summary: "Update current profile preferences",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: Some("PatchUserPreferencesRequest"), response_schema: Some("UserPreferencesView"), success_status: 200,
        parameters: &[]
    },
    CHANGE_CURRENT_PASSWORD => {
        method: Post, path: "/api/v1/profile/change-password", operation_id: "changeCurrentPassword", summary: "Change current profile password",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: Some("ChangeCurrentPasswordRequest"), response_schema: None, success_status: 204,
        parameters: &[]
    },
    LIST_PROFILE_SESSIONS => {
        method: Get, path: "/api/v1/profile/sessions", operation_id: "listProfileSessions", summary: "List current profile sessions",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: None, response_schema: Some("UserSessionsView"), success_status: 200,
        parameters: &[]
    },
    REVOKE_PROFILE_SESSION => {
        method: Delete, path: "/api/v1/profile/sessions/{sessionId}", operation_id: "revokeProfileSession", summary: "Revoke a profile session",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: None, response_schema: None, success_status: 204,
        parameters: &[ParameterContract::path_uuid("sessionId")]
    },
    REVOKE_OTHER_PROFILE_SESSIONS => {
        method: Delete, path: "/api/v1/profile/sessions", operation_id: "revokeOtherProfileSessions", summary: "Revoke other profile sessions",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: None, response_schema: Some("RevokeOtherProfileSessionsView"), success_status: 200,
        parameters: &[]
    },
    LIST_ACTIVITIES => {
        method: Get, path: "/api/v1/activities", operation_id: "listActivities", summary: "List authorized activities",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ActivitiesView"), success_status: 200,
        parameters: ACTIVITY_FILTER_PARAMETERS
    },
    GET_ACTIVITY => {
        method: Get, path: "/api/v1/activities/{id}", operation_id: "getActivity", summary: "Get an authorized activity",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ActivityView"), success_status: 200,
        parameters: &[ParameterContract::path_uuid("id")]
    },
    LIST_USERS => {
        method: Get, path: "/api/v1/users", operation_id: "listUsers", summary: "Get all Users",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("UsersView"), success_status: 200,
        parameters: USER_FILTER_PARAMETERS
    },
    SEARCH_USERS => {
        method: Get, path: "/api/v1/users/search", operation_id: "searchUsers", summary: "Search Users for assignment",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("UserSearchItems"), success_status: 200,
        parameters: USER_SEARCH_PARAMETERS
    },
    GET_USER => {
        method: Get, path: "/api/v1/users/{id}", operation_id: "getUser", summary: "Get a User by ID",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("UserView"), success_status: 200,
        parameters: &[ParameterContract::path_uuid("id")]
    },
    LIST_SERVICE_ACCOUNTS => {
        method: Get, path: "/api/v1/serviceAccounts", operation_id: "listServiceAccounts", summary: "List Service Accounts",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ServiceAccountsResponse"), success_status: 200,
        parameters: SERVICE_ACCOUNT_FILTER_PARAMETERS
    },
    CREATE_SERVICE_ACCOUNT => {
        method: Post, path: "/api/v1/serviceAccounts", operation_id: "createServiceAccount", summary: "Create a Service Account",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("CreateServiceAccountRequest"), response_schema: Some("ServiceAccountView"), success_status: 200,
        parameters: &[]
    },
    ARCHIVE_SERVICE_ACCOUNTS => {
        method: Delete, path: "/api/v1/serviceAccounts", operation_id: "archiveServiceAccounts", summary: "Archive Service Accounts",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("ArchiveServiceAccountsRequest"), response_schema: None, success_status: 204,
        parameters: &[]
    },
    GET_SERVICE_ACCOUNT_LIMITS => {
        method: Get, path: "/api/v1/serviceAccounts/limits", operation_id: "getServiceAccountLimits", summary: "Get Service Account limits",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ServiceAccountLimitsView"), success_status: 200,
        parameters: &[]
    },
    GET_SERVICE_ACCOUNT => {
        method: Get, path: "/api/v1/serviceAccounts/{id}", operation_id: "getServiceAccount", summary: "Get a Service Account",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ServiceAccountDetailResponse"), success_status: 200,
        parameters: &[ParameterContract::path_uuid("id")]
    },
    UPDATE_SERVICE_ACCOUNT => {
        method: Patch, path: "/api/v1/serviceAccounts/{id}", operation_id: "updateServiceAccount", summary: "Update a Service Account",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("UpdateServiceAccountRequest"), response_schema: Some("ServiceAccountView"), success_status: 200,
        parameters: &[ParameterContract::path_uuid("id")]
    },
    RENAME_SERVICE_ACCOUNT => {
        method: Post, path: "/api/v1/serviceAccounts/rename", operation_id: "renameServiceAccount", summary: "Rename a Service Account",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("RenameServiceAccountRequest"), response_schema: Some("ServiceAccountView"), success_status: 200,
        parameters: &[]
    },
    ADD_SERVICE_ACCOUNT_ROLE => {
        method: Post, path: "/api/v1/serviceAccounts/{id}/roles", operation_id: "addServiceAccountRole", summary: "Assign a Role to a Service Account",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("AddServiceAccountRoleRequest"), response_schema: Some("ServiceAccountView"), success_status: 200,
        parameters: &[ParameterContract::path_uuid("id")]
    },
    REMOVE_SERVICE_ACCOUNT_ROLE => {
        method: Delete, path: "/api/v1/serviceAccounts/{id}/roles/{roleId}", operation_id: "removeServiceAccountRole", summary: "Remove a Role from a Service Account",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("ServiceAccountView"), success_status: 200,
        parameters: &[ParameterContract::path_uuid("id"), ParameterContract::path_uuid("roleId")]
    },
    ADD_SERVICE_ACCOUNT_RESOURCE_ACCESS => {
        method: Post, path: "/api/v1/serviceAccounts/{id}/resource-accesses", operation_id: "addServiceAccountResourceAccess", summary: "Add a resource override to a Service Account",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("AddServiceAccountResourceAccessRequest"), response_schema: Some("ServiceAccountView"), success_status: 200,
        parameters: &[ParameterContract::path_uuid("id")]
    },
    REMOVE_SERVICE_ACCOUNT_RESOURCE_ACCESS => {
        method: Delete, path: "/api/v1/serviceAccounts/{id}/resource-accesses/{resourceAccessId}", operation_id: "removeServiceAccountResourceAccess", summary: "Remove a resource override from a Service Account",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("ServiceAccountView"), success_status: 200,
        parameters: &[ParameterContract::path_uuid("id"), ParameterContract::path_uuid("resourceAccessId")]
    },
    LIST_SERVICE_ACCOUNT_TOKENS => {
        method: Get, path: "/api/v1/serviceAccounts/{id}/tokens", operation_id: "listServiceAccountTokens", summary: "List Service Account tokens",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ServiceAccountTokensResponse"), success_status: 200,
        parameters: TOKEN_FILTER_PARAMETERS
    },
    CREATE_SERVICE_ACCOUNT_TOKEN => {
        method: Post, path: "/api/v1/serviceAccounts/{id}/tokens", operation_id: "createServiceAccountToken", summary: "Create a Service Account token",
        public: true, setup_exempt: false, authentication: Human,
        request_schema: Some("CreateServiceAccountTokenRequest"), response_schema: Some("CreatedServiceAccountTokenView"), success_status: 201,
        parameters: &[ParameterContract::path_uuid("id")]
    },
    REVOKE_SERVICE_ACCOUNT_TOKEN => {
        method: Delete, path: "/api/v1/serviceAccounts/{id}/tokens/{tokenId}", operation_id: "revokeServiceAccountToken", summary: "Revoke a Service Account token",
        public: true, setup_exempt: false, authentication: Human,
        request_schema: None, response_schema: None, success_status: 204,
        parameters: &[ParameterContract::path_uuid("id"), ParameterContract::path_uuid("tokenId")]
    }
}

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
    fn declared_path_parameters_match_route_templates() {
        for route in ROUTES {
            let template_names = route
                .path
                .split('{')
                .skip(1)
                .filter_map(|remainder| remainder.split_once('}').map(|(name, _)| name))
                .collect::<Vec<_>>();
            let declared_names = route
                .parameters
                .iter()
                .filter(|parameter| parameter.location == ParameterLocation::Path)
                .map(|parameter| parameter.name)
                .collect::<Vec<_>>();
            assert_eq!(template_names, declared_names, "{}", route.operation_id);
            assert!(
                route
                    .parameters
                    .iter()
                    .filter(|parameter| parameter.location == ParameterLocation::Path)
                    .all(|parameter| parameter.required),
                "{} has an optional path parameter",
                route.operation_id
            );
        }
    }

    #[test]
    fn parameter_names_are_unique_per_location() {
        for route in ROUTES {
            let mut parameters = HashSet::new();
            assert!(
                route
                    .parameters
                    .iter()
                    .all(|parameter| parameters.insert((parameter.location, parameter.name))),
                "{} has duplicate parameters",
                route.operation_id
            );
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
