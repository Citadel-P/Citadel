//! Server-owned OpenAPI descriptions of primitives values.
//! Field types and exhaustive conversions keep these descriptions tied to the domain.

use chrono::{DateTime, Utc};

enum_schema!(
    ResourceControlStateSchema,
    "ResourceControlState",
    citadel_primitives::ResourceControlState,
    [Idle, Queued, Processing]
);

enum_schema!(
    AutoUpdateStatusSchema,
    "AutoUpdateStatus",
    citadel_primitives::AutoUpdateStatus,
    [Unknown, UpToDate, UpdateAvailable, Updating, Failed]
);

schema_model! {
    citadel_primitives::AutoUpdateState =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = AutoUpdateState)]
    pub struct AutoUpdateStateSchema {
        pub last_checked_at: DateTime<Utc>,
        #[schema(value_type = crate::api::resources::schema_models::primitives::AutoUpdateStatusSchema)]
        pub status: citadel_primitives::AutoUpdateStatus,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub current_digest: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub remote_digest: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub last_error: Option<String>,
    }
}

enum_schema!(
    WebhookProviderSchema,
    "WebhookProvider",
    citadel_primitives::WebhookProvider,
    [GitHub, GitLab, Generic]
);

enum_schema!(
    WebhookAuthSchemeSchema,
    "WebhookAuthScheme",
    citadel_primitives::WebhookAuthScheme,
    [
        GitHubHmacSha256,
        GitLabSignedToken,
        GitLabLegacyToken,
        BearerToken
    ]
);

schema_model! {
    citadel_primitives::WebhookConfig =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[doc = " Shared webhook wire configuration and defaults."]
    #[serde(default, rename_all = "camelCase")]
    #[schema(as = WebhookConfig)]
    pub struct WebhookConfigSchema {
        pub enabled: bool,
        #[schema(value_type = crate::api::resources::schema_models::primitives::WebhookProviderSchema)]
        pub provider: citadel_primitives::WebhookProvider,
        #[schema(value_type = crate::api::resources::schema_models::primitives::WebhookAuthSchemeSchema)]
        pub auth_scheme: citadel_primitives::WebhookAuthScheme,
        pub secret: Option<String>,
        pub branch_filter: Option<String>,
    }
}

schema_model! {
    citadel_primitives::WebhookPatch =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[doc = " Partial configuration. Defaults belong to creation, never to omitted patch fields."]
    #[serde(rename_all = "camelCase", deny_unknown_fields)]
    #[schema(as = WebhookPatch)]
    pub struct WebhookPatchSchema {
        #[serde(default, skip_serializing_if = "citadel_primitives::FieldUpdate::is_missing")]
        # [schema (value_type = bool, required = false)]
        pub enabled: citadel_primitives::FieldUpdate<bool>,
        #[serde(default, skip_serializing_if = "citadel_primitives::FieldUpdate::is_missing")]
        # [schema (value_type = crate::api::resources::schema_models::primitives::WebhookProviderSchema, required = false)]
        pub provider: citadel_primitives::FieldUpdate<citadel_primitives::WebhookProvider>,
        #[serde(default, skip_serializing_if = "citadel_primitives::FieldUpdate::is_missing")]
        # [schema (value_type = crate::api::resources::schema_models::primitives::WebhookAuthSchemeSchema, required = false)]
        pub auth_scheme: citadel_primitives::FieldUpdate<citadel_primitives::WebhookAuthScheme>,
        #[serde(default, skip_serializing_if = "citadel_primitives::FieldUpdate::is_missing")]
        # [schema (value_type = Option < String >, required = false)]
        pub secret: citadel_primitives::FieldUpdate<Option<String>>,
        #[serde(default, skip_serializing_if = "citadel_primitives::FieldUpdate::is_missing")]
        # [schema (value_type = Option < String >, required = false)]
        pub branch_filter: citadel_primitives::FieldUpdate<Option<String>>,
    }
}

enum_schema!(
    ResourceTypeSchema,
    "ResourceType",
    citadel_primitives::ResourceType,
    [
        Platform,
        Deployment,
        Stack,
        Registry,
        GitRepository,
        GitAccount,
        Alert,
        AlertChannel,
        User,
        Team,
        Role,
        Binding,
        Tag,
        AutomationAction,
        License,
        BackupRepository,
        BackupPolicy,
        Volume,
        Build,
        BuildAgentPool,
        SwarmService,
        ServiceAccount
    ]
);

enum_schema!(
    PermissionLevelSchema,
    "PermissionLevel",
    citadel_primitives::PermissionLevel,
    [None, Read, Write, Execute]
);

impl Default for WebhookConfigSchema {
    fn default() -> Self {
        citadel_primitives::WebhookConfig::default().into()
    }
}

enum_schema!(
    PlatformStatusSchema,
    "PlatformStatus",
    citadel_primitives::PlatformStatus,
    [Offline, Online]
);

enum_schema!(
    UpdateBehaviorSchema,
    "UpdateBehavior",
    citadel_primitives::UpdateBehavior,
    [Disabled, Notify, AutoDeploy]
);
