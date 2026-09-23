//! OpenAPI descriptors for feature-owned vocabulary; no schema dependency in feature crates.
#[derive(utoipa::ToSchema)]
#[schema(as = MfaPolicy)]
pub enum MfaPolicySchema {
    Optional,
    RequiredForAdministrators,
    RequiredForAllUsers,
}

#[derive(utoipa::ToSchema)]
#[schema(as = ActorType)]
pub enum ActorTypeSchema {
    User,
    System,
    Agent,
    ServiceAccount,
    Team,
}

#[derive(utoipa::ToSchema)]
#[schema(as = RoleType)]
pub enum RoleTypeSchema {
    System,
    Custom,
}

#[derive(utoipa::ToSchema)]
#[schema(as = UserDateTimeFormat)]
pub enum UserDateTimeFormatSchema {
    System,
    TwentyFourHour,
    TwelveHour,
}

#[derive(utoipa::ToSchema)]
#[schema(as = UserTheme)]
pub enum UserThemeSchema {
    System,
    Light,
    Dark,
}

#[derive(utoipa::ToSchema)]
#[schema(as = LookupResourceType)]
pub enum LookupResourceTypeSchema {
    Platform,
    Deployment,
    Stack,
    Image,
    Network,
    Volume,
    Registry,
    GitRepository,
    GitAccount,
    OidcProvider,
    AutomationAction,
    Alert,
    AlertChannel,
    User,
    UserActor,
    Team,
    Role,
    ResourceBinding,
    License,
    BackupRepository,
    BackupPolicy,
    Build,
    BuildAgentPool,
    SwarmService,
    RunAsActor,
    ServiceAccount,
}

// Descriptors derive their closed wire vocabulary from the owning feature. They
// do not add an OpenAPI dependency or duplicate enum variants in that feature.
macro_rules! vocabulary_schema {
    ($schema:ident, $name:literal, $values:expr) => {
        pub struct $schema;
        impl utoipa::ToSchema for $schema {
            fn name() -> std::borrow::Cow<'static, str> {
                $name.into()
            }
        }
        impl utoipa::PartialSchema for $schema {
            fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
                utoipa::openapi::schema::ObjectBuilder::new()
                    .schema_type(utoipa::openapi::schema::Type::String)
                    .enum_values(Some($values.iter().map(|value| {
                        serde_json::to_value(value).expect("closed vocabulary serializes")
                    })))
                    .into()
            }
        }
    };
}

vocabulary_schema!(
    ActivityResourceTypeSchema,
    "ActivityResourceType",
    citadel_activities::ActivityResourceType::ALL
);

vocabulary_schema!(
    ActivityEventTypeSchema,
    "ActivityEventType",
    citadel_activities::ActivityEventType::ALL
);

vocabulary_schema!(
    LicenseCapabilitySchema,
    "LicenseCapability",
    citadel_licensing::LicenseCapability::ALL
);

vocabulary_schema!(
    LicenseStatusSchema,
    "LicenseStatus",
    citadel_licensing::LicenseStatus::ALL
);

vocabulary_schema!(
    ResourceTypeSchema,
    "ResourceType",
    citadel_primitives::ResourceType::ALL
);

vocabulary_schema!(
    PermissionLevelSchema,
    "PermissionLevel",
    [
        citadel_primitives::PermissionLevel::None,
        citadel_primitives::PermissionLevel::Read,
        citadel_primitives::PermissionLevel::Write,
        citadel_primitives::PermissionLevel::Execute
    ]
);

vocabulary_schema!(
    SpecificPermissionSchema,
    "SpecificPermission",
    citadel_primitives::SpecificPermission::ALL
);

pub struct ActorIdSchema;

impl utoipa::ToSchema for ActorIdSchema {
    fn name() -> std::borrow::Cow<'static, str> {
        "ActorId".into()
    }
}

impl utoipa::PartialSchema for ActorIdSchema {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        utoipa::openapi::schema::ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::Type::String)
            .format(Some(utoipa::openapi::schema::SchemaFormat::KnownFormat(
                utoipa::openapi::schema::KnownFormat::Uuid,
            )))
            .into()
    }
}
