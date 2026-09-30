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

pub use super::schema_models::activities::ActivityResourceTypeSchema;

pub use super::schema_models::activities::ActivityEventTypeSchema;

pub use super::schema_models::licensing::LicenseCapabilitySchema;

pub use super::schema_models::licensing::LicenseStatusSchema;

pub use super::schema_models::primitives::ResourceTypeSchema;

pub use super::schema_models::primitives::PermissionLevelSchema;

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

#[derive(utoipa::ToSchema)]
#[schema(as = UserThemeColor)]
pub enum UserThemeColorSchema {
    Neutral,
    Blue,
    Indigo,
    Violet,
    Emerald,
    Yellow,
    Orange,
    Rose,
}

#[derive(utoipa::ToSchema)]
#[schema(as = UserUiFont)]
pub enum UserUiFontSchema {
    Geist,
    Inter,
    IbmPlexSans,
    SourceSans3,
    System,
}

#[derive(utoipa::ToSchema)]
#[schema(as = UserUiRadius)]
pub enum UserUiRadiusSchema {
    None,
    Small,
    Medium,
    Large,
}

#[derive(utoipa::ToSchema)]
#[schema(as = UserContentLayout)]
pub enum UserContentLayoutSchema {
    Compact,
    Wide,
    Full,
}

#[derive(utoipa::ToSchema)]
#[schema(as = UserUiDensity)]
pub enum UserUiDensitySchema {
    Compact,
    Comfortable,
}

pub use super::schema_models::activities::ActivityStatusSchema;
