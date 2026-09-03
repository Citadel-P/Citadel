#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

impl HttpMethod {
    #[must_use]
    pub const fn as_openapi_str(self) -> &'static str {
        match self {
            Self::Get => "get",
            Self::Post => "post",
            Self::Put => "put",
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
pub enum ErrorResponse {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    TooManyRequests,
    InternalServerError,
    ServiceUnavailable,
}

impl ErrorResponse {
    #[must_use]
    pub const fn status(self) -> u16 {
        match self {
            Self::BadRequest => 400,
            Self::Unauthorized => 401,
            Self::Forbidden => 403,
            Self::NotFound => 404,
            Self::Conflict => 409,
            Self::TooManyRequests => 429,
            Self::InternalServerError => 500,
            Self::ServiceUnavailable => 503,
        }
    }

    #[must_use]
    pub const fn description(self) -> &'static str {
        match self {
            Self::BadRequest => "Bad Request",
            Self::Unauthorized => "Unauthorized",
            Self::Forbidden => "Forbidden",
            Self::NotFound => "Not Found",
            Self::Conflict => "Conflict",
            Self::TooManyRequests => "Too Many Requests",
            Self::InternalServerError => "Internal Server Error",
            Self::ServiceUnavailable => "Service Unavailable",
        }
    }
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
    ArrayString,
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
    pub const fn path_string(name: &'static str) -> Self {
        Self {
            name,
            location: ParameterLocation::Path,
            required: true,
            schema: ParameterSchema::String,
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
    pub error_responses: &'static [ErrorResponse],
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

const NETWORK_FILTER_PARAMETERS: &[ParameterContract] = &[
    ParameterContract::path_uuid("platformId"),
    ParameterContract::query("Dangling", ParameterSchema::Boolean { default: None }),
    ParameterContract::query("Driver", ParameterSchema::String),
    ParameterContract::query("Id", ParameterSchema::String),
    ParameterContract::query("Name", ParameterSchema::String),
];

const VOLUME_FILTER_PARAMETERS: &[ParameterContract] = &[
    ParameterContract::path_uuid("platformId"),
    ParameterContract::query("Dangling", ParameterSchema::Boolean { default: None }),
    ParameterContract::query("Driver", ParameterSchema::String),
    ParameterContract::query("Name", ParameterSchema::String),
];

const SWARM_TASK_FILTER_PARAMETERS: &[ParameterContract] = &[
    ParameterContract::path_uuid("platformId"),
    ParameterContract::query(
        "limit",
        ParameterSchema::Integer {
            format: IntegerFormat::Int32,
            minimum: Some(1),
            maximum: Some(200),
            default: Some(50),
        },
    ),
    ParameterContract::query("serviceId", ParameterSchema::String),
];

const REGISTRY_FILTER_PARAMETERS: &[ParameterContract] = &[
    ParameterContract::query(
        "includeDisabled",
        ParameterSchema::Boolean {
            default: Some(false),
        },
    ),
    ParameterContract::query("tags", ParameterSchema::ArrayString),
];

const GIT_REPOSITORY_FILTER_PARAMETERS: &[ParameterContract] = &[ParameterContract::query(
    "tags",
    ParameterSchema::ArrayString,
)];

const SECRET_DEFINITION_FILTER_PARAMETERS: &[ParameterContract] = &[
    ParameterContract::query("scope", ParameterSchema::Reference("ResourceBindingScope")),
    ParameterContract::query("resourceId", ParameterSchema::Uuid),
    ParameterContract::query(
        "targetResourceType",
        ParameterSchema::Reference("ResourceType"),
    ),
    ParameterContract::query("targetResourceId", ParameterSchema::Uuid),
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

const TEAM_FILTER_PARAMETERS: &[ParameterContract] = USER_FILTER_PARAMETERS;
const TEAM_SEARCH_PARAMETERS: &[ParameterContract] = USER_SEARCH_PARAMETERS;

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

const OIDC_LOGIN_PARAMETERS: &[ParameterContract] = &[
    ParameterContract::path_uuid("id"),
    ParameterContract::query("returnUrl", ParameterSchema::String),
];

const OIDC_CALLBACK_PARAMETERS: &[ParameterContract] = &[
    ParameterContract::path_uuid("id"),
    ParameterContract::query("code", ParameterSchema::String),
    ParameterContract::query("state", ParameterSchema::String),
    ParameterContract::query("error", ParameterSchema::String),
    ParameterContract::query("error_description", ParameterSchema::String),
];

const DEPLOYMENT_FILTER_PARAMETERS: &[ParameterContract] = &[
    ParameterContract::query("tags", ParameterSchema::ArrayString),
    ParameterContract::query("platformId", ParameterSchema::Uuid),
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
            error_responses: [$($error_response:ident),* $(,)?],
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
                    error_responses: &[$(ErrorResponse::$error_response),*],
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
        error_responses: [BadRequest, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    GET_READINESS => {
        method: Get, path: "/ready", operation_id: "getReadiness", summary: "Dependency readiness",
        public: false, setup_exempt: true, authentication: Anonymous,
        request_schema: None, response_schema: Some("ReadinessResponse"), success_status: 200,
        error_responses: [BadRequest, TooManyRequests, InternalServerError, ServiceUnavailable],
        parameters: &[]
    },
    GET_METRICS => {
        method: Get, path: "/metrics", operation_id: "getMetrics", summary: "OpenMetrics diagnostics",
        public: false, setup_exempt: true, authentication: Anonymous,
        request_schema: None, response_schema: None, success_status: 200,
        error_responses: [BadRequest, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    GET_SETUP_STATUS => {
        method: Get, path: "/api/v1/setup/status", operation_id: "getSetupStatus", summary: "Get setup status",
        public: false, setup_exempt: true, authentication: Anonymous,
        request_schema: None, response_schema: Some("SetupStatusView"), success_status: 200,
        error_responses: [BadRequest, TooManyRequests, InternalServerError, ServiceUnavailable],
        parameters: &[]
    },
    INITIALIZE_CITADEL => {
        method: Post, path: "/api/v1/setup/initialize", operation_id: "initializeCitadel", summary: "Initialize Citadel",
        public: false, setup_exempt: true, authentication: Anonymous,
        request_schema: Some("InitializeCitadelInput"), response_schema: Some("LoginResponse"), success_status: 200,
        error_responses: [BadRequest, Conflict, TooManyRequests, InternalServerError, ServiceUnavailable],
        parameters: &[]
    },
    LOGIN => {
        method: Post, path: "/api/v1/authentication/login", operation_id: "login", summary: "Sign in",
        public: false, setup_exempt: false, authentication: Anonymous,
        request_schema: Some("LoginRequest"), response_schema: Some("LoginResponse"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, NotFound, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    REFRESH_TOKEN => {
        method: Get, path: "/api/v1/authentication/refresh", operation_id: "refreshToken", summary: "Refresh browser access",
        public: false, setup_exempt: false, authentication: Anonymous,
        request_schema: None, response_schema: Some("RefreshTokenResponse"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, NotFound, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    LOGOUT => {
        method: Post, path: "/api/v1/authentication/logout", operation_id: "logout", summary: "End browser session",
        public: false, setup_exempt: false, authentication: Anonymous,
        request_schema: None, response_schema: None, success_status: 204,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    VERIFY_AUTHENTICATION_MFA => {
        method: Post, path: "/api/v1/authentication/mfa/verify", operation_id: "verifyAuthenticationMfa", summary: "Complete an MFA challenge",
        public: false, setup_exempt: false, authentication: Anonymous,
        request_schema: Some("MfaVerificationInput"), response_schema: Some("MfaVerificationView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    GET_AUTHENTICATION_MFA_SETUP => {
        method: Get, path: "/api/v1/authentication/mfa/setup", operation_id: "getAuthenticationMfaSetup", summary: "Get mandatory MFA setup",
        public: false, setup_exempt: false, authentication: Anonymous,
        request_schema: None, response_schema: Some("MandatoryMfaSetupView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    CONFIRM_AUTHENTICATION_MFA_SETUP => {
        method: Post, path: "/api/v1/authentication/mfa/setup/confirm", operation_id: "confirmAuthenticationMfaSetup", summary: "Complete mandatory MFA setup",
        public: false, setup_exempt: false, authentication: Anonymous,
        request_schema: Some("ConfirmMandatoryMfaSetupInput"), response_schema: Some("MandatoryMfaSetupCompleteView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    GET_PROFILE_MFA_STATUS => {
        method: Get, path: "/api/v1/profile/mfa", operation_id: "getProfileMfaStatus", summary: "Get MFA status",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: None, response_schema: Some("ProfileMfaStatusView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    START_PROFILE_MFA_SETUP => {
        method: Post, path: "/api/v1/profile/mfa/setup", operation_id: "startProfileMfaSetup", summary: "Start MFA setup",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: Some("StartProfileMfaSetupInput"), response_schema: Some("ProfileMfaSetupView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    CONFIRM_PROFILE_MFA_SETUP => {
        method: Post, path: "/api/v1/profile/mfa/setup/confirm", operation_id: "confirmProfileMfaSetup", summary: "Complete MFA setup",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: Some("ConfirmProfileMfaSetupInput"), response_schema: Some("ProfileMfaRecoveryCodesView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    DISABLE_PROFILE_MFA => {
        method: Post, path: "/api/v1/profile/mfa/disable", operation_id: "disableProfileMfa", summary: "Disable MFA",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: Some("DisableProfileMfaInput"), response_schema: None, success_status: 204,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    REGENERATE_PROFILE_MFA_RECOVERY_CODES => {
        method: Post, path: "/api/v1/profile/mfa/recovery-codes", operation_id: "regenerateProfileMfaRecoveryCodes", summary: "Regenerate MFA recovery codes",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: Some("RegenerateProfileMfaRecoveryCodesInput"), response_schema: Some("ProfileMfaRecoveryCodesView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    RESET_USER_MFA => {
        method: Delete, path: "/api/v1/users/{id}/mfa", operation_id: "resetUserMfa", summary: "Reset a user's MFA",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: None, success_status: 204,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    LIST_OIDC_LOGIN_PROVIDERS => {
        method: Get, path: "/api/v1/authentication/oidc/providers", operation_id: "listOidcLoginProviders", summary: "List enabled OIDC login providers",
        public: false, setup_exempt: false, authentication: Anonymous,
        request_schema: None, response_schema: Some("OidcLoginProvidersView"), success_status: 200,
        error_responses: [BadRequest, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    BEGIN_OIDC_LOGIN => {
        method: Get, path: "/api/v1/authentication/oidc/{id}/login", operation_id: "beginOidcLogin", summary: "Begin OIDC login",
        public: false, setup_exempt: false, authentication: Anonymous,
        request_schema: None, response_schema: None, success_status: 302,
        error_responses: [BadRequest, NotFound, TooManyRequests, InternalServerError],
        parameters: OIDC_LOGIN_PARAMETERS
    },
    COMPLETE_OIDC_LOGIN => {
        method: Get, path: "/api/v1/authentication/oidc/{id}/callback", operation_id: "completeOidcLogin", summary: "Complete OIDC login",
        public: false, setup_exempt: false, authentication: Anonymous,
        request_schema: None, response_schema: None, success_status: 302,
        error_responses: [BadRequest, NotFound, TooManyRequests, InternalServerError],
        parameters: OIDC_CALLBACK_PARAMETERS
    },
    GET_PERMISSION_MATRIX => {
        method: Get, path: "/api/v1/roles/permissions/matrix", operation_id: "getPermissionMatrix", summary: "Get permission matrix",
        public: true, setup_exempt: false, authentication: Anonymous,
        request_schema: None, response_schema: Some("PermissionMatrixResponse"), success_status: 200,
        error_responses: [BadRequest, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    GET_LICENSE_ENTITLEMENTS => {
        method: Get, path: "/api/v1/license/entitlements", operation_id: "getLicenseEntitlements", summary: "Get effective license entitlements",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("LicenseEntitlementsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    GET_LICENSE => {
        method: Get, path: "/api/v1/license", operation_id: "getLicense", summary: "Get installed license state",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("LicenseView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    INSTALL_LICENSE => {
        method: Post, path: "/api/v1/license", operation_id: "installLicense", summary: "Install or replace a license",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("InstallLicenseInput"), response_schema: Some("LicenseView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    REMOVE_LICENSE => {
        method: Delete, path: "/api/v1/license", operation_id: "removeLicense", summary: "Remove the installed license",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("LicenseView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    GET_LICENSE_REQUEST => {
        method: Get, path: "/api/v1/license/request", operation_id: "getLicenseRequest", summary: "Get license request metadata",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("LicenseRequestView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    GET_APPLICATION_INFO => {
        method: Get, path: "/api/v1/application/info", operation_id: "getApplicationInfo", summary: "Get application information",
        public: false, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ApplicationInfoView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    GET_CURRENT_PROFILE => {
        method: Get, path: "/api/v1/profile", operation_id: "getCurrentProfile", summary: "Get current profile",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: None, response_schema: Some("CurrentProfileView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    UPDATE_CURRENT_PROFILE => {
        method: Patch, path: "/api/v1/profile", operation_id: "updateCurrentProfile", summary: "Update current profile",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: Some("UpdateCurrentProfileInput"), response_schema: Some("CurrentProfileView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    GET_PROFILE_PREFERENCES => {
        method: Get, path: "/api/v1/profile/preferences", operation_id: "getProfilePreferences", summary: "Get current profile preferences",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: None, response_schema: Some("UserPreferencesView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    PATCH_PROFILE_PREFERENCES => {
        method: Patch, path: "/api/v1/profile/preferences", operation_id: "patchProfilePreferences", summary: "Update current profile preferences",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: Some("PatchUserPreferencesInput"), response_schema: Some("UserPreferencesView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    CHANGE_CURRENT_PASSWORD => {
        method: Post, path: "/api/v1/profile/change-password", operation_id: "changeCurrentPassword", summary: "Change current profile password",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: Some("ChangeCurrentPasswordInput"), response_schema: None, success_status: 204,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    LIST_PROFILE_SESSIONS => {
        method: Get, path: "/api/v1/profile/sessions", operation_id: "listProfileSessions", summary: "List current profile sessions",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: None, response_schema: Some("UserSessionsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    REVOKE_PROFILE_SESSION => {
        method: Delete, path: "/api/v1/profile/sessions/{sessionId}", operation_id: "revokeProfileSession", summary: "Revoke a profile session",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: None, response_schema: None, success_status: 204,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("sessionId")]
    },
    REVOKE_OTHER_PROFILE_SESSIONS => {
        method: Delete, path: "/api/v1/profile/sessions", operation_id: "revokeOtherProfileSessions", summary: "Revoke other profile sessions",
        public: false, setup_exempt: false, authentication: Human,
        request_schema: None, response_schema: Some("RevokeOtherProfileSessionsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    LIST_ACTIVITIES => {
        method: Get, path: "/api/v1/activities", operation_id: "listActivities", summary: "List authorized activities",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ActivitiesView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: ACTIVITY_FILTER_PARAMETERS
    },
    GET_ACTIVITY => {
        method: Get, path: "/api/v1/activities/{id}", operation_id: "getActivity", summary: "Get an authorized activity",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ActivityView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    LIST_USERS => {
        method: Get, path: "/api/v1/users", operation_id: "listUsers", summary: "Get all Users",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("UsersView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: USER_FILTER_PARAMETERS
    },
    SEARCH_USERS => {
        method: Get, path: "/api/v1/users/search", operation_id: "searchUsers", summary: "Search Users for assignment",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("UserSearchItems"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: USER_SEARCH_PARAMETERS
    },
    GET_USER => {
        method: Get, path: "/api/v1/users/{id}", operation_id: "getUser", summary: "Get a User by ID",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("UserView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    CREATE_USER => {
        method: Post, path: "/api/v1/users", operation_id: "createUser", summary: "Create a User",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: Some("CreateUserInput"), response_schema: Some("UserView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, Conflict, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    UPDATE_USER => {
        method: Patch, path: "/api/v1/users/{id}", operation_id: "updateUser", summary: "Update a User",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: Some("PatchUserInput"), response_schema: Some("UserView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    RENAME_USER => {
        method: Post, path: "/api/v1/users/rename", operation_id: "renameUser", summary: "Rename a User",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: Some("RenameResource"), response_schema: Some("UserView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    ADD_USER_ROLE => {
        method: Post, path: "/api/v1/users/{id}/roles", operation_id: "addUserRole", summary: "Assign a Role to a User",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: Some("AddUserRoleInput"), response_schema: Some("UserView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    REMOVE_USER_ROLE => {
        method: Delete, path: "/api/v1/users/{id}/roles/{roleId}", operation_id: "removeUserRole", summary: "Remove a Role from a User",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("UserView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id"), ParameterContract::path_uuid("roleId")]
    },
    ADD_USER_RESOURCE_ACCESS => {
        method: Post, path: "/api/v1/users/{id}/resource-accesses", operation_id: "addUserResourceAccess", summary: "Add a resource override to a User",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: Some("AddUserResourceAccessInput"), response_schema: Some("UserView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    REMOVE_USER_RESOURCE_ACCESS => {
        method: Delete, path: "/api/v1/users/{id}/resource-accesses", operation_id: "removeUserResourceAccess", summary: "Remove a resource override from a User",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: Some("RemoveUserResourceAccessInput"), response_schema: Some("UserView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    DELETE_USERS => {
        method: Delete, path: "/api/v1/users", operation_id: "deleteUsers", summary: "Delete Users",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: Some("DeleteUsersInput"), response_schema: None, success_status: 204,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    LIST_TEAMS => {
        method: Get, path: "/api/v1/teams", operation_id: "listTeams", summary: "Get all Teams",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("TeamsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: TEAM_FILTER_PARAMETERS
    },
    SEARCH_TEAMS => {
        method: Get, path: "/api/v1/teams/search", operation_id: "searchTeams", summary: "Search Teams for assignment",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("TeamSearchItems"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: TEAM_SEARCH_PARAMETERS
    },
    GET_TEAM => {
        method: Get, path: "/api/v1/teams/{id}", operation_id: "getTeam", summary: "Get a Team by ID",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("TeamView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    CREATE_TEAM => {
        method: Post, path: "/api/v1/teams", operation_id: "createTeam", summary: "Create a Team",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: Some("CreateTeamInput"), response_schema: Some("TeamView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, Conflict, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    PATCH_TEAM => {
        method: Patch, path: "/api/v1/teams/{id}", operation_id: "updateTeam", summary: "Update a Team",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: Some("PatchTeamInput"), response_schema: Some("TeamView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    RENAME_TEAM => {
        method: Post, path: "/api/v1/teams/rename", operation_id: "renameTeam", summary: "Rename a Team",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: Some("RenameResource"), response_schema: Some("TeamView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    ADD_TEAM_ROLE => {
        method: Post, path: "/api/v1/teams/{id}/roles", operation_id: "addTeamRole", summary: "Assign a Role to a Team",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: Some("AddTeamRoleInput"), response_schema: Some("TeamView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    REMOVE_TEAM_ROLE => {
        method: Delete, path: "/api/v1/teams/{id}/roles/{roleId}", operation_id: "removeTeamRole", summary: "Remove a Role from a Team",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("TeamView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id"), ParameterContract::path_uuid("roleId")]
    },
    ADD_TEAM_MEMBER => {
        method: Post, path: "/api/v1/teams/{id}/members", operation_id: "addTeamMember", summary: "Add an Actor to a Team",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: Some("AddTeamMemberInput"), response_schema: Some("TeamView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    REMOVE_TEAM_MEMBER => {
        method: Delete, path: "/api/v1/teams/{id}/members/{memberActorId}", operation_id: "removeTeamMember", summary: "Remove an Actor from a Team",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("TeamView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id"), ParameterContract::path_uuid("memberActorId")]
    },
    ADD_TEAM_RESOURCE_ACCESS => {
        method: Post, path: "/api/v1/teams/{id}/resource-accesses", operation_id: "addTeamResourceAccess", summary: "Add a resource override to a Team",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: Some("AddTeamResourceAccessInput"), response_schema: Some("TeamView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    REMOVE_TEAM_RESOURCE_ACCESS => {
        method: Delete, path: "/api/v1/teams/{id}/resource-accesses", operation_id: "removeTeamResourceAccess", summary: "Remove a resource override from a Team",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: Some("RemoveTeamResourceAccessInput"), response_schema: Some("TeamView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    DELETE_TEAMS => {
        method: Delete, path: "/api/v1/teams", operation_id: "deleteTeams", summary: "Delete Teams",
        public: false, setup_exempt: false, authentication: Administrator,
        request_schema: Some("DeleteTeamsInput"), response_schema: None, success_status: 204,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    LIST_ROLES => {
        method: Get, path: "/api/v1/roles", operation_id: "listRoles", summary: "Get all Roles",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("RolesView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    GET_ROLE => {
        method: Get, path: "/api/v1/roles/{id}", operation_id: "getRole", summary: "Get a Role by ID",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("RoleView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    CREATE_ROLE => {
        method: Post, path: "/api/v1/roles", operation_id: "createRole", summary: "Create a Role",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("RoleInput"), response_schema: Some("RoleView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, Conflict, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    PATCH_ROLE_PERMISSIONS => {
        method: Patch, path: "/api/v1/roles/{id}/permissions", operation_id: "updateRolePermissions", summary: "Update Role permissions",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("PatchRolePermissionsInput"), response_schema: Some("RoleView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    RENAME_ROLE => {
        method: Post, path: "/api/v1/roles/rename", operation_id: "renameRole", summary: "Rename a Role",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("RenameResource"), response_schema: Some("RoleView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    DELETE_ROLES => {
        method: Delete, path: "/api/v1/roles", operation_id: "deleteRoles", summary: "Delete Roles",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("DeleteRolesInput"), response_schema: None, success_status: 204,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    LIST_OIDC_PROVIDERS => {
        method: Get, path: "/api/v1/oidcProviders", operation_id: "listOidcProviders", summary: "List OIDC providers",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("OidcProvidersView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    GET_OIDC_PROVIDER => {
        method: Get, path: "/api/v1/oidcProviders/{id}", operation_id: "getOidcProvider", summary: "Get OIDC provider",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("OidcProviderView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    CREATE_OIDC_PROVIDER => {
        method: Post, path: "/api/v1/oidcProviders", operation_id: "createOidcProvider", summary: "Create OIDC provider",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("OidcProviderInput"), response_schema: Some("OidcProviderView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, Conflict, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    RENAME_OIDC_PROVIDER => {
        method: Post, path: "/api/v1/oidcProviders/rename", operation_id: "renameOidcProvider", summary: "Rename OIDC provider",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("RenameResource"), response_schema: Some("OidcProviderView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    UPDATE_OIDC_PROVIDER => {
        method: Patch, path: "/api/v1/oidcProviders/{id}", operation_id: "updateOidcProvider", summary: "Update OIDC provider",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("UpdateOidcProviderInput"), response_schema: Some("OidcProviderView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    UPDATE_OIDC_PROVIDER_METADATA => {
        method: Patch, path: "/api/v1/oidcProviders/{id}/_metadata", operation_id: "updateOidcProviderMetadata", summary: "Update OIDC provider metadata",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("PatchResourceMetadata"), response_schema: Some("OidcProviderView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    DELETE_OIDC_PROVIDER => {
        method: Delete, path: "/api/v1/oidcProviders/{id}", operation_id: "deleteOidcProvider", summary: "Delete OIDC provider",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: None, success_status: 204,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    TEST_OIDC_PROVIDER_DISCOVERY => {
        method: Post, path: "/api/v1/oidcProviders/{id}/testDiscovery", operation_id: "testOidcProviderDiscovery", summary: "Test OIDC provider discovery",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("OidcDiscoveryResultView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    TEST_OIDC_DISCOVERY => {
        method: Post, path: "/api/v1/oidcProviders/testDiscovery", operation_id: "testOidcDiscovery", summary: "Test OIDC discovery",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("TestOidcProviderDiscoveryInput"), response_schema: Some("OidcDiscoveryResultView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    LIST_SERVICE_ACCOUNTS => {
        method: Get, path: "/api/v1/serviceAccounts", operation_id: "listServiceAccounts", summary: "List Service Accounts",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ServiceAccountsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: SERVICE_ACCOUNT_FILTER_PARAMETERS
    },
    CREATE_SERVICE_ACCOUNT => {
        method: Post, path: "/api/v1/serviceAccounts", operation_id: "createServiceAccount", summary: "Create a Service Account",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("CreateServiceAccountInput"), response_schema: Some("ServiceAccountView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    ARCHIVE_SERVICE_ACCOUNTS => {
        method: Delete, path: "/api/v1/serviceAccounts", operation_id: "archiveServiceAccounts", summary: "Archive Service Accounts",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("DeleteServiceAccountsInput"), response_schema: None, success_status: 204,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    GET_SERVICE_ACCOUNT_LIMITS => {
        method: Get, path: "/api/v1/serviceAccounts/limits", operation_id: "getServiceAccountLimits", summary: "Get Service Account limits",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ServiceAccountLimitsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    GET_SERVICE_ACCOUNT => {
        method: Get, path: "/api/v1/serviceAccounts/{id}", operation_id: "getServiceAccount", summary: "Get a Service Account",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ServiceAccountView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    UPDATE_SERVICE_ACCOUNT => {
        method: Patch, path: "/api/v1/serviceAccounts/{id}", operation_id: "updateServiceAccount", summary: "Update a Service Account",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("PatchServiceAccountInput"), response_schema: Some("ServiceAccountView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    RENAME_SERVICE_ACCOUNT => {
        method: Post, path: "/api/v1/serviceAccounts/rename", operation_id: "renameServiceAccount", summary: "Rename a Service Account",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("RenameResource"), response_schema: Some("ServiceAccountView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    ADD_SERVICE_ACCOUNT_ROLE => {
        method: Post, path: "/api/v1/serviceAccounts/{id}/roles", operation_id: "addServiceAccountRole", summary: "Assign a Role to a Service Account",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("AddServiceAccountRoleInput"), response_schema: Some("ServiceAccountView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    REMOVE_SERVICE_ACCOUNT_ROLE => {
        method: Delete, path: "/api/v1/serviceAccounts/{id}/roles/{roleId}", operation_id: "removeServiceAccountRole", summary: "Remove a Role from a Service Account",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("ServiceAccountView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id"), ParameterContract::path_uuid("roleId")]
    },
    ADD_SERVICE_ACCOUNT_RESOURCE_ACCESS => {
        method: Post, path: "/api/v1/serviceAccounts/{id}/resource-accesses", operation_id: "addServiceAccountResourceAccess", summary: "Add a resource override to a Service Account",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: Some("ServiceAccountResourceAccessInput"), response_schema: Some("ServiceAccountView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    REMOVE_SERVICE_ACCOUNT_RESOURCE_ACCESS => {
        method: Delete, path: "/api/v1/serviceAccounts/{id}/resource-accesses/{resourceAccessId}", operation_id: "removeServiceAccountResourceAccess", summary: "Remove a resource override from a Service Account",
        public: true, setup_exempt: false, authentication: Administrator,
        request_schema: None, response_schema: Some("ServiceAccountView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id"), ParameterContract::path_uuid("resourceAccessId")]
    },
    LIST_SERVICE_ACCOUNT_TOKENS => {
        method: Get, path: "/api/v1/serviceAccounts/{id}/tokens", operation_id: "listServiceAccountTokens", summary: "List Service Account tokens",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ServiceAccountTokensView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: TOKEN_FILTER_PARAMETERS
    },
    CREATE_SERVICE_ACCOUNT_TOKEN => {
        method: Post, path: "/api/v1/serviceAccounts/{id}/tokens", operation_id: "createServiceAccountToken", summary: "Create a Service Account token",
        public: true, setup_exempt: false, authentication: Human,
        request_schema: Some("CreateServiceAccountTokenInput"), response_schema: Some("CreatedServiceAccountTokenView"), success_status: 201,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    REVOKE_SERVICE_ACCOUNT_TOKEN => {
        method: Delete, path: "/api/v1/serviceAccounts/{id}/tokens/{tokenId}", operation_id: "revokeServiceAccountToken", summary: "Revoke a Service Account token",
        public: true, setup_exempt: false, authentication: Human,
        request_schema: None, response_schema: None, success_status: 204,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id"), ParameterContract::path_uuid("tokenId")]
    },
    LIST_DEPLOYMENTS => {
        method: Get, path: "/api/v1/deployments", operation_id: "listDeployments", summary: "List authorized Deployments",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("DeploymentsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: DEPLOYMENT_FILTER_PARAMETERS
    },
    CREATE_DEPLOYMENT => {
        method: Post, path: "/api/v1/deployments", operation_id: "createDeployment", summary: "Create a Deployment",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("CreateDeploymentInput"), response_schema: Some("DeploymentView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    DELETE_DEPLOYMENTS => {
        method: Delete, path: "/api/v1/deployments", operation_id: "deleteDeployments", summary: "Delete Deployments",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("DeploymentIds"), response_schema: None, success_status: 204,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError, ServiceUnavailable],
        parameters: &[]
    },
    RENAME_DEPLOYMENT => {
        method: Post, path: "/api/v1/deployments/rename", operation_id: "renameDeployment", summary: "Rename a Deployment",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("RenameResource"), response_schema: Some("DeploymentView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError],
        parameters: &[]
    },
    GET_DEPLOYMENT => {
        method: Get, path: "/api/v1/deployments/{deploymentId}", operation_id: "getDeployment", summary: "Get a Deployment",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("DeploymentView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("deploymentId")]
    },
    GET_DEPLOYMENT_CONFIG => {
        method: Get, path: "/api/v1/deployments/{deploymentId}/_cfg", operation_id: "getDeploymentConfig", summary: "Get Deployment configuration",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("DeploymentConfigView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("deploymentId")]
    },
    GET_DEPLOYMENT_DUPLICATE_DRAFT => {
        method: Get, path: "/api/v1/deployments/{deploymentId}/duplicate-draft", operation_id: "getDeploymentDuplicateDraft", summary: "Build a Deployment duplicate draft",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("DeploymentDuplicateDraftView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("deploymentId")]
    },
    UPDATE_DEPLOYMENT => {
        method: Patch, path: "/api/v1/deployments/{id}", operation_id: "updateDeployment", summary: "Update Deployment configuration",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("PatchDeploymentInput"), response_schema: Some("DeploymentView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    UPDATE_DEPLOYMENT_METADATA => {
        method: Patch, path: "/api/v1/deployments/{id}/_metadata", operation_id: "updateDeploymentMetadata", summary: "Update Deployment metadata",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("PatchResourceMetadata"), response_schema: Some("DeploymentView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    LIST_PLATFORMS => {
        method: Get, path: "/api/v1/platforms", operation_id: "listPlatforms", summary: "List authorized Platforms",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("PlatformsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::query("tags", ParameterSchema::ArrayString)]
    },
    GET_PLATFORM => {
        method: Get, path: "/api/v1/platforms/{id}", operation_id: "getPlatfom", summary: "Get a Platform",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("PlatformView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    UPDATE_PLATFORM_METADATA => {
        method: Patch, path: "/api/v1/platforms/{id}/_metadata", operation_id: "updatePlatformMetadata", summary: "Update Platform metadata",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("PatchResourceMetadata"), response_schema: Some("PlatformView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    LIST_PLATFORM_CONTAINERS => {
        method: Get, path: "/api/v1/platforms/{id}/containers", operation_id: "listContainers", summary: "List Platform Containers",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ContainersView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    GET_CONTAINER => {
        method: Get, path: "/api/v1/containers/{id}", operation_id: "getContainer", summary: "Get a Container",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ContainerView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("id")]
    },
    LIST_PLATFORM_IMAGES => {
        method: Get, path: "/api/v1/images/{platformId}", operation_id: "listImages", summary: "List Platform Images",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ImagesView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("platformId")]
    },
    LIST_PLATFORM_NETWORKS => {
        method: Get, path: "/api/v1/networks/{platformId}", operation_id: "listNetworks", summary: "List Platform Networks",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("NetworksView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, Conflict, TooManyRequests, InternalServerError],
        parameters: NETWORK_FILTER_PARAMETERS
    },
    GET_PLATFORM_NETWORK => {
        method: Get, path: "/api/v1/networks/{platformId}/{networkId}", operation_id: "inspectNetwork", summary: "Inspect a Platform Network",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("DockerNetworkDetailsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, Conflict, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("platformId"), ParameterContract::path_string("networkId"), ParameterContract::query("dockerNodeId", ParameterSchema::String)]
    },
    LIST_PLATFORM_VOLUMES => {
        method: Get, path: "/api/v1/volumes/{platformId}", operation_id: "listVolumes", summary: "List Platform Volumes",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("VolumesView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, Conflict, TooManyRequests, InternalServerError],
        parameters: VOLUME_FILTER_PARAMETERS
    },
    GET_PLATFORM_VOLUME => {
        method: Get, path: "/api/v1/volumes/{platformId}/{name}", operation_id: "inspectVolume", summary: "Inspect a Platform Volume",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("DockerVolumeResultView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, Conflict, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("platformId"), ParameterContract::path_string("name"), ParameterContract::query("dockerNodeId", ParameterSchema::String)]
    },
    LIST_SWARM_NODES => {
        method: Get, path: "/api/v1/platforms/{platformId}/swarm/nodes", operation_id: "listSwarmNodes", summary: "List Swarm Nodes",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("SwarmNodesView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("platformId")]
    },
    GET_SWARM_NODE => {
        method: Get, path: "/api/v1/platforms/{platformId}/swarm/nodes/{nodeId}", operation_id: "getSwarmNode", summary: "Get a Swarm Node",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("SwarmNodeView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("platformId"), ParameterContract::path_string("nodeId")]
    },
    LIST_SWARM_SERVICES => {
        method: Get, path: "/api/v1/platforms/{platformId}/swarm/services", operation_id: "listSwarmServices", summary: "List Swarm Services",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("SwarmServicesView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("platformId")]
    },
    GET_SWARM_SERVICE => {
        method: Get, path: "/api/v1/platforms/{platformId}/swarm/services/{resourceId}", operation_id: "getSwarmService", summary: "Get a Swarm Service",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("SwarmServiceView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("platformId"), ParameterContract::path_string("resourceId")]
    },
    LIST_SWARM_TASKS => {
        method: Get, path: "/api/v1/platforms/{platformId}/swarm/tasks", operation_id: "listSwarmTasks", summary: "List Swarm Tasks",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("SwarmTasksView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: SWARM_TASK_FILTER_PARAMETERS
    },
    GET_SWARM_TASK => {
        method: Get, path: "/api/v1/platforms/{platformId}/swarm/tasks/{resourceId}", operation_id: "getSwarmTask", summary: "Get a Swarm Task",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("SwarmTaskView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("platformId"), ParameterContract::path_string("resourceId")]
    },
    LIST_SWARM_NETWORKS => {
        method: Get, path: "/api/v1/platforms/{platformId}/swarm/networks", operation_id: "listSwarmNetworks", summary: "List Swarm Networks",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("SwarmNetworksView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("platformId")]
    },
    GET_SWARM_NETWORK => {
        method: Get, path: "/api/v1/platforms/{platformId}/swarm/networks/{resourceId}", operation_id: "getSwarmNetwork", summary: "Get a Swarm Network",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("SwarmNetworkView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("platformId"), ParameterContract::path_string("resourceId")]
    },
    LIST_SWARM_CONFIGS => {
        method: Get, path: "/api/v1/platforms/{platformId}/swarm/configs", operation_id: "listSwarmConfigs", summary: "List Swarm Configs",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("SwarmConfigsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("platformId")]
    },
    GET_SWARM_CONFIG => {
        method: Get, path: "/api/v1/platforms/{platformId}/swarm/configs/{resourceId}", operation_id: "getSwarmConfig", summary: "Get a Swarm Config",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("SwarmConfigView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("platformId"), ParameterContract::path_string("resourceId")]
    },
    LIST_SWARM_SECRETS => {
        method: Get, path: "/api/v1/platforms/{platformId}/swarm/secrets", operation_id: "listSwarmSecrets", summary: "List Swarm Secrets",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("SwarmSecretsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("platformId")]
    },
    GET_SWARM_SECRET => {
        method: Get, path: "/api/v1/platforms/{platformId}/swarm/secrets/{resourceId}", operation_id: "getSwarmSecret", summary: "Get a Swarm Secret",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("SwarmSecretView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError],
        parameters: &[ParameterContract::path_uuid("platformId"), ParameterContract::path_string("resourceId")]
    },
    LIST_TAGS => {
        method: Get, path: "/api/v1/tags", operation_id: "listTags", summary: "List resource tags",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("TagsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError], parameters: &[]
    },
    CREATE_TAG => {
        method: Post, path: "/api/v1/tags", operation_id: "createTag", summary: "Create a resource tag",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("CreateTagInput"), response_schema: Some("TagView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[]
    },
    PATCH_TAG => {
        method: Patch, path: "/api/v1/tags/{id}", operation_id: "patchTag", summary: "Update a resource tag",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("PatchTagInput"), response_schema: Some("TagView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_uuid("id")]
    },
    DELETE_TAG => {
        method: Delete, path: "/api/v1/tags/{id}", operation_id: "deleteTag", summary: "Delete a resource tag",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: None, success_status: 204,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_uuid("id")]
    },
    GET_PLATFORM_TAGS => {
        method: Get, path: "/api/v1/platforms/{id}/tags", operation_id: "getPlatformTags", summary: "Get Platform tags",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ResourceTagsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_uuid("id")]
    },
    REPLACE_PLATFORM_TAGS => {
        method: Put, path: "/api/v1/platforms/{id}/tags", operation_id: "replacePlatformTags", summary: "Replace Platform tags",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("ReplaceResourceTagsInput"), response_schema: Some("ResourceTagsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_uuid("id")]
    },
    LIST_REGISTRIES => {
        method: Get, path: "/api/v1/registries", operation_id: "listRegistries", summary: "List Registries",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("RegistriesView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError], parameters: REGISTRY_FILTER_PARAMETERS
    },
    CREATE_REGISTRY => {
        method: Post, path: "/api/v1/registries", operation_id: "createRegistry", summary: "Create a Registry",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("CreateRegistryInput"), response_schema: Some("RegistryView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[]
    },
    DELETE_REGISTRIES => {
        method: Delete, path: "/api/v1/registries", operation_id: "deleteRegistries", summary: "Delete Registries",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("DeleteRegistriesInput"), response_schema: None, success_status: 204,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[]
    },
    GET_REGISTRY => {
        method: Get, path: "/api/v1/registries/{id}", operation_id: "getRegistry", summary: "Get a Registry",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("RegistryView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_uuid("id")]
    },
    GET_REGISTRY_CONFIG => {
        method: Get, path: "/api/v1/registries/{id}/_cfg", operation_id: "getRegistryConfig", summary: "Get Registry configuration",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("RegistryConfigView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_uuid("id")]
    },
    UPDATE_REGISTRY => {
        method: Patch, path: "/api/v1/registries/{id}", operation_id: "updateRegistry", summary: "Update a Registry",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("PatchRegistryInput"), response_schema: Some("RegistryView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_uuid("id")]
    },
    UPDATE_REGISTRY_METADATA => {
        method: Patch, path: "/api/v1/registries/{id}/_metadata", operation_id: "updateRegistryMetadata", summary: "Update Registry metadata",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("PatchResourceMetadata"), response_schema: Some("RegistryView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_uuid("id")]
    },
    RENAME_REGISTRY => {
        method: Post, path: "/api/v1/registries/rename", operation_id: "renameRegistry", summary: "Rename a Registry",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("RenameResource"), response_schema: Some("RegistryView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[]
    },
    GET_REGISTRY_TAGS => {
        method: Get, path: "/api/v1/registries/{id}/tags", operation_id: "getRegistryTags", summary: "Get Registry tags",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ResourceTagsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_uuid("id")]
    },
    REPLACE_REGISTRY_TAGS => {
        method: Put, path: "/api/v1/registries/{id}/tags", operation_id: "replaceRegistryTags", summary: "Replace Registry tags",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("ReplaceResourceTagsInput"), response_schema: Some("ResourceTagsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_uuid("id")]
    },
    LIST_GIT_REPOSITORIES => {
        method: Get, path: "/api/v1/gitRepositories", operation_id: "listGitRepositories", summary: "List Git repositories",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("GitRepositoriesView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError], parameters: GIT_REPOSITORY_FILTER_PARAMETERS
    },
    CREATE_GIT_REPOSITORY => {
        method: Post, path: "/api/v1/gitRepositories", operation_id: "createGitRepository", summary: "Create a Git repository",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("CreateGitRepositoryInput"), response_schema: Some("GitRepositoryView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[]
    },
    DELETE_GIT_REPOSITORIES => {
        method: Delete, path: "/api/v1/gitRepositories", operation_id: "deleteGitRepositories", summary: "Delete Git repositories",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("DeleteGitRepositoriesInput"), response_schema: None, success_status: 204,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[]
    },
    GET_GIT_REPOSITORY => {
        method: Get, path: "/api/v1/gitRepositories/{id}", operation_id: "getGitRepository", summary: "Get a Git repository",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("GitRepositoryView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_uuid("id")]
    },
    GET_GIT_REPOSITORY_CONFIG => {
        method: Get, path: "/api/v1/gitRepositories/{id}/_cfg", operation_id: "getGitRepositoryConfig", summary: "Get Git repository configuration",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("GitRepositoryConfigView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_uuid("id")]
    },
    UPDATE_GIT_REPOSITORY => {
        method: Patch, path: "/api/v1/gitRepositories/{id}", operation_id: "updateGitRepository", summary: "Update a Git repository",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("PatchGitRepositoryInput"), response_schema: Some("GitRepositoryView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_uuid("id")]
    },
    UPDATE_GIT_REPOSITORY_METADATA => {
        method: Patch, path: "/api/v1/gitRepositories/{id}/_metadata", operation_id: "updateGitRepositoryMetadata", summary: "Update Git repository metadata",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("PatchResourceMetadata"), response_schema: Some("GitRepositoryView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_uuid("id")]
    },
    RENAME_GIT_REPOSITORY => {
        method: Post, path: "/api/v1/gitRepositories/rename", operation_id: "renameGitRepository", summary: "Rename a Git repository",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("RenameResource"), response_schema: Some("GitRepositoryView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[]
    },
    GET_GIT_REPOSITORY_TAGS => {
        method: Get, path: "/api/v1/gitRepositories/{id}/tags", operation_id: "getGitRepositoryTags", summary: "Get Git repository tags",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ResourceTagsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_uuid("id")]
    },
    REPLACE_GIT_REPOSITORY_TAGS => {
        method: Put, path: "/api/v1/gitRepositories/{id}/tags", operation_id: "replaceGitRepositoryTags", summary: "Replace Git repository tags",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("ReplaceResourceTagsInput"), response_schema: Some("ResourceTagsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_uuid("id")]
    },
    CREATE_NETWORK => {
        method: Post, path: "/api/v1/networks", operation_id: "createNetwork", summary: "Create a Network",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("CreateNetworkInput"), response_schema: Some("CreateNetworkView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[]
    },
    DELETE_NETWORKS => {
        method: Delete, path: "/api/v1/networks", operation_id: "deleteNetworks", summary: "Delete Networks",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("DeleteNetworksInput"), response_schema: None, success_status: 204,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[]
    },
    CREATE_VOLUME => {
        method: Post, path: "/api/v1/volumes", operation_id: "createVolume", summary: "Create a Volume",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("CreateVolumeInput"), response_schema: Some("DockerVolumeResultView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[]
    },
    DELETE_VOLUMES => {
        method: Delete, path: "/api/v1/volumes", operation_id: "deleteVolumes", summary: "Delete Volumes",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("DeleteVolumesInput"), response_schema: None, success_status: 204,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[]
    },
    GET_GLOBAL_RESOURCE_BINDINGS => {
        method: Get, path: "/api/v1/resourceBindings/global", operation_id: "getGlobalResourceBindings", summary: "Get global resource bindings",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ResourceBindingsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError], parameters: &[]
    },
    CREATE_GLOBAL_RESOURCE_BINDING => {
        method: Post, path: "/api/v1/resourceBindings/global", operation_id: "createGlobalResourceBinding", summary: "Create a global resource binding",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("ResourceBindingInput"), response_schema: Some("ResourceBindingsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, Conflict, TooManyRequests, InternalServerError], parameters: &[]
    },
    UPDATE_GLOBAL_RESOURCE_BINDING => {
        method: Patch, path: "/api/v1/resourceBindings/global", operation_id: "updateGlobalResourceBinding", summary: "Update a global resource binding",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("UpdateResourceBindingInput"), response_schema: Some("ResourceBindingsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[]
    },
    DELETE_GLOBAL_RESOURCE_BINDING => {
        method: Delete, path: "/api/v1/resourceBindings/global/{id}", operation_id: "deleteGlobalResourceBinding", summary: "Delete a global resource binding",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ResourceBindingsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_uuid("id")]
    },
    GET_RESOURCE_BINDINGS => {
        method: Get, path: "/api/v1/resourceBindings/{scope}/{resourceId}", operation_id: "getResourceBindings", summary: "Get resource bindings",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ResourceBindingsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_string("scope"), ParameterContract::path_uuid("resourceId")]
    },
    CREATE_RESOURCE_BINDING => {
        method: Post, path: "/api/v1/resourceBindings/{scope}/{resourceId}", operation_id: "createResourceBinding", summary: "Create a resource binding",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("ResourceBindingInput"), response_schema: Some("ResourceBindingsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_string("scope"), ParameterContract::path_uuid("resourceId")]
    },
    UPDATE_RESOURCE_BINDING => {
        method: Patch, path: "/api/v1/resourceBindings/{scope}/{resourceId}", operation_id: "updateResourceBinding", summary: "Update a resource binding",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("UpdateResourceBindingInput"), response_schema: Some("ResourceBindingsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_string("scope"), ParameterContract::path_uuid("resourceId")]
    },
    DELETE_RESOURCE_BINDING => {
        method: Delete, path: "/api/v1/resourceBindings/{scope}/{resourceId}/{id}", operation_id: "deleteResourceBinding", summary: "Delete a resource binding",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("ResourceBindingsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_string("scope"), ParameterContract::path_uuid("resourceId"), ParameterContract::path_uuid("id")]
    },
    LIST_SECRET_DEFINITIONS => {
        method: Get, path: "/api/v1/resourceBindings/secrets", operation_id: "listSecretDefinitions", summary: "List Secret definitions",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("SecretDefinitionsView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, TooManyRequests, InternalServerError], parameters: SECRET_DEFINITION_FILTER_PARAMETERS
    },
    CREATE_INTERNAL_SECRET => {
        method: Post, path: "/api/v1/resourceBindings/secrets", operation_id: "createInternalSecret", summary: "Create an internal encrypted Secret",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("CreateInternalSecretInput"), response_schema: Some("SecretDefinitionView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[ParameterContract::query("scope", ParameterSchema::String), ParameterContract::query("resourceId", ParameterSchema::Uuid)]
    },
    CREATE_EXTERNAL_SECRET => {
        method: Post, path: "/api/v1/resourceBindings/secrets/external", operation_id: "createExternalSecret", summary: "Create an external Secret definition",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("CreateExternalSecretInput"), response_schema: Some("SecretDefinitionView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[]
    },
    UPDATE_EXTERNAL_SECRET => {
        method: Patch, path: "/api/v1/resourceBindings/secrets/external/{id}", operation_id: "updateExternalSecret", summary: "Update an external Secret definition",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("UpdateExternalSecretInput"), response_schema: Some("SecretDefinitionView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_uuid("id")]
    },
    DELETE_SECRET_DEFINITION => {
        method: Delete, path: "/api/v1/resourceBindings/secrets/{id}", operation_id: "deleteSecretDefinition", summary: "Delete an unused Secret definition",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: None, success_status: 204,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_uuid("id")]
    },
    LIST_SECRET_PROVIDERS => {
        method: Get, path: "/api/v1/resourceBindings/secret-providers", operation_id: "listSecretProviders", summary: "List Secret providers",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: Some("SecretProvidersView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, TooManyRequests, InternalServerError], parameters: &[]
    },
    CREATE_VAULT_KV2_SECRET_PROVIDER => {
        method: Post, path: "/api/v1/resourceBindings/secret-providers/vault-kv2", operation_id: "createVaultKvV2SecretProvider", summary: "Create a Vault-compatible KV v2 Secret provider",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("CreateVaultKvV2SecretProviderInput"), response_schema: Some("SecretProviderView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, Conflict, TooManyRequests, InternalServerError], parameters: &[]
    },
    UPDATE_VAULT_KV2_SECRET_PROVIDER => {
        method: Patch, path: "/api/v1/resourceBindings/secret-providers/vault-kv2/{id}", operation_id: "updateVaultKvV2SecretProvider", summary: "Update a Vault-compatible KV v2 Secret provider",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: Some("UpdateVaultKvV2SecretProviderInput"), response_schema: Some("SecretProviderView"), success_status: 200,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_uuid("id")]
    },
    DELETE_SECRET_PROVIDER => {
        method: Delete, path: "/api/v1/resourceBindings/secret-providers/{id}", operation_id: "deleteSecretProvider", summary: "Delete a Secret provider",
        public: true, setup_exempt: false, authentication: Actor,
        request_schema: None, response_schema: None, success_status: 204,
        error_responses: [BadRequest, Unauthorized, Forbidden, NotFound, Conflict, TooManyRequests, InternalServerError], parameters: &[ParameterContract::path_uuid("id")]
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
    fn error_responses_are_explicit_unique_and_include_transport_failures() {
        for route in ROUTES {
            let statuses = route
                .error_responses
                .iter()
                .map(|response| response.status())
                .collect::<HashSet<_>>();
            assert_eq!(
                statuses.len(),
                route.error_responses.len(),
                "{} declares a duplicate error response",
                route.operation_id
            );
            assert!(
                statuses.contains(&400) && statuses.contains(&429) && statuses.contains(&500),
                "{} omits a transport-level error response",
                route.operation_id
            );
            assert!(
                !statuses.contains(&route.success_status),
                "{} declares its success response as an error",
                route.operation_id
            );
            if route.authentication != RouteAuthentication::Anonymous {
                assert!(
                    statuses.contains(&401) && statuses.contains(&403),
                    "{} omits an authentication or authorization response",
                    route.operation_id
                );
            }
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

    #[test]
    fn mfa_routes_preserve_the_dotnet_http_contract() {
        let expected = [
            (
                routes::VERIFY_AUTHENTICATION_MFA,
                HttpMethod::Post,
                "/api/v1/authentication/mfa/verify",
                RouteAuthentication::Anonymous,
            ),
            (
                routes::GET_AUTHENTICATION_MFA_SETUP,
                HttpMethod::Get,
                "/api/v1/authentication/mfa/setup",
                RouteAuthentication::Anonymous,
            ),
            (
                routes::CONFIRM_AUTHENTICATION_MFA_SETUP,
                HttpMethod::Post,
                "/api/v1/authentication/mfa/setup/confirm",
                RouteAuthentication::Anonymous,
            ),
            (
                routes::GET_PROFILE_MFA_STATUS,
                HttpMethod::Get,
                "/api/v1/profile/mfa",
                RouteAuthentication::Human,
            ),
            (
                routes::START_PROFILE_MFA_SETUP,
                HttpMethod::Post,
                "/api/v1/profile/mfa/setup",
                RouteAuthentication::Human,
            ),
            (
                routes::CONFIRM_PROFILE_MFA_SETUP,
                HttpMethod::Post,
                "/api/v1/profile/mfa/setup/confirm",
                RouteAuthentication::Human,
            ),
            (
                routes::DISABLE_PROFILE_MFA,
                HttpMethod::Post,
                "/api/v1/profile/mfa/disable",
                RouteAuthentication::Human,
            ),
            (
                routes::REGENERATE_PROFILE_MFA_RECOVERY_CODES,
                HttpMethod::Post,
                "/api/v1/profile/mfa/recovery-codes",
                RouteAuthentication::Human,
            ),
            (
                routes::RESET_USER_MFA,
                HttpMethod::Delete,
                "/api/v1/users/{id}/mfa",
                RouteAuthentication::Administrator,
            ),
        ];

        for (route, method, path, authentication) in expected {
            assert_eq!(route.method, method);
            assert_eq!(route.path, path);
            assert_eq!(route.authentication, authentication);
            assert!(!route.public);
        }
    }

    #[test]
    fn user_read_routes_preserve_the_dotnet_http_contract() {
        let expected = [
            (
                routes::LIST_USERS,
                "/api/v1/users",
                "listUsers",
                vec!["Name", "Page", "PageSize"],
            ),
            (
                routes::SEARCH_USERS,
                "/api/v1/users/search",
                "searchUsers",
                vec!["Query", "Limit"],
            ),
            (
                routes::GET_USER,
                "/api/v1/users/{id}",
                "getUser",
                vec!["id"],
            ),
        ];

        for (route, path, operation_id, parameter_names) in expected {
            assert_eq!(route.method, HttpMethod::Get);
            assert_eq!(route.path, path);
            assert_eq!(route.operation_id, operation_id);
            assert_eq!(route.authentication, RouteAuthentication::Administrator);
            assert!(!route.public);
            assert_eq!(
                route
                    .parameters
                    .iter()
                    .map(|parameter| parameter.name)
                    .collect::<Vec<_>>(),
                parameter_names
            );
        }
    }

    #[test]
    fn user_mutation_routes_preserve_the_dotnet_http_contract() {
        let expected = [
            (
                routes::CREATE_USER,
                HttpMethod::Post,
                "/api/v1/users",
                "createUser",
            ),
            (
                routes::UPDATE_USER,
                HttpMethod::Patch,
                "/api/v1/users/{id}",
                "updateUser",
            ),
            (
                routes::RENAME_USER,
                HttpMethod::Post,
                "/api/v1/users/rename",
                "renameUser",
            ),
            (
                routes::ADD_USER_ROLE,
                HttpMethod::Post,
                "/api/v1/users/{id}/roles",
                "addUserRole",
            ),
            (
                routes::REMOVE_USER_ROLE,
                HttpMethod::Delete,
                "/api/v1/users/{id}/roles/{roleId}",
                "removeUserRole",
            ),
            (
                routes::ADD_USER_RESOURCE_ACCESS,
                HttpMethod::Post,
                "/api/v1/users/{id}/resource-accesses",
                "addUserResourceAccess",
            ),
            (
                routes::REMOVE_USER_RESOURCE_ACCESS,
                HttpMethod::Delete,
                "/api/v1/users/{id}/resource-accesses",
                "removeUserResourceAccess",
            ),
            (
                routes::DELETE_USERS,
                HttpMethod::Delete,
                "/api/v1/users",
                "deleteUsers",
            ),
        ];

        for (route, method, path, operation_id) in expected {
            assert_eq!(route.method, method);
            assert_eq!(route.path, path);
            assert_eq!(route.operation_id, operation_id);
            assert_eq!(route.authentication, RouteAuthentication::Administrator);
            assert!(!route.public);
        }
    }

    #[test]
    fn team_routes_preserve_the_dotnet_http_contract() {
        let expected = [
            (
                routes::LIST_TEAMS,
                HttpMethod::Get,
                "/api/v1/teams",
                "listTeams",
            ),
            (
                routes::SEARCH_TEAMS,
                HttpMethod::Get,
                "/api/v1/teams/search",
                "searchTeams",
            ),
            (
                routes::GET_TEAM,
                HttpMethod::Get,
                "/api/v1/teams/{id}",
                "getTeam",
            ),
            (
                routes::CREATE_TEAM,
                HttpMethod::Post,
                "/api/v1/teams",
                "createTeam",
            ),
            (
                routes::PATCH_TEAM,
                HttpMethod::Patch,
                "/api/v1/teams/{id}",
                "updateTeam",
            ),
            (
                routes::RENAME_TEAM,
                HttpMethod::Post,
                "/api/v1/teams/rename",
                "renameTeam",
            ),
            (
                routes::ADD_TEAM_ROLE,
                HttpMethod::Post,
                "/api/v1/teams/{id}/roles",
                "addTeamRole",
            ),
            (
                routes::REMOVE_TEAM_ROLE,
                HttpMethod::Delete,
                "/api/v1/teams/{id}/roles/{roleId}",
                "removeTeamRole",
            ),
            (
                routes::ADD_TEAM_MEMBER,
                HttpMethod::Post,
                "/api/v1/teams/{id}/members",
                "addTeamMember",
            ),
            (
                routes::REMOVE_TEAM_MEMBER,
                HttpMethod::Delete,
                "/api/v1/teams/{id}/members/{memberActorId}",
                "removeTeamMember",
            ),
            (
                routes::ADD_TEAM_RESOURCE_ACCESS,
                HttpMethod::Post,
                "/api/v1/teams/{id}/resource-accesses",
                "addTeamResourceAccess",
            ),
            (
                routes::REMOVE_TEAM_RESOURCE_ACCESS,
                HttpMethod::Delete,
                "/api/v1/teams/{id}/resource-accesses",
                "removeTeamResourceAccess",
            ),
            (
                routes::DELETE_TEAMS,
                HttpMethod::Delete,
                "/api/v1/teams",
                "deleteTeams",
            ),
        ];

        for (route, method, path, operation_id) in expected {
            assert_eq!(route.method, method);
            assert_eq!(route.path, path);
            assert_eq!(route.operation_id, operation_id);
            assert_eq!(route.authentication, RouteAuthentication::Administrator);
            assert!(!route.public);
        }
    }

    #[test]
    fn role_routes_preserve_the_dotnet_http_contract() {
        let expected = [
            (
                routes::LIST_ROLES,
                HttpMethod::Get,
                "/api/v1/roles",
                "listRoles",
            ),
            (
                routes::GET_ROLE,
                HttpMethod::Get,
                "/api/v1/roles/{id}",
                "getRole",
            ),
            (
                routes::CREATE_ROLE,
                HttpMethod::Post,
                "/api/v1/roles",
                "createRole",
            ),
            (
                routes::PATCH_ROLE_PERMISSIONS,
                HttpMethod::Patch,
                "/api/v1/roles/{id}/permissions",
                "updateRolePermissions",
            ),
            (
                routes::RENAME_ROLE,
                HttpMethod::Post,
                "/api/v1/roles/rename",
                "renameRole",
            ),
            (
                routes::DELETE_ROLES,
                HttpMethod::Delete,
                "/api/v1/roles",
                "deleteRoles",
            ),
        ];

        for (route, method, path, operation_id) in expected {
            assert_eq!(route.method, method);
            assert_eq!(route.path, path);
            assert_eq!(route.operation_id, operation_id);
            assert_eq!(route.authentication, RouteAuthentication::Administrator);
            assert!(route.public);
        }
    }

    #[test]
    fn license_routes_preserve_the_dotnet_http_contract() {
        let expected = [
            (
                routes::GET_LICENSE,
                HttpMethod::Get,
                "/api/v1/license",
                "getLicense",
            ),
            (
                routes::GET_LICENSE_ENTITLEMENTS,
                HttpMethod::Get,
                "/api/v1/license/entitlements",
                "getLicenseEntitlements",
            ),
            (
                routes::INSTALL_LICENSE,
                HttpMethod::Post,
                "/api/v1/license",
                "installLicense",
            ),
            (
                routes::REMOVE_LICENSE,
                HttpMethod::Delete,
                "/api/v1/license",
                "removeLicense",
            ),
            (
                routes::GET_LICENSE_REQUEST,
                HttpMethod::Get,
                "/api/v1/license/request",
                "getLicenseRequest",
            ),
        ];

        for (route, method, path, operation_id) in expected {
            assert_eq!(route.method, method);
            assert_eq!(route.path, path);
            assert_eq!(route.operation_id, operation_id);
            assert!(route.public);
        }
        assert_eq!(
            routes::GET_LICENSE_ENTITLEMENTS.authentication,
            RouteAuthentication::Actor
        );
    }

    #[test]
    fn oidc_routes_preserve_the_dotnet_http_contract() {
        let browser = [
            (
                routes::LIST_OIDC_LOGIN_PROVIDERS,
                HttpMethod::Get,
                "/api/v1/authentication/oidc/providers",
                "listOidcLoginProviders",
            ),
            (
                routes::BEGIN_OIDC_LOGIN,
                HttpMethod::Get,
                "/api/v1/authentication/oidc/{id}/login",
                "beginOidcLogin",
            ),
            (
                routes::COMPLETE_OIDC_LOGIN,
                HttpMethod::Get,
                "/api/v1/authentication/oidc/{id}/callback",
                "completeOidcLogin",
            ),
        ];
        for (route, method, path, operation_id) in browser {
            assert_eq!(route.method, method);
            assert_eq!(route.path, path);
            assert_eq!(route.operation_id, operation_id);
            assert_eq!(route.authentication, RouteAuthentication::Anonymous);
            assert!(!route.public);
        }

        let administration = [
            (
                routes::LIST_OIDC_PROVIDERS,
                HttpMethod::Get,
                "/api/v1/oidcProviders",
                "listOidcProviders",
            ),
            (
                routes::GET_OIDC_PROVIDER,
                HttpMethod::Get,
                "/api/v1/oidcProviders/{id}",
                "getOidcProvider",
            ),
            (
                routes::CREATE_OIDC_PROVIDER,
                HttpMethod::Post,
                "/api/v1/oidcProviders",
                "createOidcProvider",
            ),
            (
                routes::RENAME_OIDC_PROVIDER,
                HttpMethod::Post,
                "/api/v1/oidcProviders/rename",
                "renameOidcProvider",
            ),
            (
                routes::UPDATE_OIDC_PROVIDER,
                HttpMethod::Patch,
                "/api/v1/oidcProviders/{id}",
                "updateOidcProvider",
            ),
            (
                routes::UPDATE_OIDC_PROVIDER_METADATA,
                HttpMethod::Patch,
                "/api/v1/oidcProviders/{id}/_metadata",
                "updateOidcProviderMetadata",
            ),
            (
                routes::DELETE_OIDC_PROVIDER,
                HttpMethod::Delete,
                "/api/v1/oidcProviders/{id}",
                "deleteOidcProvider",
            ),
            (
                routes::TEST_OIDC_PROVIDER_DISCOVERY,
                HttpMethod::Post,
                "/api/v1/oidcProviders/{id}/testDiscovery",
                "testOidcProviderDiscovery",
            ),
            (
                routes::TEST_OIDC_DISCOVERY,
                HttpMethod::Post,
                "/api/v1/oidcProviders/testDiscovery",
                "testOidcDiscovery",
            ),
        ];
        for (route, method, path, operation_id) in administration {
            assert_eq!(route.method, method);
            assert_eq!(route.path, path);
            assert_eq!(route.operation_id, operation_id);
            assert_eq!(route.authentication, RouteAuthentication::Administrator);
            assert!(route.public);
        }
    }

    #[test]
    fn phase_five_routes_preserve_mutation_methods_and_actor_authentication() {
        let expected = [
            (routes::CREATE_TAG, HttpMethod::Post, "/api/v1/tags"),
            (
                routes::REPLACE_PLATFORM_TAGS,
                HttpMethod::Put,
                "/api/v1/platforms/{id}/tags",
            ),
            (
                routes::UPDATE_REGISTRY,
                HttpMethod::Patch,
                "/api/v1/registries/{id}",
            ),
            (
                routes::UPDATE_GIT_REPOSITORY,
                HttpMethod::Patch,
                "/api/v1/gitRepositories/{id}",
            ),
            (
                routes::CREATE_RESOURCE_BINDING,
                HttpMethod::Post,
                "/api/v1/resourceBindings/{scope}/{resourceId}",
            ),
            (routes::CREATE_NETWORK, HttpMethod::Post, "/api/v1/networks"),
            (
                routes::DELETE_VOLUMES,
                HttpMethod::Delete,
                "/api/v1/volumes",
            ),
        ];
        for (route, method, path) in expected {
            assert_eq!(route.method, method);
            assert_eq!(route.path, path);
            assert_eq!(route.authentication, RouteAuthentication::Actor);
            assert!(route.public);
            assert!(route.error_responses.contains(&ErrorResponse::Unauthorized));
            assert!(route.error_responses.contains(&ErrorResponse::Forbidden));
        }
    }

    #[test]
    fn deployment_crud_routes_preserve_the_dotnet_http_contract() {
        let expected = [
            (
                routes::LIST_DEPLOYMENTS,
                HttpMethod::Get,
                "/api/v1/deployments",
                "listDeployments",
            ),
            (
                routes::CREATE_DEPLOYMENT,
                HttpMethod::Post,
                "/api/v1/deployments",
                "createDeployment",
            ),
            (
                routes::DELETE_DEPLOYMENTS,
                HttpMethod::Delete,
                "/api/v1/deployments",
                "deleteDeployments",
            ),
            (
                routes::RENAME_DEPLOYMENT,
                HttpMethod::Post,
                "/api/v1/deployments/rename",
                "renameDeployment",
            ),
            (
                routes::GET_DEPLOYMENT,
                HttpMethod::Get,
                "/api/v1/deployments/{deploymentId}",
                "getDeployment",
            ),
            (
                routes::GET_DEPLOYMENT_CONFIG,
                HttpMethod::Get,
                "/api/v1/deployments/{deploymentId}/_cfg",
                "getDeploymentConfig",
            ),
            (
                routes::GET_DEPLOYMENT_DUPLICATE_DRAFT,
                HttpMethod::Get,
                "/api/v1/deployments/{deploymentId}/duplicate-draft",
                "getDeploymentDuplicateDraft",
            ),
            (
                routes::UPDATE_DEPLOYMENT,
                HttpMethod::Patch,
                "/api/v1/deployments/{id}",
                "updateDeployment",
            ),
            (
                routes::UPDATE_DEPLOYMENT_METADATA,
                HttpMethod::Patch,
                "/api/v1/deployments/{id}/_metadata",
                "updateDeploymentMetadata",
            ),
        ];

        for (route, method, path, operation_id) in expected {
            assert_eq!(route.method, method);
            assert_eq!(route.path, path);
            assert_eq!(route.operation_id, operation_id);
            assert_eq!(route.authentication, RouteAuthentication::Actor);
            assert!(route.public);
            assert!(route.error_responses.contains(&ErrorResponse::Unauthorized));
        }
    }
}
