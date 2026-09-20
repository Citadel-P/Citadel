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
