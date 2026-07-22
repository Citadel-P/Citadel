/* eslint-disable */
/* tslint:disable */
// @ts-nocheck
/*
 * ---------------------------------------------------------------
 * ## THIS FILE WAS GENERATED VIA SWAGGER-TYPESCRIPT-API        ##
 * ##                                                           ##
 * ## AUTHOR: acacode                                           ##
 * ## SOURCE: https://github.com/acacode/swagger-typescript-api ##
 * ---------------------------------------------------------------
 */

/** @default "GitHub" */
export enum WebhookProvider {
  GitHub = "GitHub",
  GitLab = "GitLab",
}

/** @default "GitHubHmacSha256" */
export enum WebhookAuthScheme {
  GitHubHmacSha256 = "GitHubHmacSha256",
  GitLabSignedToken = "GitLabSignedToken",
  GitLabLegacyToken = "GitLabLegacyToken",
}

export enum VolumeSharing {
  None = "None",
  ReadOnly = "ReadOnly",
  OneWriter = "OneWriter",
  All = "All",
}

export enum VolumeScope {
  Single = "Single",
  Multi = "Multi",
}

export enum VolumeFileEntryType {
  Directory = "Directory",
  File = "File",
  Symlink = "Symlink",
  Other = "Other",
}

/** @default "Live" */
export enum VolumeBackupConsistency {
  Live = "Live",
  StopAttachedContainers = "StopAttachedContainers",
}

export enum UserTheme {
  System = "System",
  Light = "Light",
  Dark = "Dark",
}

export enum UserDateTimeFormat {
  System = "System",
  TwentyFourHour = "TwentyFourHour",
  TwelveHour = "TwelveHour",
}

export enum UpdateBehavior {
  Disabled = "Disabled",
  Notify = "Notify",
  AutoDeploy = "AutoDeploy",
}

export enum StopSignal {
  SIGTERM = "SIGTERM",
  SIGKILL = "SIGKILL",
  SIGINT = "SIGINT",
}

export enum StackVolumeKind {
  DeclaredNamed = "DeclaredNamed",
  ExternalNamed = "ExternalNamed",
  AnonymousNamed = "AnonymousNamed",
}

export enum StackUpdateBehavior {
  Disabled = "Disabled",
  Notify = "Notify",
  ServiceAutoDeploy = "ServiceAutoDeploy",
  StackAutoDeploy = "StackAutoDeploy",
}

export enum StackSource {
  WebEditor = "WebEditor",
  Git = "Git",
}

export enum StackReleaseStatus {
  Unknown = "Unknown",
  Created = "Created",
  Applying = "Applying",
  Healthy = "Healthy",
  Pending = "Pending",
  Paused = "Paused",
  Degraded = "Degraded",
  Failed = "Failed",
  Stopped = "Stopped",
}

export enum StackReconciliationStatus {
  NoDrift = "NoDrift",
  Reconciled = "Reconciled",
  Partial = "Partial",
  RequiresReapply = "RequiresReapply",
  Disabled = "Disabled",
  Failed = "Failed",
}

export enum StackReconciliationActionType {
  StartContainer = "StartContainer",
  ResumeContainer = "ResumeContainer",
  RemoveContainer = "RemoveContainer",
}

export enum StackDriftMode {
  Disabled = "Disabled",
  DetectOnly = "DetectOnly",
  AutoFix = "AutoFix",
}

export enum StackApplyEventType {
  StdOut = "StdOut",
  StdErr = "StdErr",
  SystemMessage = "SystemMessage",
  CommandCompleted = "CommandCompleted",
}

export enum SpecificPermission {
  None = "None",
  Logs = "Logs",
  Inspect = "Inspect",
  Apply = "Apply",
  Pull = "Pull",
  Terminal = "Terminal",
  ResourceBindings = "ResourceBindings",
  Releases = "Releases",
  Restore = "Restore",
  Browse = "Browse",
  Download = "Download",
}

export enum SecretProviderType {
  InternalEncrypted = "InternalEncrypted",
  VaultCompatibleKvV2 = "VaultCompatibleKvV2",
}

export enum ScheduleType {
  Daily = "Daily",
  Weekly = "Weekly",
}

export enum S3BucketLookup {
  Auto = "Auto",
  Path = "Path",
  Dns = "Dns",
}

export enum RoleType {
  System = "System",
  Custom = "Custom",
}

export enum ResourceType {
  Platform = "Platform",
  Deployment = "Deployment",
  Stack = "Stack",
  Registry = "Registry",
  GitRepository = "GitRepository",
  GitAccount = "GitAccount",
  Alert = "Alert",
  AlertChannel = "AlertChannel",
  User = "User",
  Team = "Team",
  Role = "Role",
  Binding = "Binding",
  Tag = "Tag",
  AutomationAction = "AutomationAction",
  License = "License",
  BackupRepository = "BackupRepository",
  BackupPolicy = "BackupPolicy",
  Volume = "Volume",
  Build = "Build",
  BuildAgentPool = "BuildAgentPool",
}

export enum ResourceControlState {
  Idle = "Idle",
  Processing = "Processing",
}

export enum ResourceBindingScope {
  Global = "Global",
  Stack = "Stack",
  Deployment = "Deployment",
}

export enum ResourceBindingKind {
  Variable = "Variable",
  Secret = "Secret",
}

export enum RegistryType {
  Custom = "Custom",
  DockerHub = "DockerHub",
  Azure = "Azure",
  AWS = "AWS",
  Gitlab = "Gitlab",
  GitHub = "GitHub",
}

export enum RegistryStatus {
  Active = "Active",
  Disabled = "Disabled",
  Deprecated = "Deprecated",
}

export enum PruneResource {
  All = "All",
  Volume = "Volume",
  Network = "Network",
  Image = "Image",
  Build = "Build",
}

export enum PlatformType {
  Docker = "Docker",
  DockerSwarm = "DockerSwarm",
  Kubernetes = "Kubernetes",
}

export enum PlatformStatus {
  Offline = "Offline",
  Online = "Online",
}

export enum PlatformConnectorType {
  Unknown = "Unknown",
  Local = "Local",
  Agent = "Agent",
  EdgeAgent = "EdgeAgent",
}

export enum PermissionLevel {
  None = "None",
  Read = "Read",
  Write = "Write",
  Execute = "Execute",
}

export enum MfaPolicy {
  Optional = "Optional",
  RequiredForAdministrators = "RequiredForAdministrators",
  RequiredForAllUsers = "RequiredForAllUsers",
}

export enum LookupResourceType {
  Platform = "Platform",
  Deployment = "Deployment",
  Stack = "Stack",
  Image = "Image",
  Network = "Network",
  Volume = "Volume",
  Registry = "Registry",
  GitRepository = "GitRepository",
  GitAccount = "GitAccount",
  OidcProvider = "OidcProvider",
  AutomationAction = "AutomationAction",
  Alert = "Alert",
  AlertChannel = "AlertChannel",
  User = "User",
  UserActor = "UserActor",
  Team = "Team",
  Role = "Role",
  ResourceBinding = "ResourceBinding",
  License = "License",
  BackupRepository = "BackupRepository",
  BackupPolicy = "BackupPolicy",
  BuildAgentPool = "BuildAgentPool",
}

export enum LoginNextStep {
  Completed = "Completed",
  VerifyMfa = "VerifyMfa",
  EnrollMfa = "EnrollMfa",
}

export enum LicenseStatus {
  Community = "Community",
  Valid = "Valid",
  GracePeriod = "GracePeriod",
  NotYetValid = "NotYetValid",
  Expired = "Expired",
  Invalid = "Invalid",
  InstanceMismatch = "InstanceMismatch",
  UnsupportedSchema = "UnsupportedSchema",
  UnknownSigningKey = "UnknownSigningKey",
}

export enum LicenseLimit {
  CustomRoles = "CustomRoles",
  ActiveUsers = "ActiveUsers",
  Platforms = "Platforms",
  BackupPolicies = "BackupPolicies",
  AutomationActions = "AutomationActions",
}

export enum GitTransport {
  Http = "Http",
  Https = "Https",
  Ssh = "Ssh",
}

export enum GitRepositorySyncMode {
  Manual = "Manual",
  PullInterval = "PullInterval",
}

export enum GitReposStatus {
  Unknown = "Unknown",
  Pending = "Pending",
  Created = "Created",
  Healthy = "Healthy",
  Degraded = "Degraded",
}

export enum GitAuthType {
  Basic = "Basic",
  Token = "Token",
  SshKey = "SshKey",
}

export enum DockerHubTagStatus {
  Active = "Active",
  Inactive = "Inactive",
}

export enum DockerHubImageStatus {
  Active = "Active",
  Inactive = "Inactive",
}

export enum DeploymentStatus {
  Unknown = "Unknown",
  Created = "Created",
  Pending = "Pending",
  Applying = "Applying",
  Healthy = "Healthy",
  Degraded = "Degraded",
  Failed = "Failed",
  Stopped = "Stopped",
}

export enum DayOfWeek {
  Sunday = "Sunday",
  Monday = "Monday",
  Tuesday = "Tuesday",
  Wednesday = "Wednesday",
  Thursday = "Thursday",
  Friday = "Friday",
  Saturday = "Saturday",
}

export enum CpuArchitecture {
  Amd64 = "Amd64",
  Arm64 = "Arm64",
}

export enum ContainerStateStatus {
  Unknown = "Unknown",
  Created = "Created",
  Running = "Running",
  Paused = "Paused",
  Restarting = "Restarting",
  Exited = "Exited",
  Removing = "Removing",
  Dead = "Dead",
  Offline = "Offline",
}

export enum ContainerRestartPolicy {
  No = "No",
  Always = "Always",
  OnFailure = "OnFailure",
  UnlessStopped = "UnlessStopped",
}

export enum BuildRunTrigger {
  Manual = "Manual",
  Automation = "Automation",
  Schedule = "Schedule",
  Webhook = "Webhook",
}

export enum BuildRunStatus {
  Queued = "Queued",
  Preparing = "Preparing",
  Running = "Running",
  Succeeded = "Succeeded",
  Failed = "Failed",
  TimedOut = "TimedOut",
  Cancelled = "Cancelled",
  Interrupted = "Interrupted",
}

export enum BuildProjectBuilderKind {
  Platform = "Platform",
  BuildAgentPool = "BuildAgentPool",
}

export enum BuildAgentPoolValidationStatus {
  NotTested = "NotTested",
  Ready = "Ready",
  Invalid = "Invalid",
  Degraded = "Degraded",
}

export enum BuildAgentPoolProvider {
  AwsEc2 = "AwsEc2",
  SelfManagedVm = "SelfManagedVm",
}

/** @default "InboundAgent" */
export enum BuildAgentPoolConnectionMode {
  InboundAgent = "InboundAgent",
  EdgeAgent = "EdgeAgent",
}

export enum BackupSourceType {
  DockerVolume = "DockerVolume",
  CitadelSystem = "CitadelSystem",
  Stack = "Stack",
  Deployment = "Deployment",
}

export enum BackupSnapshotAvailability {
  Pending = "Pending",
  Available = "Available",
  Expired = "Expired",
  Missing = "Missing",
  NotCreated = "NotCreated",
}

export enum BackupRunTrigger {
  Manual = "Manual",
  Schedule = "Schedule",
  Automation = "Automation",
  Webhook = "Webhook",
}

export enum BackupRunStatus {
  Queued = "Queued",
  Preparing = "Preparing",
  Running = "Running",
  ApplyingRetention = "ApplyingRetention",
  Succeeded = "Succeeded",
  SucceededWithWarnings = "SucceededWithWarnings",
  Failed = "Failed",
  TimedOut = "TimedOut",
  Cancelled = "Cancelled",
  Rejected = "Rejected",
  Interrupted = "Interrupted",
}

export enum BackupRunItemStatus {
  Pending = "Pending",
  Running = "Running",
  Succeeded = "Succeeded",
  Failed = "Failed",
  Cancelled = "Cancelled",
}

export enum BackupRestoreStatus {
  Queued = "Queued",
  Preparing = "Preparing",
  Running = "Running",
  Succeeded = "Succeeded",
  SucceededWithWarnings = "SucceededWithWarnings",
  Failed = "Failed",
  TimedOut = "TimedOut",
  Cancelled = "Cancelled",
  Rejected = "Rejected",
  Interrupted = "Interrupted",
}

export enum BackupRepositoryValidationStatus {
  Unknown = "Unknown",
  Ready = "Ready",
  Uninitialized = "Uninitialized",
  Unavailable = "Unavailable",
  InvalidPassword = "InvalidPassword",
  InvalidConfiguration = "InvalidConfiguration",
}

export enum BackupRepositoryType {
  FileSystem = "FileSystem",
  S3Compatible = "S3Compatible",
}

export enum BackupRepositoryStatus {
  Unknown = "Unknown",
  Uninitialized = "Uninitialized",
  Ready = "Ready",
}

export enum BackupExecutionLocation {
  Core = "Core",
  Platform = "Platform",
}

export enum BackupCoverageStatus {
  NotApplicable = "NotApplicable",
  Unprotected = "Unprotected",
  Protected = "Protected",
  Warning = "Warning",
  Failed = "Failed",
}

export enum AutoUpdateStatus {
  Unknown = "Unknown",
  UpToDate = "UpToDate",
  UpdateAvailable = "UpdateAvailable",
  Updating = "Updating",
  Failed = "Failed",
}

export enum AlertType {
  PlatformCpuHigh = "PlatformCpuHigh",
  PlatformRamHigh = "PlatformRamHigh",
  PlatformUnreachable = "PlatformUnreachable",
  PlatformVersionMismatch = "PlatformVersionMismatch",
  UnmanagedContainerCreated = "UnmanagedContainerCreated",
  DeploymentImageUpdateAvailable = "DeploymentImageUpdateAvailable",
  DeploymentAutoDeployFailed = "DeploymentAutoDeployFailed",
  DeploymentAutoUpdated = "DeploymentAutoUpdated",
  StackImageUpdateAvailable = "StackImageUpdateAvailable",
  StackAutoDeployFailed = "StackAutoDeployFailed",
  StackAutoUpdated = "StackAutoUpdated",
  StackServiceAutoDeployFailed = "StackServiceAutoDeployFailed",
  StackServiceAutoUpdated = "StackServiceAutoUpdated",
  StackDriftDetected = "StackDriftDetected",
  StackDriftAutoReconciled = "StackDriftAutoReconciled",
  StackGitUpdateAvailable = "StackGitUpdateAvailable",
  StackGitAutoUpdated = "StackGitAutoUpdated",
  StackGitAutoDeployFailed = "StackGitAutoDeployFailed",
  StackConfigurationResolutionFailed = "StackConfigurationResolutionFailed",
  DeploymentConfigurationResolutionFailed = "DeploymentConfigurationResolutionFailed",
  WebhookAuthenticationFailed = "WebhookAuthenticationFailed",
  WebhookDispatchFailed = "WebhookDispatchFailed",
  WebhookGitRepoSyncFailed = "WebhookGitRepoSyncFailed",
  WebhookStackGitDeployFailed = "WebhookStackGitDeployFailed",
  AutomationActionRunFailed = "AutomationActionRunFailed",
  LicenseEnteredGracePeriod = "LicenseEnteredGracePeriod",
  LicenseExpired = "LicenseExpired",
}

export enum AlertSeverity {
  Info = "Info",
  Warning = "Warning",
  Critical = "Critical",
}

export enum AlertRuleStatus {
  Enabled = "Enabled",
  Disabled = "Disabled",
}

export enum AlertResourceType {
  Platform = "Platform",
  Deployment = "Deployment",
  Stack = "Stack",
  GitRepository = "GitRepository",
  Webhook = "Webhook",
  AutomationAction = "AutomationAction",
  License = "License",
}

export enum AlertEventStatus {
  Active = "Active",
  Acknowledged = "Acknowledged",
  Resolved = "Resolved",
}

export enum AlertDestination {
  Generic = "Generic",
  Bark = "Bark",
  Discord = "Discord",
  Gotify = "Gotify",
  GoogleChat = "Google_Chat",
  IFTTT = "IFTTT",
  Join = "Join",
  Lark = "Lark",
  Mattermost = "Mattermost",
  Matrix = "Matrix",
  Ntfy = "Ntfy",
  OpsGenie = "OpsGenie",
  Pushbullet = "Pushbullet",
  Pushover = "Pushover",
  Rocketchat = "Rocketchat",
  Signal = "Signal",
  Slack = "Slack",
  Teams = "Teams",
  Telegram = "Telegram",
  WeCom = "WeCom",
  ZulipChat = "Zulip_Chat",
}

export enum ActorType {
  User = "User",
  System = "System",
  Agent = "Agent",
  Service = "Service",
  Team = "Team",
}

export enum ActivityStatus {
  Success = "Success",
  Failure = "Failure",
  Warning = "Warning",
  Information = "Information",
}

export enum ActivityResourceType {
  Platform = "Platform",
  Registry = "Registry",
  Deployment = "Deployment",
  Stack = "Stack",
  AlertRule = "AlertRule",
  GitRepository = "GitRepository",
  OidcProvider = "OidcProvider",
  AutomationAction = "AutomationAction",
  User = "User",
  License = "License",
  Build = "Build",
  BuildAgentPool = "BuildAgentPool",
  Volume = "Volume",
}

export enum ActivityEventType {
  DeploymentCreated = "DeploymentCreated",
  DeploymentDuplicated = "DeploymentDuplicated",
  DeploymentUpdated = "DeploymentUpdated",
  DeploymentRenamed = "DeploymentRenamed",
  DeploymentDeleted = "DeploymentDeleted",
  DeploymentStarted = "DeploymentStarted",
  DeploymentStopped = "DeploymentStopped",
  DeploymentPaused = "DeploymentPaused",
  DeploymentApplied = "DeploymentApplied",
  DeploymentDegraded = "DeploymentDegraded",
  PlatformCreated = "PlatformCreated",
  PlatformDeleted = "PlatformDeleted",
  PlatformConnected = "PlatformConnected",
  PlatformDisconnected = "PlatformDisconnected",
  PlatformRenamed = "PlatformRenamed",
  RegistryCreated = "RegistryCreated",
  RegistryRenamed = "RegistryRenamed",
  RegistryUpdated = "RegistryUpdated",
  RegistryDeleted = "RegistryDeleted",
  AlertRuleCreated = "AlertRuleCreated",
  AlertRuleUpdated = "AlertRuleUpdated",
  AlertRuleDeleted = "AlertRuleDeleted",
  AlertRuleRenamed = "AlertRuleRenamed",
  GitRepoCreated = "GitRepoCreated",
  GitRepoUpdated = "GitRepoUpdated",
  GitRepoDeleted = "GitRepoDeleted",
  GitRepoRenamed = "GitRepoRenamed",
  GitRepoPulled = "GitRepoPulled",
  GitRepoCloned = "GitRepoCloned",
  GitRepoWebhookReceived = "GitRepoWebhookReceived",
  OidcProviderCreated = "OidcProviderCreated",
  OidcProviderUpdated = "OidcProviderUpdated",
  OidcProviderRenamed = "OidcProviderRenamed",
  OidcProviderDeleted = "OidcProviderDeleted",
  ActionCreated = "ActionCreated",
  ActionUpdated = "ActionUpdated",
  ActionRenamed = "ActionRenamed",
  ActionDeleted = "ActionDeleted",
  ActionRunQueued = "ActionRunQueued",
  ActionRunStarted = "ActionRunStarted",
  ActionRunSucceeded = "ActionRunSucceeded",
  ActionRunFailed = "ActionRunFailed",
  ActionRunTimedOut = "ActionRunTimedOut",
  ActionRunCancelled = "ActionRunCancelled",
  ActionRunRejected = "ActionRunRejected",
  StackCreated = "StackCreated",
  StackDuplicated = "StackDuplicated",
  StackUpdated = "StackUpdated",
  StackRenamed = "StackRenamed",
  StackDeleted = "StackDeleted",
  StackStarted = "StackStarted",
  StackStopped = "StackStopped",
  StackPaused = "StackPaused",
  StackApplied = "StackApplied",
  StackRollback = "StackRollback",
  StackDegraded = "StackDegraded",
  StackDriftDetected = "StackDriftDetected",
  StackDriftResolved = "StackDriftResolved",
  StackReconciliationAttempted = "StackReconciliationAttempted",
  StackGitUpdateAvailable = "StackGitUpdateAvailable",
  StackGitAutoUpdated = "StackGitAutoUpdated",
  StackGitAutoDeployFailed = "StackGitAutoDeployFailed",
  StackWebhookReceived = "StackWebhookReceived",
  UserProfileUpdated = "UserProfileUpdated",
  UserPreferencesUpdated = "UserPreferencesUpdated",
  UserPasswordChanged = "UserPasswordChanged",
  UserSessionRevoked = "UserSessionRevoked",
  UserOtherSessionsRevoked = "UserOtherSessionsRevoked",
  UserMfaEnabled = "UserMfaEnabled",
  UserMfaDisabled = "UserMfaDisabled",
  UserMfaVerificationFailed = "UserMfaVerificationFailed",
  UserMfaRecoveryCodeUsed = "UserMfaRecoveryCodeUsed",
  UserMfaRecoveryCodesRegenerated = "UserMfaRecoveryCodesRegenerated",
  UserMfaResetByAdministrator = "UserMfaResetByAdministrator",
  LicenseInstalled = "LicenseInstalled",
  LicenseReplaced = "LicenseReplaced",
  LicenseRemoved = "LicenseRemoved",
  LicenseEnteredGracePeriod = "LicenseEnteredGracePeriod",
  LicenseExpired = "LicenseExpired",
  LicenseValidationFailed = "LicenseValidationFailed",
  VolumeContentDownloaded = "VolumeContentDownloaded",
  BuildCreated = "BuildCreated",
  BuildUpdated = "BuildUpdated",
  BuildRenamed = "BuildRenamed",
  BuildDeleted = "BuildDeleted",
  BuildRunQueued = "BuildRunQueued",
  BuildRunStarted = "BuildRunStarted",
  BuildRunSucceeded = "BuildRunSucceeded",
  BuildRunFailed = "BuildRunFailed",
  BuildRunTimedOut = "BuildRunTimedOut",
  BuildRunCancelled = "BuildRunCancelled",
  BuildWebhookReceived = "BuildWebhookReceived",
  BuildAgentPoolCreated = "BuildAgentPoolCreated",
  BuildAgentPoolUpdated = "BuildAgentPoolUpdated",
  BuildAgentPoolRenamed = "BuildAgentPoolRenamed",
  BuildAgentPoolDeleted = "BuildAgentPoolDeleted",
  BuildAgentPoolTested = "BuildAgentPoolTested",
}

export enum ActionRunTrigger {
  Manual = "Manual",
  Test = "Test",
  Schedule = "Schedule",
  Webhook = "Webhook",
}

export enum ActionRunStatus {
  Queued = "Queued",
  Running = "Running",
  Succeeded = "Succeeded",
  Failed = "Failed",
  TimedOut = "TimedOut",
  Cancelled = "Cancelled",
  Rejected = "Rejected",
}

export type StackUpdateState = BaseStackUpdateState &
  (
    | BaseStackUpdateStateTypeMapping<
        "Git",
        StackUpdateStateGitStackUpdateState
      >
    | BaseStackUpdateStateTypeMapping<
        "WebEditor",
        StackUpdateStateManualStackUpdateState
      >
  );

export type StackSpec = BaseStackSpec &
  (
    | BaseStackSpecTypeMapping<"WebEditor", StackSpecManualStack>
    | BaseStackSpecTypeMapping<"Git", StackSpecGitStack>
  );

export type StackDrift = BaseStackDrift &
  (
    | BaseStackDriftTypeMapping<"MissingContainer", StackDriftMissingContainer>
    | BaseStackDriftTypeMapping<"ExtraContainer", StackDriftExtraContainer>
    | BaseStackDriftTypeMapping<"ContainerStopped", StackDriftContainerStopped>
    | BaseStackDriftTypeMapping<"ContainerPaused", StackDriftContainerPaused>
    | BaseStackDriftTypeMapping<
        "ContainerUnhealthy",
        StackDriftContainerUnhealthy
      >
    | BaseStackDriftTypeMapping<"ImageMismatch", StackDriftImageMismatch>
    | BaseStackDriftTypeMapping<
        "ConfigHashMismatch",
        StackDriftConfigHashMismatch
      >
  );

export type RegistryConfiguration = BaseRegistryConfiguration &
  (
    | BaseRegistryConfigurationTypeMapping<
        "AWS",
        RegistryConfigurationAWSRegistry
      >
    | BaseRegistryConfigurationTypeMapping<
        "Azure",
        RegistryConfigurationAzureRegistry
      >
    | BaseRegistryConfigurationTypeMapping<
        "Gitlab",
        RegistryConfigurationGitlabRegistry
      >
    | BaseRegistryConfigurationTypeMapping<
        "DockerHub",
        RegistryConfigurationDockerHubRegistry
      >
    | BaseRegistryConfigurationTypeMapping<
        "GitHub",
        RegistryConfigurationGitHubRegistry
      >
    | BaseRegistryConfigurationTypeMapping<
        "Custom",
        RegistryConfigurationCustomRegistry
      >
  );

export type PlatformDescriptor = BasePlatformDescriptor &
  (
    | BasePlatformDescriptorTypeMapping<
        "Docker",
        PlatformDescriptorDockerPlatformDescriptor
      >
    | BasePlatformDescriptorTypeMapping<
        "DockerSwarm",
        PlatformDescriptorDockerSwarmPlatformDescriptor
      >
    | BasePlatformDescriptorTypeMapping<
        "Kubernetes",
        PlatformDescriptorKubernetesPlatformDescriptor
      >
  );

export type IImageRepository = BaseIImageRepository &
  (
    | BaseIImageRepositoryTypeMapping<
        "GitHub",
        IImageRepositoryGitHubPackageResponse
      >
    | BaseIImageRepositoryTypeMapping<
        "DockerHub",
        IImageRepositoryDockerHubRepositoryResponse
      >
  );

export type GitAuthConfiguration = BaseGitAuthConfiguration &
  (
    | BaseGitAuthConfigurationTypeMapping<
        "Basic",
        GitAuthConfigurationBasicAuth
      >
    | BaseGitAuthConfigurationTypeMapping<
        "Token",
        GitAuthConfigurationTokenAuth
      >
    | BaseGitAuthConfigurationTypeMapping<
        "SshKey",
        GitAuthConfigurationSshKeyAuth
      >
  );

export type DeploymentImageInfo = BaseDeploymentImageInfo &
  (
    | BaseDeploymentImageInfoTypeMapping<"Local", DeploymentImageInfoLocalImage>
    | BaseDeploymentImageInfoTypeMapping<
        "External",
        DeploymentImageInfoExternalImage
      >
    | BaseDeploymentImageInfoTypeMapping<"Build", DeploymentImageInfoBuildImage>
  );

export type BuildAgentPoolProviderSpec = BaseBuildAgentPoolProviderSpec &
  (
    | BaseBuildAgentPoolProviderSpecTypeMapping<
        "AwsEc2",
        BuildAgentPoolProviderSpecAwsEc2BuildAgentPoolProviderSpec
      >
    | BaseBuildAgentPoolProviderSpecTypeMapping<
        "SelfManagedVm",
        BuildAgentPoolProviderSpecSelfManagedVmBuildAgentPoolProviderSpec
      >
  );

export type BackupSourceSpec = BaseBackupSourceSpec &
  (
    | BaseBackupSourceSpecTypeMapping<
        "DockerVolume",
        BackupSourceSpecDockerVolumeBackupSource
      >
    | BaseBackupSourceSpecTypeMapping<
        "CitadelSystem",
        BackupSourceSpecCitadelSystemBackupSource
      >
    | BaseBackupSourceSpecTypeMapping<
        "Stack",
        BackupSourceSpecStackBackupSource
      >
    | BaseBackupSourceSpecTypeMapping<
        "Deployment",
        BackupSourceSpecDeploymentBackupSource
      >
  );

export type BackupRepositorySpec = BaseBackupRepositorySpec &
  (
    | BaseBackupRepositorySpecTypeMapping<
        "FileSystem",
        BackupRepositorySpecFileSystemBackupRepositorySpec
      >
    | BaseBackupRepositorySpecTypeMapping<
        "S3Compatible",
        BackupRepositorySpecS3CompatibleBackupRepositorySpec
      >
  );

export type AlertRuleQuietHour = BaseAlertRuleQuietHour &
  (
    | BaseAlertRuleQuietHourTypeMapping<
        "Daily",
        AlertRuleQuietHourDailyQuietHour
      >
    | BaseAlertRuleQuietHourTypeMapping<
        "Weekly",
        AlertRuleQuietHourWeeklyQuietHour
      >
  );

export type AlertEventInfo = BaseAlertEventInfo &
  (
    | BaseAlertEventInfoTypeMapping<
        "PlatformCpuHigh",
        AlertEventInfoPlatformCpuHighAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "PlatformRamHigh",
        AlertEventInfoPlatformRamHighAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "PlatformUnreachable",
        AlertEventInfoPlatformUnreachableAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "UnmanagedContainerCreated",
        AlertEventInfoUnmanagedContainerCreatedAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "PlatformVersionMismatch",
        AlertEventInfoPlatformVersionMismatchAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "DeploymentImageUpdateAvailable",
        AlertEventInfoDeploymentImageUpdateAvailableAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "DeploymentAutoUpdated",
        AlertEventInfoDeploymentAutoUpdatedAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "DeploymentAutoDeployFailed",
        AlertEventInfoDeploymentAutoDeployFailedAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "StackImageUpdateAvailable",
        AlertEventInfoStackImageUpdateAvailableAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "StackAutoUpdated",
        AlertEventInfoStackAutoUpdatedAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "StackAutoDeployFailed",
        AlertEventInfoStackDeployFailedAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "StackServiceAutoUpdated",
        AlertEventInfoStackServiceAutoUpdatedAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "StackServiceAutoDeployFailed",
        AlertEventInfoStackServiceAutoDeployFailedAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "StackDriftDetected",
        AlertEventInfoStackDriftDetectedAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "StackDriftAutoReconciled",
        AlertEventInfoStackDriftAutoReconciledAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "StackGitUpdateAvailable",
        AlertEventInfoStackGitUpdateAvailableAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "StackGitAutoUpdated",
        AlertEventInfoStackGitAutoUpdatedAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "StackGitAutoDeployFailed",
        AlertEventInfoStackGitAutoDeployFailedAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "StackConfigurationResolutionFailed",
        AlertEventInfoStackConfigurationResolutionFailedAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "DeploymentConfigurationResolutionFailed",
        AlertEventInfoDeploymentConfigurationResolutionFailedAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "WebhookAuthenticationFailed",
        AlertEventInfoWebhookAuthenticationFailedAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "WebhookDispatchFailed",
        AlertEventInfoWebhookDispatchFailedAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "WebhookGitRepoSyncFailed",
        AlertEventInfoWebhookGitRepoSyncFailedAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "WebhookStackGitDeployFailed",
        AlertEventInfoWebhookStackGitDeployFailedAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "AutomationActionRunFailed",
        AlertEventInfoAutomationActionRunFailedAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "LicenseEnteredGracePeriod",
        AlertEventInfoLicenseEnteredGracePeriodAlertInfo
      >
    | BaseAlertEventInfoTypeMapping<
        "LicenseExpired",
        AlertEventInfoLicenseExpiredAlertInfo
      >
  );

export type ActivityEventInfo = BaseActivityEventInfo &
  (
    | BaseActivityEventInfoTypeMapping<
        "DeploymentCreated",
        ActivityEventInfoDeploymentCreated
      >
    | BaseActivityEventInfoTypeMapping<
        "DeploymentDuplicated",
        ActivityEventInfoDeploymentDuplicated
      >
    | BaseActivityEventInfoTypeMapping<
        "DeploymentUpdated",
        ActivityEventInfoDeploymentUpdated
      >
    | BaseActivityEventInfoTypeMapping<
        "DeploymentRenamed",
        ActivityEventInfoDeploymentRenamed
      >
    | BaseActivityEventInfoTypeMapping<
        "DeploymentDeleted",
        ActivityEventInfoDeploymentDeleted
      >
    | BaseActivityEventInfoTypeMapping<
        "DeploymentStarted",
        ActivityEventInfoDeploymentStarted
      >
    | BaseActivityEventInfoTypeMapping<
        "DeploymentStopped",
        ActivityEventInfoDeploymentStopped
      >
    | BaseActivityEventInfoTypeMapping<
        "DeploymentPaused",
        ActivityEventInfoDeploymentPaused
      >
    | BaseActivityEventInfoTypeMapping<
        "DeploymentApplied",
        ActivityEventInfoDeploymentApplied
      >
    | BaseActivityEventInfoTypeMapping<
        "DeploymentDegraded",
        ActivityEventInfoDeploymentDegraded
      >
    | BaseActivityEventInfoTypeMapping<
        "StackCreated",
        ActivityEventInfoStackCreated
      >
    | BaseActivityEventInfoTypeMapping<
        "StackDuplicated",
        ActivityEventInfoStackDuplicated
      >
    | BaseActivityEventInfoTypeMapping<
        "StackUpdated",
        ActivityEventInfoStackUpdated
      >
    | BaseActivityEventInfoTypeMapping<
        "StackRenamed",
        ActivityEventInfoStackRenamed
      >
    | BaseActivityEventInfoTypeMapping<
        "StackDeleted",
        ActivityEventInfoStackDeleted
      >
    | BaseActivityEventInfoTypeMapping<
        "StackStarted",
        ActivityEventInfoStackStarted
      >
    | BaseActivityEventInfoTypeMapping<
        "StackStopped",
        ActivityEventInfoStackStopped
      >
    | BaseActivityEventInfoTypeMapping<
        "StackPaused",
        ActivityEventInfoStackPaused
      >
    | BaseActivityEventInfoTypeMapping<
        "StackApplied",
        ActivityEventInfoStackApplied
      >
    | BaseActivityEventInfoTypeMapping<
        "StackRollback",
        ActivityEventInfoStackRollback
      >
    | BaseActivityEventInfoTypeMapping<
        "StackDegraded",
        ActivityEventInfoStackDegraded
      >
    | BaseActivityEventInfoTypeMapping<
        "StackDriftDetected",
        ActivityEventInfoStackDriftDetected
      >
    | BaseActivityEventInfoTypeMapping<
        "StackDriftResolved",
        ActivityEventInfoStackDriftResolved
      >
    | BaseActivityEventInfoTypeMapping<
        "StackReconciliationAttempted",
        ActivityEventInfoStackReconciliationAttempted
      >
    | BaseActivityEventInfoTypeMapping<
        "StackGitUpdateAvailable",
        ActivityEventInfoStackGitUpdateAvailable
      >
    | BaseActivityEventInfoTypeMapping<
        "StackGitAutoUpdated",
        ActivityEventInfoStackGitAutoUpdated
      >
    | BaseActivityEventInfoTypeMapping<
        "StackGitAutoDeployFailed",
        ActivityEventInfoStackGitAutoDeployFailed
      >
    | BaseActivityEventInfoTypeMapping<
        "AlertRuleCreated",
        ActivityEventInfoAlertRuleCreated
      >
    | BaseActivityEventInfoTypeMapping<
        "AlertRuleUpdated",
        ActivityEventInfoAlertRuleUpdated
      >
    | BaseActivityEventInfoTypeMapping<
        "AlertRuleDeleted",
        ActivityEventInfoAlertRuleDeleted
      >
    | BaseActivityEventInfoTypeMapping<
        "AlertRuleRenamed",
        ActivityEventInfoAlertRuleRenamed
      >
    | BaseActivityEventInfoTypeMapping<
        "PlatformCreated",
        ActivityEventInfoPlatformCreated
      >
    | BaseActivityEventInfoTypeMapping<
        "PlatformDeleted",
        ActivityEventInfoPlatformDeleted
      >
    | BaseActivityEventInfoTypeMapping<
        "PlatformConnected",
        ActivityEventInfoPlatformConnected
      >
    | BaseActivityEventInfoTypeMapping<
        "PlatformDisconnected",
        ActivityEventInfoPlatformDisconnected
      >
    | BaseActivityEventInfoTypeMapping<
        "PlatformRenamed",
        ActivityEventInfoPlatformRenamed
      >
    | BaseActivityEventInfoTypeMapping<
        "RegistryRenamed",
        ActivityEventInfoRegistryRenamed
      >
    | BaseActivityEventInfoTypeMapping<
        "RegistryCreated",
        ActivityEventInfoRegistryCreated
      >
    | BaseActivityEventInfoTypeMapping<
        "RegistryUpdated",
        ActivityEventInfoRegistryUpdated
      >
    | BaseActivityEventInfoTypeMapping<
        "RegistryDeleted",
        ActivityEventInfoRegistryDeleted
      >
    | BaseActivityEventInfoTypeMapping<
        "GitRepoCreated",
        ActivityEventInfoGitRepoCreated
      >
    | BaseActivityEventInfoTypeMapping<
        "GitRepoUpdated",
        ActivityEventInfoGitRepoUpdated
      >
    | BaseActivityEventInfoTypeMapping<
        "GitRepoRenamed",
        ActivityEventInfoGitRepoRenamed
      >
    | BaseActivityEventInfoTypeMapping<
        "GitRepoDeleted",
        ActivityEventInfoGitRepoDeleted
      >
    | BaseActivityEventInfoTypeMapping<
        "GitRepoCloned",
        ActivityEventInfoGitRepoCloned
      >
    | BaseActivityEventInfoTypeMapping<
        "GitRepoPulled",
        ActivityEventInfoGitRepoPulled
      >
    | BaseActivityEventInfoTypeMapping<
        "GitRepoWebhookReceived",
        ActivityEventInfoGitRepoWebhookReceived
      >
    | BaseActivityEventInfoTypeMapping<
        "OidcProviderCreated",
        ActivityEventInfoOidcProviderCreated
      >
    | BaseActivityEventInfoTypeMapping<
        "OidcProviderUpdated",
        ActivityEventInfoOidcProviderUpdated
      >
    | BaseActivityEventInfoTypeMapping<
        "OidcProviderRenamed",
        ActivityEventInfoOidcProviderRenamed
      >
    | BaseActivityEventInfoTypeMapping<
        "OidcProviderDeleted",
        ActivityEventInfoOidcProviderDeleted
      >
    | BaseActivityEventInfoTypeMapping<
        "ActionCreated",
        ActivityEventInfoAutomationActionCreated
      >
    | BaseActivityEventInfoTypeMapping<
        "ActionUpdated",
        ActivityEventInfoAutomationActionUpdated
      >
    | BaseActivityEventInfoTypeMapping<
        "ActionRenamed",
        ActivityEventInfoAutomationActionRenamed
      >
    | BaseActivityEventInfoTypeMapping<
        "ActionDeleted",
        ActivityEventInfoAutomationActionDeleted
      >
    | BaseActivityEventInfoTypeMapping<
        "ActionRunQueued",
        ActivityEventInfoAutomationActionRunQueued
      >
    | BaseActivityEventInfoTypeMapping<
        "ActionRunStarted",
        ActivityEventInfoAutomationActionRunStarted
      >
    | BaseActivityEventInfoTypeMapping<
        "ActionRunSucceeded",
        ActivityEventInfoAutomationActionRunSucceeded
      >
    | BaseActivityEventInfoTypeMapping<
        "ActionRunFailed",
        ActivityEventInfoAutomationActionRunFailed
      >
    | BaseActivityEventInfoTypeMapping<
        "ActionRunTimedOut",
        ActivityEventInfoAutomationActionRunTimedOut
      >
    | BaseActivityEventInfoTypeMapping<
        "ActionRunCancelled",
        ActivityEventInfoAutomationActionRunCancelled
      >
    | BaseActivityEventInfoTypeMapping<
        "ActionRunRejected",
        ActivityEventInfoAutomationActionRunRejected
      >
    | BaseActivityEventInfoTypeMapping<
        "StackWebhookReceived",
        ActivityEventInfoStackWebhookReceived
      >
    | BaseActivityEventInfoTypeMapping<
        "UserProfileUpdated",
        ActivityEventInfoUserProfileUpdated
      >
    | BaseActivityEventInfoTypeMapping<
        "UserPreferencesUpdated",
        ActivityEventInfoUserPreferencesUpdated
      >
    | BaseActivityEventInfoTypeMapping<
        "UserPasswordChanged",
        ActivityEventInfoUserPasswordChanged
      >
    | BaseActivityEventInfoTypeMapping<
        "UserSessionRevoked",
        ActivityEventInfoUserSessionRevoked
      >
    | BaseActivityEventInfoTypeMapping<
        "UserOtherSessionsRevoked",
        ActivityEventInfoUserOtherSessionsRevoked
      >
    | BaseActivityEventInfoTypeMapping<
        "UserMfaEnabled",
        ActivityEventInfoUserMfaEnabled
      >
    | BaseActivityEventInfoTypeMapping<
        "UserMfaDisabled",
        ActivityEventInfoUserMfaDisabled
      >
    | BaseActivityEventInfoTypeMapping<
        "UserMfaVerificationFailed",
        ActivityEventInfoUserMfaVerificationFailed
      >
    | BaseActivityEventInfoTypeMapping<
        "UserMfaRecoveryCodeUsed",
        ActivityEventInfoUserMfaRecoveryCodeUsed
      >
    | BaseActivityEventInfoTypeMapping<
        "UserMfaRecoveryCodesRegenerated",
        ActivityEventInfoUserMfaRecoveryCodesRegenerated
      >
    | BaseActivityEventInfoTypeMapping<
        "UserMfaResetByAdministrator",
        ActivityEventInfoUserMfaResetByAdministrator
      >
    | BaseActivityEventInfoTypeMapping<
        "LicenseInstalled",
        ActivityEventInfoLicenseInstalled
      >
    | BaseActivityEventInfoTypeMapping<
        "LicenseReplaced",
        ActivityEventInfoLicenseReplaced
      >
    | BaseActivityEventInfoTypeMapping<
        "LicenseRemoved",
        ActivityEventInfoLicenseRemoved
      >
    | BaseActivityEventInfoTypeMapping<
        "LicenseEnteredGracePeriod",
        ActivityEventInfoLicenseEnteredGracePeriod
      >
    | BaseActivityEventInfoTypeMapping<
        "LicenseExpired",
        ActivityEventInfoLicenseExpired
      >
    | BaseActivityEventInfoTypeMapping<
        "LicenseValidationFailed",
        ActivityEventInfoLicenseValidationFailed
      >
    | BaseActivityEventInfoTypeMapping<
        "VolumeContentDownloaded",
        ActivityEventInfoVolumeContentDownloaded
      >
    | BaseActivityEventInfoTypeMapping<
        "BuildCreated",
        ActivityEventInfoBuildCreated
      >
    | BaseActivityEventInfoTypeMapping<
        "BuildUpdated",
        ActivityEventInfoBuildUpdated
      >
    | BaseActivityEventInfoTypeMapping<
        "BuildRenamed",
        ActivityEventInfoBuildRenamed
      >
    | BaseActivityEventInfoTypeMapping<
        "BuildDeleted",
        ActivityEventInfoBuildDeleted
      >
    | BaseActivityEventInfoTypeMapping<
        "BuildRunQueued",
        ActivityEventInfoBuildRunQueued
      >
    | BaseActivityEventInfoTypeMapping<
        "BuildRunStarted",
        ActivityEventInfoBuildRunStarted
      >
    | BaseActivityEventInfoTypeMapping<
        "BuildRunSucceeded",
        ActivityEventInfoBuildRunSucceeded
      >
    | BaseActivityEventInfoTypeMapping<
        "BuildRunFailed",
        ActivityEventInfoBuildRunFailed
      >
    | BaseActivityEventInfoTypeMapping<
        "BuildRunTimedOut",
        ActivityEventInfoBuildRunTimedOut
      >
    | BaseActivityEventInfoTypeMapping<
        "BuildRunCancelled",
        ActivityEventInfoBuildRunCancelled
      >
    | BaseActivityEventInfoTypeMapping<
        "BuildWebhookReceived",
        ActivityEventInfoBuildWebhookReceived
      >
    | BaseActivityEventInfoTypeMapping<
        "BuildAgentPoolCreated",
        ActivityEventInfoBuildAgentPoolCreated
      >
    | BaseActivityEventInfoTypeMapping<
        "BuildAgentPoolUpdated",
        ActivityEventInfoBuildAgentPoolUpdated
      >
    | BaseActivityEventInfoTypeMapping<
        "BuildAgentPoolRenamed",
        ActivityEventInfoBuildAgentPoolRenamed
      >
    | BaseActivityEventInfoTypeMapping<
        "BuildAgentPoolDeleted",
        ActivityEventInfoBuildAgentPoolDeleted
      >
    | BaseActivityEventInfoTypeMapping<
        "BuildAgentPoolTested",
        ActivityEventInfoBuildAgentPoolTested
      >
  );

export interface AcknowledgeAlertEventsInput {
  ids: string[];
}

export interface ActivitiesView {
  pagedResult: PagedResultViewOfActivityView;
}

export interface ActivityChangedField {
  name: string;
  oldValue: null | string;
  newValue: null | string;
}

export interface ActivityEventInfoAlertRuleCreated {
  $type?: "AlertRuleCreated";
  alertRule: AlertRuleSnapshot;
}

export interface ActivityEventInfoAlertRuleDeleted {
  $type?: "AlertRuleDeleted";
  alertRule: AlertRuleSnapshot;
}

export interface ActivityEventInfoAlertRuleRenamed {
  $type?: "AlertRuleRenamed";
  oldName: string;
  newName: string;
}

export interface ActivityEventInfoAlertRuleUpdated {
  $type?: "AlertRuleUpdated";
  oldRule: AlertRuleSnapshot;
  newRule: AlertRuleSnapshot;
}

export interface ActivityEventInfoAutomationActionCreated {
  $type?: "ActionCreated";
  action: AutomationActionSnapshot;
}

export interface ActivityEventInfoAutomationActionDeleted {
  $type?: "ActionDeleted";
  action: AutomationActionSnapshot;
}

export interface ActivityEventInfoAutomationActionRenamed {
  $type?: "ActionRenamed";
  oldName: string;
  newName: string;
}

export interface ActivityEventInfoAutomationActionRunCancelled {
  $type?: "ActionRunCancelled";
  /** @format uuid */
  runId: string;
  trigger: ActionRunTrigger;
}

export interface ActivityEventInfoAutomationActionRunFailed {
  $type?: "ActionRunFailed";
  /** @format uuid */
  runId: string;
  trigger: ActionRunTrigger;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  exitCode: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  durationMs: null | number | string;
  errorMessage: null | string;
}

export interface ActivityEventInfoAutomationActionRunQueued {
  $type?: "ActionRunQueued";
  /** @format uuid */
  runId: string;
  trigger: ActionRunTrigger;
}

export interface ActivityEventInfoAutomationActionRunRejected {
  $type?: "ActionRunRejected";
  /** @format uuid */
  runId: string;
  trigger: ActionRunTrigger;
  reason: string;
}

export interface ActivityEventInfoAutomationActionRunStarted {
  $type?: "ActionRunStarted";
  /** @format uuid */
  runId: string;
  trigger: ActionRunTrigger;
}

export interface ActivityEventInfoAutomationActionRunSucceeded {
  $type?: "ActionRunSucceeded";
  /** @format uuid */
  runId: string;
  trigger: ActionRunTrigger;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  exitCode: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  durationMs: null | number | string;
}

export interface ActivityEventInfoAutomationActionRunTimedOut {
  $type?: "ActionRunTimedOut";
  /** @format uuid */
  runId: string;
  trigger: ActionRunTrigger;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  durationMs: null | number | string;
  errorMessage: null | string;
}

export interface ActivityEventInfoAutomationActionUpdated {
  $type?: "ActionUpdated";
  oldAction: AutomationActionSnapshot;
  newAction: AutomationActionSnapshot;
}

export interface ActivityEventInfoBuildAgentPoolCreated {
  $type?: "BuildAgentPoolCreated";
  pool: BuildAgentPoolSnapshot;
}

export interface ActivityEventInfoBuildAgentPoolDeleted {
  $type?: "BuildAgentPoolDeleted";
  pool: BuildAgentPoolSnapshot;
}

export interface ActivityEventInfoBuildAgentPoolRenamed {
  $type?: "BuildAgentPoolRenamed";
  oldName: string;
  newName: string;
}

export interface ActivityEventInfoBuildAgentPoolTested {
  $type?: "BuildAgentPoolTested";
  pool: BuildAgentPoolSnapshot;
  status: BuildAgentPoolValidationStatus;
  message: null | string;
}

export interface ActivityEventInfoBuildAgentPoolUpdated {
  $type?: "BuildAgentPoolUpdated";
  oldPool: BuildAgentPoolSnapshot;
  newPool: BuildAgentPoolSnapshot;
}

export interface ActivityEventInfoBuildCreated {
  $type?: "BuildCreated";
  build: BuildProjectSnapshot;
}

export interface ActivityEventInfoBuildDeleted {
  $type?: "BuildDeleted";
  build: BuildProjectSnapshot;
}

export interface ActivityEventInfoBuildRenamed {
  $type?: "BuildRenamed";
  oldName: string;
  newName: string;
}

export interface ActivityEventInfoBuildRunCancelled {
  $type?: "BuildRunCancelled";
  /** @format uuid */
  runId: string;
  trigger: BuildRunTrigger;
}

export interface ActivityEventInfoBuildRunFailed {
  $type?: "BuildRunFailed";
  /** @format uuid */
  runId: string;
  trigger: BuildRunTrigger;
  status: BuildRunStatus;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  exitCode: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  durationMs: null | number | string;
  errorMessage: null | string;
}

export interface ActivityEventInfoBuildRunQueued {
  $type?: "BuildRunQueued";
  /** @format uuid */
  runId: string;
  trigger: BuildRunTrigger;
}

export interface ActivityEventInfoBuildRunStarted {
  $type?: "BuildRunStarted";
  /** @format uuid */
  runId: string;
  trigger: BuildRunTrigger;
}

export interface ActivityEventInfoBuildRunSucceeded {
  $type?: "BuildRunSucceeded";
  /** @format uuid */
  runId: string;
  trigger: BuildRunTrigger;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  exitCode: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  durationMs: null | number | string;
  imageDigest: null | string;
}

export interface ActivityEventInfoBuildRunTimedOut {
  $type?: "BuildRunTimedOut";
  /** @format uuid */
  runId: string;
  trigger: BuildRunTrigger;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  durationMs: null | number | string;
  errorMessage: null | string;
}

export interface ActivityEventInfoBuildUpdated {
  $type?: "BuildUpdated";
  oldBuild: BuildProjectSnapshot;
  newBuild: BuildProjectSnapshot;
}

export interface ActivityEventInfoBuildWebhookReceived {
  $type?: "BuildWebhookReceived";
  /** @format uuid */
  requestId: string;
  authType: string;
  execution: string;
  status: string;
  reason: null | string;
  eventType: null | string;
  deliveryId: null | string;
  branch: null | string;
  commitSha: null | string;
  repositoryFullName: null | string;
  dispatchedBranch?: null | string;
  dispatchedCommitSha?: null | string;
}

export interface ActivityEventInfoDeploymentApplied {
  $type?: "DeploymentApplied";
  deployment: null | DeploymentSnapshot;
  result: DeploymentResultSnapshot;
}

export interface ActivityEventInfoDeploymentCreated {
  $type?: "DeploymentCreated";
  deployment: DeploymentSnapshot;
}

export interface ActivityEventInfoDeploymentDegraded {
  $type?: "DeploymentDegraded";
  reason: string;
}

export interface ActivityEventInfoDeploymentDeleted {
  $type?: "DeploymentDeleted";
  deployment: DeploymentSnapshot;
}

export interface ActivityEventInfoDeploymentDuplicated {
  $type?: "DeploymentDuplicated";
  deployment: DeploymentSnapshot;
  source: ActivitySourceResource;
}

export interface ActivityEventInfoDeploymentPaused {
  $type?: "DeploymentPaused";
  containerIds: string[];
}

export interface ActivityEventInfoDeploymentRenamed {
  $type?: "DeploymentRenamed";
  oldName: string;
  newName: string;
}

export interface ActivityEventInfoDeploymentStarted {
  $type?: "DeploymentStarted";
  containerIds: string[];
}

export interface ActivityEventInfoDeploymentStopped {
  $type?: "DeploymentStopped";
  containerIds: string[];
}

export interface ActivityEventInfoDeploymentUpdated {
  $type?: "DeploymentUpdated";
  oldDeployment: DeploymentSnapshot;
  newDeployment: DeploymentSnapshot;
}

export interface ActivityEventInfoGitRepoCloned {
  $type?: "GitRepoCloned";
  gitRepo: GitRepositorySnapshot;
  result: RepoSyncResultSnapshot;
}

export interface ActivityEventInfoGitRepoCreated {
  $type?: "GitRepoCreated";
  gitRepo: GitRepositorySnapshot;
}

export interface ActivityEventInfoGitRepoDeleted {
  $type?: "GitRepoDeleted";
  gitRepo: GitRepositorySnapshot;
}

export interface ActivityEventInfoGitRepoPulled {
  $type?: "GitRepoPulled";
  gitRepo: GitRepositorySnapshot;
  result: RepoSyncResultSnapshot;
}

export interface ActivityEventInfoGitRepoRenamed {
  $type?: "GitRepoRenamed";
  oldName: string;
  newName: string;
}

export interface ActivityEventInfoGitRepoUpdated {
  $type?: "GitRepoUpdated";
  oldGitRepo: GitRepositorySnapshot;
  newGitRepo: GitRepositorySnapshot;
}

export interface ActivityEventInfoGitRepoWebhookReceived {
  $type?: "GitRepoWebhookReceived";
  /** @format uuid */
  requestId: string;
  authType: string;
  execution: string;
  status: string;
  reason: null | string;
  eventType: null | string;
  deliveryId: null | string;
  branch: null | string;
  commitSha: null | string;
  repositoryFullName: null | string;
  dispatchedBranch?: null | string;
  dispatchedCommitSha?: null | string;
}

export interface ActivityEventInfoLicenseEnteredGracePeriod {
  $type?: "LicenseEnteredGracePeriod";
  license: LicenseActivitySnapshot;
}

export interface ActivityEventInfoLicenseExpired {
  $type?: "LicenseExpired";
  license: LicenseActivitySnapshot;
}

export interface ActivityEventInfoLicenseInstalled {
  $type?: "LicenseInstalled";
  license: LicenseActivitySnapshot;
}

export interface ActivityEventInfoLicenseRemoved {
  $type?: "LicenseRemoved";
  license: LicenseActivitySnapshot;
}

export interface ActivityEventInfoLicenseReplaced {
  $type?: "LicenseReplaced";
  oldLicense: LicenseActivitySnapshot;
  newLicense: LicenseActivitySnapshot;
}

export interface ActivityEventInfoLicenseValidationFailed {
  $type?: "LicenseValidationFailed";
  fingerprint: null | string;
  status: LicenseStatus;
  errorCode: null | string;
}

export interface ActivityEventInfoOidcProviderCreated {
  $type?: "OidcProviderCreated";
  provider: OidcProviderActivitySnapshot;
}

export interface ActivityEventInfoOidcProviderDeleted {
  $type?: "OidcProviderDeleted";
  provider: OidcProviderActivitySnapshot;
}

export interface ActivityEventInfoOidcProviderRenamed {
  $type?: "OidcProviderRenamed";
  oldName: string;
  newName: string;
}

export interface ActivityEventInfoOidcProviderUpdated {
  $type?: "OidcProviderUpdated";
  oldProvider: OidcProviderActivitySnapshot;
  newProvider: OidcProviderActivitySnapshot;
}

export interface ActivityEventInfoPlatformConnected {
  $type?: "PlatformConnected";
  platform: PlatformSnapshot;
  previousStatus: PlatformStatus;
}

export interface ActivityEventInfoPlatformCreated {
  $type?: "PlatformCreated";
  platform: PlatformSnapshot;
}

export interface ActivityEventInfoPlatformDeleted {
  $type?: "PlatformDeleted";
  platform: PlatformSnapshot;
}

export interface ActivityEventInfoPlatformDisconnected {
  $type?: "PlatformDisconnected";
  platform: PlatformSnapshot;
  previousStatus: PlatformStatus;
}

export interface ActivityEventInfoPlatformRenamed {
  $type?: "PlatformRenamed";
  oldName: string;
  newName: string;
}

export interface ActivityEventInfoRegistryCreated {
  $type?: "RegistryCreated";
  registry: RegistrySnapshot;
}

export interface ActivityEventInfoRegistryDeleted {
  $type?: "RegistryDeleted";
  registry: RegistrySnapshot;
}

export interface ActivityEventInfoRegistryRenamed {
  $type?: "RegistryRenamed";
  oldName: string;
  newName: string;
}

export interface ActivityEventInfoRegistryUpdated {
  $type?: "RegistryUpdated";
  oldRegistry: RegistrySnapshot;
  newRegistry: RegistrySnapshot;
}

export interface ActivityEventInfoStackApplied {
  $type?: "StackApplied";
  stack: null | StackSnapshot;
  result: StackResultSnapshot;
}

export interface ActivityEventInfoStackCreated {
  $type?: "StackCreated";
  stack: StackSnapshot;
}

export interface ActivityEventInfoStackDegraded {
  $type?: "StackDegraded";
  reason: string;
}

export interface ActivityEventInfoStackDeleted {
  $type?: "StackDeleted";
  stack: StackSnapshot;
}

export interface ActivityEventInfoStackDriftDetected {
  $type?: "StackDriftDetected";
  reason: string;
  fingerprint: string;
}

export interface ActivityEventInfoStackDriftResolved {
  $type?: "StackDriftResolved";
  previousFingerprint: string;
}

export interface ActivityEventInfoStackDuplicated {
  $type?: "StackDuplicated";
  stack: StackSnapshot;
  source: ActivitySourceResource;
}

export interface ActivityEventInfoStackGitAutoDeployFailed {
  $type?: "StackGitAutoDeployFailed";
  gitRepositoryName: string;
  branch: string;
  currentCommitSha: string;
  remoteCommitSha: string;
  reason: string;
}

export interface ActivityEventInfoStackGitAutoUpdated {
  $type?: "StackGitAutoUpdated";
  gitRepositoryName: string;
  branch: string;
  previousCommitSha: string;
  updatedCommitSha: string;
}

export interface ActivityEventInfoStackGitUpdateAvailable {
  $type?: "StackGitUpdateAvailable";
  gitRepositoryName: string;
  branch: string;
  currentCommitSha: string;
  remoteCommitSha: string;
}

export interface ActivityEventInfoStackPaused {
  $type?: "StackPaused";
  containerIds: string[];
}

export interface ActivityEventInfoStackReconciliationAttempted {
  $type?: "StackReconciliationAttempted";
  status: StackReconciliationStatus;
  actions: StackReconciliationAction[];
  driftFingerprint: string;
}

export interface ActivityEventInfoStackRenamed {
  $type?: "StackRenamed";
  oldName: string;
  newName: string;
}

export interface ActivityEventInfoStackRollback {
  $type?: "StackRollback";
  oldStack: null | StackSnapshot;
  newStack: null | StackSnapshot;
  result: StackResultSnapshot;
}

export interface ActivityEventInfoStackStarted {
  $type?: "StackStarted";
  containerIds: string[];
}

export interface ActivityEventInfoStackStopped {
  $type?: "StackStopped";
  containerIds: string[];
}

export interface ActivityEventInfoStackUpdated {
  $type?: "StackUpdated";
  oldStack: StackSnapshot;
  newStack: StackSnapshot;
}

export interface ActivityEventInfoStackWebhookReceived {
  $type?: "StackWebhookReceived";
  /** @format uuid */
  requestId: string;
  authType: string;
  execution: string;
  status: string;
  reason: null | string;
  eventType: null | string;
  deliveryId: null | string;
  branch: null | string;
  commitSha: null | string;
  repositoryFullName: null | string;
  dispatchedBranch?: null | string;
  dispatchedCommitSha?: null | string;
}

export interface ActivityEventInfoUserMfaDisabled {
  $type?: "UserMfaDisabled";
}

export interface ActivityEventInfoUserMfaEnabled {
  $type?: "UserMfaEnabled";
}

export interface ActivityEventInfoUserMfaRecoveryCodeUsed {
  $type?: "UserMfaRecoveryCodeUsed";
}

export interface ActivityEventInfoUserMfaRecoveryCodesRegenerated {
  $type?: "UserMfaRecoveryCodesRegenerated";
}

export interface ActivityEventInfoUserMfaResetByAdministrator {
  $type?: "UserMfaResetByAdministrator";
  /** @format uuid */
  targetUserId: string;
}

export interface ActivityEventInfoUserMfaVerificationFailed {
  $type?: "UserMfaVerificationFailed";
}

export interface ActivityEventInfoUserOtherSessionsRevoked {
  $type?: "UserOtherSessionsRevoked";
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  count: number | string;
}

export interface ActivityEventInfoUserPasswordChanged {
  $type?: "UserPasswordChanged";
}

export interface ActivityEventInfoUserPreferencesUpdated {
  $type?: "UserPreferencesUpdated";
  changes: ActivityChangedField[];
}

export interface ActivityEventInfoUserProfileUpdated {
  $type?: "UserProfileUpdated";
  changes: ActivityChangedField[];
}

export interface ActivityEventInfoUserSessionRevoked {
  $type?: "UserSessionRevoked";
  /** @format uuid */
  sessionId: string;
}

export interface ActivityEventInfoVolumeContentDownloaded {
  $type?: "VolumeContentDownloaded";
  volumeName: string;
  path: string;
  isDirectory: boolean;
  fileName: string;
}

export interface ActivitySourceResource {
  resourceType: ActivityResourceType;
  /** @format uuid */
  resourceId: string;
  resourceName: string;
}

export interface ActivityView {
  /** @format uuid */
  id: string;
  /** @format uuid */
  platformId: null | string;
  /** @format uuid */
  resourceId: null | string;
  platformName: string;
  resourceName: string;
  platformStatus: PlatformStatus;
  resourceType: ActivityResourceType;
  eventType: ActivityEventType;
  status: ActivityStatus;
  /** @format date-time */
  createdAt: any;
  info: ActivityEventInfo;
  /** @format uuid */
  actorId: string;
  actorName: string;
  actorType: ActorType;
}

export interface ActorView {
  /** @format uuid */
  id: string;
  name: string;
  type: ActorType;
  isEnabled: boolean;
}

export interface AddTeamMemberInput {
  /** @format uuid */
  userId: string;
}

export interface AddTeamResourceAccessInput {
  resourceType: ResourceType;
  /** @format uuid */
  resourceId: string;
  permissionLevel: PermissionLevel;
  specificPermissions: null | SpecificPermission[];
}

export interface AddTeamRoleInput {
  /** @format uuid */
  roleId: string;
}

export interface AddUserResourceAccessInput {
  resourceType: ResourceType;
  /** @format uuid */
  resourceId: string;
  permissionLevel: PermissionLevel;
  specificPermissions: null | SpecificPermission[];
}

export interface AddUserRoleInput {
  /** @format uuid */
  roleId: string;
}

export interface AgentSetupView {
  hubPublicKey: string;
  environment: Record<string, string>;
  agentImage: string;
  dockerRunCommand: string;
}

export interface AlertChannelInput {
  name: string;
  alertDestination: AlertDestination;
  url: string;
  isActive: boolean;
}

export interface AlertChannelView {
  /** @format uuid */
  id: string;
  name: string;
  alertDestination: AlertDestination;
  url: string;
  isActive: boolean;
  /** @format uuid */
  createdByActorId: string;
  /** @format date-time */
  createdAt: any;
}

export interface AlertChannelsView {
  channels: AlertChannelView[];
  capabilities: ResourceCapabilities;
}

export interface AlertEventInfoAutomationActionRunFailedAlertInfo {
  $type?: "AutomationActionRunFailed";
  actionName: string;
  /** @format uuid */
  runId: string;
  trigger: ActionRunTrigger;
  status: ActionRunStatus;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  exitCode: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  durationMs: null | number | string;
  reason: string;
  humanMessage?: null | string;
}

export interface AlertEventInfoDeploymentAutoDeployFailedAlertInfo {
  $type?: "DeploymentAutoDeployFailed";
  deploymentName: string;
  reason: string;
  humanMessage?: null | string;
}

export interface AlertEventInfoDeploymentAutoUpdatedAlertInfo {
  $type?: "DeploymentAutoUpdated";
  deploymentName: string;
  previousImage: string;
  updatedImage: string;
  humanMessage?: null | string;
}

export interface AlertEventInfoDeploymentConfigurationResolutionFailedAlertInfo {
  $type?: "DeploymentConfigurationResolutionFailed";
  deploymentName: string;
  reason: string;
  humanMessage?: null | string;
}

export interface AlertEventInfoDeploymentImageUpdateAvailableAlertInfo {
  $type?: "DeploymentImageUpdateAvailable";
  deploymentName: string;
  currentImage: string;
  latestImage: string;
  humanMessage?: null | string;
}

export interface AlertEventInfoLicenseEnteredGracePeriodAlertInfo {
  $type?: "LicenseEnteredGracePeriod";
  licenseId: null | string;
  customerName: null | string;
  fingerprint: null | string;
  /** @format date-time */
  expiresAt: any;
  /** @format date-time */
  graceUntil: any;
  humanMessage?: null | string;
}

export interface AlertEventInfoLicenseExpiredAlertInfo {
  $type?: "LicenseExpired";
  licenseId: null | string;
  customerName: null | string;
  fingerprint: null | string;
  /** @format date-time */
  expiresAt: any;
  /** @format date-time */
  graceUntil: any;
  humanMessage?: null | string;
}

export interface AlertEventInfoPlatformCpuHighAlertInfo {
  $type?: "PlatformCpuHigh";
  platformName: string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  cpuUsagePercent: number | string;
  humanMessage?: null | string;
}

export interface AlertEventInfoPlatformRamHighAlertInfo {
  $type?: "PlatformRamHigh";
  platformName: string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  ramUsagePercent: number | string;
  humanMessage?: null | string;
}

export interface AlertEventInfoPlatformUnreachableAlertInfo {
  $type?: "PlatformUnreachable";
  platformName: string;
  /** @format uuid */
  id: string;
  address: string;
  humanMessage?: null | string;
}

export interface AlertEventInfoPlatformVersionMismatchAlertInfo {
  $type?: "PlatformVersionMismatch";
  platformName: string;
  currentAgentVersion: string;
  expectedAgentVersion: string;
  humanMessage?: null | string;
}

export interface AlertEventInfoStackAutoUpdatedAlertInfo {
  $type?: "StackAutoUpdated";
  stackName: string;
  updates: StackImageUpdateItem[];
  humanMessage?: null | string;
}

export interface AlertEventInfoStackConfigurationResolutionFailedAlertInfo {
  $type?: "StackConfigurationResolutionFailed";
  stackName: string;
  reason: string;
  humanMessage?: null | string;
}

export interface AlertEventInfoStackDeployFailedAlertInfo {
  $type?: "StackAutoDeployFailed";
  stackName: string;
  reason: string;
  humanMessage?: null | string;
}

export interface AlertEventInfoStackDriftAutoReconciledAlertInfo {
  $type?: "StackDriftAutoReconciled";
  /** @format uuid */
  stackId: string;
  stackName: string;
  /** @format uuid */
  platformId: string;
  platformName: string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  driftCount: number | string;
  driftSummaries: string[];
  actions: StackReconciliationAction[];
  humanMessage?: null | string;
}

export interface AlertEventInfoStackDriftDetectedAlertInfo {
  $type?: "StackDriftDetected";
  /** @format uuid */
  stackId: string;
  stackName: string;
  /** @format uuid */
  platformId: string;
  platformName: string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  driftCount: number | string;
  hasAutoFixableDrift: boolean;
  hasStructuralDrift: boolean;
  driftSummaries: string[];
  humanMessage?: null | string;
}

export interface AlertEventInfoStackGitAutoDeployFailedAlertInfo {
  $type?: "StackGitAutoDeployFailed";
  stackName: string;
  gitRepositoryName: string;
  branch: string;
  currentCommitSha: string;
  remoteCommitSha: string;
  reason: string;
  humanMessage?: null | string;
}

export interface AlertEventInfoStackGitAutoUpdatedAlertInfo {
  $type?: "StackGitAutoUpdated";
  stackName: string;
  gitRepositoryName: string;
  branch: string;
  previousCommitSha: string;
  updatedCommitSha: string;
  humanMessage?: null | string;
}

export interface AlertEventInfoStackGitUpdateAvailableAlertInfo {
  $type?: "StackGitUpdateAvailable";
  stackName: string;
  gitRepositoryName: string;
  branch: string;
  currentCommitSha: string;
  remoteCommitSha: string;
  humanMessage?: null | string;
}

export interface AlertEventInfoStackImageUpdateAvailableAlertInfo {
  $type?: "StackImageUpdateAvailable";
  stackName: string;
  updates: StackImageUpdateItem[];
  humanMessage?: null | string;
}

export interface AlertEventInfoStackServiceAutoDeployFailedAlertInfo {
  $type?: "StackServiceAutoDeployFailed";
  stackName: string;
  serviceNames: string[];
  reason: string;
  humanMessage?: null | string;
}

export interface AlertEventInfoStackServiceAutoUpdatedAlertInfo {
  $type?: "StackServiceAutoUpdated";
  stackName: string;
  updates: StackImageUpdateItem[];
  humanMessage?: null | string;
}

export interface AlertEventInfoUnmanagedContainerCreatedAlertInfo {
  $type?: "UnmanagedContainerCreated";
  platformName: string;
  platformAddress: string;
  containerName: string;
  containerId: string;
  humanMessage?: null | string;
}

export interface AlertEventInfoWebhookAuthenticationFailedAlertInfo {
  $type?: "WebhookAuthenticationFailed";
  humanMessage?: null | string;
  resourceName: string;
  resourceType: string;
  provider: string;
  execution: string;
  reason: string;
  /** @format uuid */
  requestId: string;
  eventType: null | string;
  deliveryId: null | string;
  branch: null | string;
  commitSha: null | string;
  repositoryFullName: null | string;
}

export interface AlertEventInfoWebhookDispatchFailedAlertInfo {
  $type?: "WebhookDispatchFailed";
  humanMessage?: null | string;
  resourceName: string;
  resourceType: string;
  provider: string;
  execution: string;
  reason: string;
  /** @format uuid */
  requestId: string;
  eventType: null | string;
  deliveryId: null | string;
  branch: null | string;
  commitSha: null | string;
  repositoryFullName: null | string;
}

export interface AlertEventInfoWebhookGitRepoSyncFailedAlertInfo {
  $type?: "WebhookGitRepoSyncFailed";
  gitRepositoryName: string;
  branch: string;
  reason: string;
  humanMessage?: null | string;
}

export interface AlertEventInfoWebhookStackGitDeployFailedAlertInfo {
  $type?: "WebhookStackGitDeployFailed";
  stackName: string;
  gitRepositoryName: string;
  branch: string;
  reason: string;
  humanMessage?: null | string;
}

export interface AlertEventView {
  /** @format uuid */
  id: string;
  /** @format uuid */
  alertRuleId: string;
  type: AlertType;
  severity: AlertSeverity;
  status: AlertEventStatus;
  message: string;
  info: AlertEventInfo;
  /** @format uuid */
  resourceId: null | string;
  resourceName: string;
  resourceType: AlertResourceType;
  resourcePath: null | string;
  /** @format uuid */
  acknowledgedByActorId: null | string;
  /** @format date-time */
  acknowledgedAt: any;
  /** @format uuid */
  resolvedByActorId: null | string;
  /** @format date-time */
  resolvedAt: any;
  /** @format uuid */
  actorId: null | string;
  actorName: null | string;
  actorType: null | ActorType;
  resolutionNote: null | string;
  /** @format date-time */
  createdAt: any;
  /** @format date-time */
  updatedAt: any;
}

export interface AlertEventsView {
  pagedResult: PagedResultViewOfAlertEventView;
}

export interface AlertRuleConfigView {
  /** @format uuid */
  id: string;
  name: string;
  description: null | string;
  type: AlertType;
  severity: AlertSeverity;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  cooldownSeconds: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  requiredMatches: null | number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  threshold: null | number | string;
  status: AlertRuleStatus;
  channelIds: string[];
  limitedTo: AlertRuleLimitedTo[];
  quietHours: AlertRuleQuietHour[];
}

export interface AlertRuleLimitedTo {
  resourceType: AlertResourceType;
  /** @format uuid */
  resourceId: string;
}

export interface AlertRuleQuietHourDailyQuietHour {
  $type?: "Daily";
  name: string;
  scheduleType?: ScheduleType;
  /** @format time */
  startTime: string;
  /** @format time */
  endTime: string;
  timezone: string;
  description: null | string;
  timeZoneInfo?: TimeZoneInfo;
}

export interface AlertRuleQuietHourWeeklyQuietHour {
  $type?: "Weekly";
  dayOfWeek: DayOfWeek;
  name: string;
  scheduleType?: ScheduleType;
  /** @format time */
  startTime: string;
  /** @format time */
  endTime: string;
  timezone: string;
  description: null | string;
  timeZoneInfo?: TimeZoneInfo;
}

export interface AlertRuleSnapshot {
  /** @format uuid */
  id: string;
  type: AlertType;
  severity: AlertSeverity;
  name: string;
  description: null | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  cooldownSeconds: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  requiredMatches: null | number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  threshold: null | number | string;
  status: AlertRuleStatus;
  channelIds: string[];
  limitedTo: AlertRuleLimitedTo[];
  quietHours: AlertRuleQuietHour[];
}

export interface AlertRuleView {
  /** @format uuid */
  id: string;
  name: string;
  description: null | string;
  type: AlertType;
  severity: AlertSeverity;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  cooldownSeconds: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  requiredMatches: null | number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  threshold: null | number | string;
  status: AlertRuleStatus;
  channels: AlertChannelView[];
  limitedTo: AlertRuleLimitedTo[];
  quietHours: AlertRuleQuietHour[];
  capabilities?: null | ResourceCapabilities;
}

export interface AlertRulesView {
  alertRules: AlertRuleView[];
  capabilities: ResourceCapabilities;
}

export interface ApplicationInfoView {
  name: string;
  version: string;
  informationalVersion: string;
}

export interface ApplyDeploymentInput {
  /** @format uuid */
  id: string;
  /** @default false */
  recreate?: null | boolean;
}

export interface ApplyStackInput {
  /** @format uuid */
  id: string;
  /** @default false */
  recreate?: null | boolean;
}

export interface AutoUpdateState {
  /** @format date-time */
  lastCheckedAt: any;
  status: AutoUpdateStatus;
  currentDigest?: null | string;
  remoteDigest?: null | string;
  lastError?: null | string;
}

export interface AutomationActionInput {
  name: string;
  description: null | string;
  code: string;
  defaultArgsJson: null | string;
  enabled: boolean;
  scheduleEnabled: boolean;
  scheduleCron: null | string;
  scheduleTimeZone: null | string;
  webhook: null | AutomationWebhookConfig;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  timeoutSeconds: null | number | string;
  alertOnFailure: boolean;
  /** @format uuid */
  runAsActorId: null | string;
  tagIds?: null | string[];
}

export interface AutomationActionRunLogsView {
  /** @format uuid */
  runId: string;
  logs: string;
}

export interface AutomationActionRunStreamError {
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  code: null | number | string;
  message: null | string;
}

export interface AutomationActionRunStreamItem {
  /** @format uuid */
  runId?: null | string;
  status?: null | string;
  stream?: null | string;
  progressMessage?: null | string;
  errorMessage?: null | string;
  error?: null | AutomationActionRunStreamError;
}

export interface AutomationActionRunView {
  /** @format uuid */
  id: string;
  /** @format uuid */
  actionId: string;
  actionName: string;
  trigger: ActionRunTrigger;
  status: ActionRunStatus;
  /** @format uuid */
  runAsActorId: string;
  /** @format uuid */
  triggeredByActorId: null | string;
  argsJson: string;
  codeSnapshot: null | string;
  codeHash: string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  timeoutSeconds: number | string;
  /** @format date-time */
  queuedAt: any;
  /** @format date-time */
  startedAt: any;
  /** @format date-time */
  finishedAt: any;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  durationMs: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  exitCode: null | number | string;
  logs: null | string;
  errorMessage: null | string;
}

export interface AutomationActionRunsView {
  runs: AutomationActionRunView[];
}

export interface AutomationActionSnapshot {
  /** @format uuid */
  id: string;
  name: string;
  description: null | string;
  code: string;
  defaultArgsJson: string;
  enabled: boolean;
  scheduleEnabled: boolean;
  scheduleCron: null | string;
  scheduleTimeZone: string;
  webhook: null | AutomationWebhookConfig;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  timeoutSeconds: number | string;
  alertOnFailure: boolean;
  /** @format uuid */
  runAsActorId: string;
}

export interface AutomationActionView {
  /** @format uuid */
  id: string;
  name: string;
  description: null | string;
  code: string;
  defaultArgsJson: string;
  enabled: boolean;
  scheduleEnabled: boolean;
  scheduleCron: null | string;
  scheduleTimeZone: string;
  webhook: null | AutomationWebhookConfig;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  timeoutSeconds: number | string;
  alertOnFailure: boolean;
  /** @format uuid */
  runAsActorId: string;
  /** @format date-time */
  lastScheduledRunAt: any;
  controlState: ResourceControlState;
  /** @format uuid */
  currentRunId: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  rowVersion: number | string;
  latestRun: null | AutomationActionRunView;
  /** @format uuid */
  createdByActorId: string;
  /** @format date-time */
  createdAt: any;
  /** @format date-time */
  updatedAt: any;
  tags: TagSummaryView[];
  capabilities?: null | ResourceCapabilities;
}

export interface AutomationActionsView {
  actions: AutomationActionView[];
  capabilities: ResourceCapabilities;
}

export interface AutomationWebhookConfig {
  /** @default false */
  enabled?: boolean;
  provider?: WebhookProvider;
  authScheme?: WebhookAuthScheme;
  secret?: null | string;
  branchFilter?: null | string;
}

export interface BackupAffectedContainer {
  dockerContainerId: string;
  name: string;
  originalState: ContainerStateStatus;
  stopAttempted: boolean;
  restartAttempted: boolean;
  restartSucceeded: boolean;
}

export interface BackupCoverageView {
  status: BackupCoverageStatus;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  policyCount: number | string;
  /** @format uuid */
  lastRunId: null | string;
  lastRunStatus: null | BackupRunStatus;
  /** @format date-time */
  lastRunAt: any;
  /** @format date-time */
  lastSuccessfulRunAt: any;
  /** @format date-time */
  nextRunAt: any;
}

export interface BackupEventsView {
  /** @format uuid */
  runId: string;
  events: string[];
}

export interface BackupLogsView {
  /** @format uuid */
  runId: string;
  logs: string;
}

export interface BackupPoliciesView {
  policies: BackupPolicyView[];
  capabilities: ResourceCapabilities;
}

export interface BackupPolicyInput {
  name: string;
  description: null | string;
  source: BackupSourceSpec;
  /** @format uuid */
  backupRepositoryId: string;
  /** @default true */
  enabled?: boolean;
  cron?: null | string;
  timeZone?: null | string;
  webhook?: null | BackupWebhookConfig;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  keepLastSuccessful?: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  timeoutSeconds?: null | number | string;
  /** @default true */
  alertOnFailure?: boolean;
  /** @format uuid */
  runAsActorId?: null | string;
  tagIds?: null | string[];
}

export interface BackupPolicyView {
  /** @format uuid */
  id: string;
  name: string;
  normalizedName: string;
  description: null | string;
  source: BackupSourceSpec;
  /** @format uuid */
  backupRepositoryId: string;
  enabled: boolean;
  cron: null | string;
  timeZone: null | string;
  webhook: null | BackupWebhookConfig;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  keepLastSuccessful: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  timeoutSeconds: number | string;
  alertOnFailure: boolean;
  /** @format uuid */
  runAsActorId: string;
  controlState: ResourceControlState;
  /** @format uuid */
  currentRunId: null | string;
  /** @format date-time */
  lastScheduledRunAt: any;
  /** @format date-time */
  firstSuccessfulRunAt: any;
  /** @format uuid */
  createdByActorId: string;
  /** @format date-time */
  createdAt: any;
  /** @format date-time */
  updatedAt: any;
  /** @format date-time */
  archivedAt: any;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  rowVersion: number | string;
  tags: TagSummaryView[];
  capabilities?: null | ResourceCapabilities;
}

export interface BackupRepositoriesView {
  repositories: BackupRepositoryView[];
  capabilities: ResourceCapabilities;
}

export interface BackupRepositoryInput {
  name: string;
  description: null | string;
  spec: BackupRepositorySpec;
  /** @format uuid */
  passwordSecretId: string;
}

export interface BackupRepositorySpecFileSystemBackupRepositorySpec {
  $type?: "FileSystem";
  location: BackupExecutionLocation;
  /** @format uuid */
  platformId: null | string;
  path: string;
  type?: BackupRepositoryType;
}

export interface BackupRepositorySpecS3CompatibleBackupRepositorySpec {
  $type?: "S3Compatible";
  /** @format uri */
  endpoint: string;
  bucket: string;
  prefix: null | string;
  region: null | string;
  bucketLookup: S3BucketLookup;
  /** @format uuid */
  accessKeySecretId: string;
  /** @format uuid */
  secretKeySecretId: string;
  /** @format uuid */
  sessionTokenSecretId: null | string;
  /** @default false */
  allowInsecureHttp?: boolean;
  type?: BackupRepositoryType;
}

export interface BackupRepositoryValidationView {
  /** @format uuid */
  id: string;
  /** @format uuid */
  backupRepositoryId: string;
  location: BackupExecutionLocation;
  /** @format uuid */
  platformId: null | string;
  status: BackupRepositoryValidationStatus;
  /** @format date-time */
  lastValidatedAt: any;
  lastErrorCode: null | string;
  lastErrorMessage: null | string;
}

export interface BackupRepositoryView {
  /** @format uuid */
  id: string;
  name: string;
  normalizedName: string;
  description: null | string;
  type: BackupRepositoryType;
  spec: BackupRepositorySpec;
  /** @format uuid */
  passwordSecretId: string;
  status: BackupRepositoryStatus;
  controlState: ResourceControlState;
  /** @format uuid */
  currentRunId: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  controlStartedAt: null | number | string;
  /** @format date-time */
  lastPrunedAt: any;
  /** @format date-time */
  lastCheckedAt: any;
  /** @format uuid */
  createdByActorId: string;
  /** @format date-time */
  createdAt: any;
  /** @format date-time */
  updatedAt: any;
  /** @format date-time */
  archivedAt: any;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  rowVersion: number | string;
  capabilities?: null | ResourceCapabilities;
}

export interface BackupRestoreRunStreamItem {
  /** @format uuid */
  restoreRunId: string;
  status: null | BackupRestoreStatus;
  message: null | string;
  stream?: null | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  exitCode?: null | number | string;
}

export interface BackupRestoreRunView {
  /** @format uuid */
  id: string;
  /** @format uuid */
  backupRunId: string;
  /** @format uuid */
  backupRepositoryId: string;
  status: BackupRestoreStatus;
  /** @format uuid */
  targetPlatformId: string;
  targetVolumeName: string;
  overwriteExisting: boolean;
  targetVolumeCreatedByCitadel: boolean;
  affectedContainers: BackupAffectedContainer[];
  warnings: BackupRunWarning[];
  /** @format date-time */
  queuedAt: any;
  /** @format date-time */
  startedAt: any;
  /** @format date-time */
  completedAt: any;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  exitCode: null | number | string;
  errorCode: null | string;
  errorMessage: null | string;
  /** @format uuid */
  triggeredByActorId: string;
}

export interface BackupRestoreRunsView {
  runs: BackupRestoreRunView[];
}

export interface BackupRunItemView {
  /** @format uuid */
  id: string;
  /** @format uuid */
  backupRunId: string;
  /** @format uuid */
  platformId: string;
  volumeName: string;
  status: BackupRunItemStatus;
  resticSnapshotId: null | string;
  parentSnapshotId: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  filesProcessed: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  bytesProcessed: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  bytesAdded: null | number | string;
  /** @format date-time */
  startedAt: any;
  /** @format date-time */
  completedAt: any;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  exitCode: null | number | string;
  errorCode: null | string;
  errorMessage: null | string;
}

export interface BackupRunStreamItem {
  /** @format uuid */
  runId: string;
  status: null | BackupRunStatus;
  message: null | string;
  stream?: null | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  exitCode?: null | number | string;
}

export interface BackupRunView {
  /** @format uuid */
  id: string;
  /** @format uuid */
  backupPolicyId: string;
  /** @format uuid */
  backupRepositoryId: string;
  policyNameSnapshot: string;
  sourceSnapshot: BackupSourceSpec;
  repositoryTypeSnapshot: BackupRepositoryType;
  trigger: BackupRunTrigger;
  /** @format uuid */
  triggerSourceId: null | string;
  status: BackupRunStatus;
  resticSnapshotId: null | string;
  parentSnapshotId: null | string;
  snapshotAvailability: BackupSnapshotAvailability;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  filesProcessed: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  bytesProcessed: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  bytesAdded: null | number | string;
  warnings: BackupRunWarning[];
  /** @format date-time */
  queuedAt: any;
  /** @format date-time */
  startedAt: any;
  /** @format date-time */
  completedAt: any;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  exitCode: null | number | string;
  errorCode: null | string;
  errorMessage: null | string;
  /** @format uuid */
  triggeredByActorId: string;
  items: BackupRunItemView[];
}

export interface BackupRunWarning {
  code: string;
  message: string;
}

export interface BackupRunsView {
  runs: BackupRunView[];
}

export interface BackupSourceSpecCitadelSystemBackupSource {
  $type?: "CitadelSystem";
  type?: BackupSourceType;
  stableKey?: null | string;
}

export interface BackupSourceSpecDeploymentBackupSource {
  $type?: "Deployment";
  /** @format uuid */
  deploymentId: string;
  type?: BackupSourceType;
  stableKey?: null | string;
}

export interface BackupSourceSpecDockerVolumeBackupSource {
  $type?: "DockerVolume";
  /** @format uuid */
  platformId: string;
  volumeName: string;
  consistency?: VolumeBackupConsistency;
  type?: BackupSourceType;
  stableKey?: null | string;
}

export interface BackupSourceSpecStackBackupSource {
  $type?: "Stack";
  /** @format uuid */
  stackId: string;
  type?: BackupSourceType;
  stableKey?: null | string;
}

export interface BackupWebhookConfig {
  /** @default false */
  enabled?: boolean;
  provider?: WebhookProvider;
  authScheme?: WebhookAuthScheme;
  secret?: null | string;
  branchFilter?: null | string;
}

export interface BindOptions {
  propagation: null | string;
  nonRecursive: null | boolean;
  createMountpoint: null | boolean;
  readOnlyNonRecursive: null | boolean;
  readOnlyForceRecursive: null | boolean;
}

export interface BuildAgentPoolInput {
  name: string;
  description: null | string;
  enabled: boolean;
  providerSpec: BuildAgentPoolProviderSpec;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  maxActiveBuilders: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  queueTimeoutSeconds: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  provisioningTimeoutSeconds: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  registrationTimeoutSeconds: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  heartbeatTimeoutSeconds: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  cleanupTimeoutSeconds: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  maximumInstanceLifetimeSeconds: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  failureRetentionMinutes: null | number | string;
  tagIds: null | string[];
}

export interface BuildAgentPoolProviderSpecAwsEc2BuildAgentPoolProviderSpec {
  $type?: "AwsEc2";
  region: string;
  instanceType: string;
  architecture: CpuArchitecture;
  amiId: string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  rootVolumeSizeGb: number | string;
  subnetId: string;
  securityGroupIds: string[];
  instanceProfileName: null | string;
  assignPublicIp: boolean;
  /** @format uuid */
  awsCredentialSecretId: null | string;
  assumeRoleArn: null | string;
  keyPairName: null | string;
  tags?: null | Record<string, string>;
  provider?: BuildAgentPoolProvider;
}

export interface BuildAgentPoolProviderSpecSelfManagedVmBuildAgentPoolProviderSpec {
  $type?: "SelfManagedVm";
  endpoint: null | string;
  architecture: CpuArchitecture;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  maxWorkers: number | string;
  /** @format uuid */
  registrationSecretId?: null | string;
  labels?: null | string[];
  connectionMode?: BuildAgentPoolConnectionMode;
  provider?: BuildAgentPoolProvider;
}

export interface BuildAgentPoolSnapshot {
  /** @format uuid */
  id: string;
  name: string;
  description: null | string;
  enabled: boolean;
  provider: BuildAgentPoolProvider;
  providerSpec: BuildAgentPoolProviderSpec;
  architecture: CpuArchitecture;
  region: string;
  instanceType: string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  maxActiveBuilders: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  queueTimeoutSeconds: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  provisioningTimeoutSeconds: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  registrationTimeoutSeconds: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  heartbeatTimeoutSeconds: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  cleanupTimeoutSeconds: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  maximumInstanceLifetimeSeconds: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  failureRetentionMinutes: number | string;
  lastValidationStatus: BuildAgentPoolValidationStatus;
  lastValidationMessage: null | string;
  /** @format date-time */
  lastValidatedAt: any;
}

export interface BuildAgentPoolView {
  /** @format uuid */
  id: string;
  name: string;
  normalizedName: string;
  description: null | string;
  enabled: boolean;
  provider: BuildAgentPoolProvider;
  providerSpec: BuildAgentPoolProviderSpec;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  maxActiveBuilders: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  queueTimeoutSeconds: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  provisioningTimeoutSeconds: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  registrationTimeoutSeconds: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  heartbeatTimeoutSeconds: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  cleanupTimeoutSeconds: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  maximumInstanceLifetimeSeconds: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  failureRetentionMinutes: number | string;
  lastValidationStatus: BuildAgentPoolValidationStatus;
  lastValidationMessage: null | string;
  /** @format date-time */
  lastValidatedAt: any;
  /** @format uuid */
  createdByActorId: string;
  /** @format date-time */
  createdAt: any;
  /** @format date-time */
  updatedAt: any;
  /** @format date-time */
  archivedAt: any;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  rowVersion: number | string;
  tags: TagSummaryView[];
  capabilities?: null | ResourceCapabilities;
}

export interface BuildAgentPoolsView {
  pools: BuildAgentPoolView[];
  capabilities: ResourceCapabilities;
}

export interface BuildArgSpec {
  name: string;
  value?: null | string;
  /** @format uuid */
  resourceBindingId?: null | string;
}

export interface BuildLogsView {
  /** @format uuid */
  runId: string;
  logs: BuildRunLogEntry[];
}

export interface BuildPlatformSnapshot {
  /** @format uuid */
  id: string;
  name: string;
  address: string;
  connectorType: PlatformConnectorType;
}

export interface BuildProjectInput {
  name: string;
  description: null | string;
  enabled: boolean;
  /** @format uuid */
  gitRepositoryId: string;
  branch: null | string;
  contextPath: null | string;
  dockerfilePath: null | string;
  target: null | string;
  buildArgs: null | BuildArgSpec[];
  buildSecrets: null | BuildSecretSpec[];
  /** @format uuid */
  platformId: null | string;
  /** @format uuid */
  registryId: string;
  imageRepository: string;
  tagTemplates: null | string[];
  webhook: null | BuildWebhookConfig;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  timeoutSeconds: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  retentionRunCount: null | number | string;
  tagIds?: null | string[];
  builderKind?: BuildProjectBuilderKind;
  /** @format uuid */
  buildAgentPoolId?: null | string;
}

export interface BuildProjectSnapshot {
  /** @format uuid */
  id: string;
  name: string;
  description: null | string;
  enabled: boolean;
  /** @format uuid */
  gitRepositoryId: string;
  branch: string;
  contextPath: string;
  dockerfilePath: string;
  target: null | string;
  builderKind: BuildProjectBuilderKind;
  /** @format uuid */
  platformId: null | string;
  /** @format uuid */
  buildAgentPoolId: null | string;
  /** @format uuid */
  registryId: string;
  imageRepository: string;
  tagTemplates: string[];
  webhook: null | BuildWebhookConfig;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  timeoutSeconds: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  retentionRunCount: number | string;
  buildSecrets?: null | BuildSecretSpec[];
}

export interface BuildProjectView {
  /** @format uuid */
  id: string;
  name: string;
  normalizedName: string;
  description: null | string;
  enabled: boolean;
  /** @format uuid */
  gitRepositoryId: string;
  branch: string;
  contextPath: string;
  dockerfilePath: string;
  target: null | string;
  buildArgs: BuildArgSpec[];
  buildSecrets: BuildSecretSpec[];
  builderKind: BuildProjectBuilderKind;
  /** @format uuid */
  platformId: null | string;
  /** @format uuid */
  buildAgentPoolId: null | string;
  /** @format uuid */
  registryId: string;
  imageRepository: string;
  tagTemplates: string[];
  webhook: null | BuildWebhookConfig;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  timeoutSeconds: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  retentionRunCount: number | string;
  /** @format uuid */
  currentRunId: null | string;
  controlState: ResourceControlState;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  controlStartedAt: null | number | string;
  /** @format uuid */
  createdByActorId: string;
  /** @format date-time */
  createdAt: any;
  /** @format date-time */
  updatedAt: any;
  /** @format date-time */
  archivedAt: any;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  rowVersion: number | string;
  latestRun: null | BuildRunView;
  tags: TagSummaryView[];
  capabilities?: null | ResourceCapabilities;
}

export interface BuildProjectsView {
  projects: BuildProjectView[];
  capabilities: ResourceCapabilities;
}

export interface BuildRegistrySnapshot {
  /** @format uuid */
  id: string;
  name: string;
  registryHost: string;
}

export interface BuildRunLogEntry {
  /** @format uuid */
  id: string;
  /** @format uuid */
  buildRunId: string;
  /** @format date-time */
  createdAt: any;
  stream: string;
  message: string;
}

export interface BuildRunView {
  /** @format uuid */
  id: string;
  /** @format uuid */
  buildProjectId: string;
  projectNameSnapshot: string;
  /** @format uuid */
  gitRepositoryId: string;
  gitRepositoryNameSnapshot: string;
  branch: string;
  resolvedCommitSha: null | string;
  contextPath: string;
  dockerfilePath: string;
  target: null | string;
  buildArgsSnapshot: BuildArgSpec[];
  buildSecretIdsSnapshot: string[];
  platformSnapshot: BuildPlatformSnapshot;
  registrySnapshot: BuildRegistrySnapshot;
  imageRepository: string;
  tagTemplatesSnapshot: string[];
  imageReferences: string[];
  trigger: BuildRunTrigger;
  /** @format uuid */
  triggerSourceId: null | string;
  status: BuildRunStatus;
  imageDigest: null | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  timeoutSeconds: number | string;
  /** @format date-time */
  queuedAt: any;
  /** @format date-time */
  startedAt: any;
  /** @format date-time */
  completedAt: any;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  exitCode: null | number | string;
  errorCode: null | string;
  errorMessage: null | string;
  /** @format uuid */
  triggeredByActorId: string;
}

export interface BuildRunsView {
  runs: BuildRunView[];
}

export interface BuildSecretSpec {
  id: string;
  /** @format uuid */
  secretId: string;
}

export interface BuildWebhookConfig {
  /** @default false */
  enabled?: boolean;
  provider?: WebhookProvider;
  authScheme?: WebhookAuthScheme;
  secret?: null | string;
  branchFilter?: null | string;
}

export interface ChangeCurrentPasswordInput {
  currentPassword: string;
  newPassword: string;
}

export interface ClusterVolume {
  id: string;
  version: null | VolumeVersionInfo;
  createdAt: string;
  updatedAt: string;
  spec: null | VolumeSpecification;
  info: null | ClusterVolumeInfo;
  publishStatus: VolumePublishStatus[];
}

export interface ClusterVolumeInfo {
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  capacityBytes: null | number | string;
  volumeContext: Record<string, string>;
  volumeID: string;
  accessibleTopology: TopologyEntry[];
}

export interface ConfigFromInput {
  network: string;
}

export interface ConfirmMandatoryMfaSetupInput {
  code: string;
}

export interface ConfirmProfileMfaSetupInput {
  code: string;
}

export interface ContainerConfiguration {
  hostname: null | string;
  domainname: null | string;
  user: null | string;
  attachStdin: null | boolean;
  attachStdout: null | boolean;
  attachStderr: null | boolean;
  exposedPorts: null | string[];
  tty: null | boolean;
  openStdin: null | boolean;
  stdinOnce: null | boolean;
  env: string[];
  cmd: string[];
  image: null | string;
  volumes: null | string[];
  workingDir: null | string;
  entrypoint: string[];
  networkDisabled: null | boolean;
  macAddress: null | string;
  onBuild: string[];
  labels: Record<string, string>;
}

export interface ContainerDataView {
  name: string;
  image: string;
  id: string;
  imageId: string;
  state: ContainerStateStatus;
  controlState: ResourceControlState;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  created?: null | number | string;
  stack?: null | string;
  containerStat?: null | ContainerStatView;
  ports?: null | Record<string, HostPortBinding[]>;
  /** @format uuid */
  deploymentId?: null | string;
  /** @format uuid */
  stackId?: null | string;
  capabilities?: null | PlatformCapabilities;
}

export interface ContainerHealthStatus {
  status: null | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  failingStreak: null | number | string;
}

export interface ContainerImageResult {
  id: string;
  name: string;
  state: ContainerStateStatus;
  volumes: string[];
  networks: Record<string, string>;
  ports: Record<string, HostPortBinding[]>;
}

export interface ContainerInfoView {
  name: string;
  containerId: string;
  /** @format uuid */
  platformId: string;
  startedAt: string;
  finishedAt: string;
  platformName: string;
  volumes: string[];
  ports: Record<string, HostPortBinding[]>;
  networks: Record<string, string>;
  state: ContainerStateStatus;
  imageView: null | ImageView;
  deploymentView?: null | DeploymentView;
  capabilities?: null | PlatformCapabilities;
}

export interface ContainerInspectView {
  id: string;
  created: string;
  path: null | string;
  state: null | ContainerRuntimeState;
  image: null | string;
  resolvConfPath: null | string;
  hostnamePath: null | string;
  hostsPath: null | string;
  logPath: null | string;
  name: null | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  restartCount: null | number | string;
  driver: null | string;
  platform: null | string;
  mountLabel: null | string;
  processLabel: null | string;
  appArmorProfile: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  sizeRw: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  sizeRootFs: null | number | string;
  args: null | string[];
  execIDs: string[];
  mounts: null | MountPointInfo[];
  hostConfig: null | HostConfiguration;
  graphDriver: null | GraphDriverDataInfo;
  config: null | ContainerConfiguration;
  networkSettings: null | NetworkSettingsInfo;
}

export interface ContainerRuntimeState {
  status: ContainerStateStatus;
  running: null | boolean;
  paused: null | boolean;
  restarting: null | boolean;
  oomKilled: null | boolean;
  dead: null | boolean;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  pid: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  exitCode: null | number | string;
  error: null | string;
  startedAt: null | string;
  finishedAt: null | string;
  health: null | ContainerHealthStatus;
}

export interface ContainerStatView {
  /** @format uuid */
  containerId?: string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  memoryActive?: number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  memoryCache?: number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  cpuUsage?: number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  memoryLimit?: number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  rxBytes?: number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  txBytes?: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  created?: number | string;
}

export interface ContainerStatsView {
  stats: ContainerStatView[];
}

export interface ContainerView {
  /** @format uuid */
  id: string;
  /** @format uuid */
  platformId: string;
  containerId: string;
  name: string;
  dockerImageId: string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  created: number | string;
  state: ContainerStateStatus;
  controlState: ResourceControlState;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  updated: number | string;
  stack: null | string;
  lastStats: null | ContainerStatView;
  ports: Record<string, HostPortBinding[]>;
  /** @format uuid */
  deploymentId: null | string;
  /** @format uuid */
  stackId: null | string;
  platform?: null | PlatformView;
  imageView?: null | ImageView;
  deploymentView?: null | DeploymentView;
  capabilities?: null | PlatformCapabilities;
}

export interface ContainerVolumeResult {
  id: string;
  name: string;
  image: string;
  imageId: string;
  state: ContainerStateStatus;
  networks: Record<string, string>;
  ports: Record<string, HostPortBinding[]>;
}

export interface ContainersDataView {
  containers: ContainerDataView[];
}

export interface ContainersView {
  containers: ContainerView[];
  capabilities: PlatformCapabilities;
}

export interface CreateAlertRuleInput {
  name: null | string;
  description: null | string;
  type: AlertType;
  severity: AlertSeverity;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  cooldownSeconds: null | number | string;
  status: AlertRuleStatus;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  requiredMatches?: null | number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  threshold?: null | number | string;
  channelIds?: null | string[];
  limitedTo?: null | AlertRuleLimitedTo[];
  quietHours?: null | AlertRuleQuietHour[];
}

export interface CreateDeploymentInput {
  name: string;
  /** @format uuid */
  platformId: string;
  description: null | string;
  spec: DeploymentSpec;
  tagIds?: null | string[];
  duplicateSource?: null | DuplicateSourceInput;
}

export interface CreateExternalSecretInput {
  name: string;
  /** @format uuid */
  providerId: string;
  externalPath: string;
  externalKey: string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  externalVersion: null | number | string;
}

export interface CreateGitRepositoryInput {
  name: string;
  description: null | string;
  url: string;
  defaultBranch: string;
  /** @format uuid */
  gitAccountId: null | string;
  syncMode: GitRepositorySyncMode;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  syncIntervalMinutes: null | number | string;
  webhook: null | RepoWebhookConfig;
  onClone: null | RepoCommand;
  onPull: null | RepoCommand;
  tagIds?: null | string[];
}

export interface CreateInternalSecretInput {
  name: string;
  value: string;
}

export interface CreateNetworkInput {
  /** @format uuid */
  platformId: string;
  name: string;
  driver: string;
  scope: string;
  internal: null | boolean;
  attachable: null | boolean;
  ingress: null | boolean;
  enableIPv6: null | boolean;
  enableIPv4: null | boolean;
  configOnly: null | boolean;
  ipam?: null | IPAMInput;
  configFrom?: null | ConfigFromInput;
  labels?: null | Record<string, string>;
  options?: null | Record<string, string>;
}

export interface CreateNetworkView {
  id: string;
}

export interface CreatePlatformInput {
  name: string;
  address: null | string;
  description?: null | string;
  type?: PlatformType;
  connectorType?: PlatformConnectorType;
  tagIds?: null | string[];
}

export interface CreateRegistryInput {
  name: string;
  registryHost: string;
  status: RegistryStatus;
  configuration: RegistryConfiguration;
  description?: null | string;
  tagIds?: null | string[];
}

export interface CreateStackInput {
  name: string;
  /** @format uuid */
  platformId: string;
  description: null | string;
  stackSource: StackSource;
  spec: StackSpec;
  driftPolicy?: null | StackDriftPolicy;
  tagIds?: null | string[];
  duplicateSource?: null | DuplicateSourceInput;
}

export interface CreateTagInput {
  name: string;
  color: string;
}

export interface CreateTeamInput {
  name: string;
  userIds?: null | string[];
  roleIds?: null | string[];
  resourceAccesses?: null | TeamResourceAccessInput[];
}

export interface CreateUserInput {
  name: string;
  email: string;
  password: string;
  /** @default true */
  isEnabled?: boolean;
  teamIds?: null | string[];
  roleIds?: null | string[];
  resourceAccesses?: null | UserResourceAccessInput[];
}

export interface CreateVaultKvV2SecretProviderInput {
  name: string;
  address: string;
  mountPath: string;
  token: string;
}

export interface CreateVolumeInput {
  /** @format uuid */
  platformId: string;
  name: string;
  driver: string;
  labels?: null | Record<string, string>;
  options?: null | Record<string, string>;
}

export type CurrentProfileAuthenticationType = any;

export interface CurrentProfileAuthenticationView {
  type: CurrentProfileAuthenticationType;
  label: string;
  canChangePassword: boolean;
  canUseLocalPasswordMfa: boolean;
  /** @format uuid */
  oidcProviderId?: null | string;
  oidcProviderName?: null | string;
}

export interface CurrentProfileView {
  /** @format uuid */
  id: string;
  displayName: string;
  email: string;
  authentication: CurrentProfileAuthenticationView;
  /** @format date-time */
  createdAt: any;
  directRoles: ProfileResourceInfoView[];
  teams: ProfileResourceInfoView[];
}

export interface DeleteAlertChannelsInput {
  ids: string[];
}

export interface DeleteAlertRulesInput {
  ids: string[];
}

export interface DeleteContainersRequest {
  containerIds: string[];
  /** @default false */
  v?: null | boolean;
  /** @default false */
  force?: null | boolean;
  /** @default false */
  link?: null | boolean;
}

export interface DeleteGitAccountsInput {
  ids: string[];
}

export interface DeleteGitRepositoriesInput {
  ids: string[];
}

export interface DeleteImageResponseItem {
  result: Record<string, string>;
}

export interface DeleteImageResult {
  items: DeleteImageResponseItem[];
}

export interface DeleteImagesRequest {
  /** @format uuid */
  platformId: string;
  ids: string[];
  /** @default false */
  force?: boolean;
  /** @default false */
  noPrune?: boolean;
}

export interface DeleteNetworksInput {
  /** @format uuid */
  platformId: string;
  ids: string[];
}

export interface DeletePlatformsInput {
  ids: string[];
}

export interface DeleteRegistriesInput {
  ids: string[];
}

export interface DeleteRolesInput {
  ids: string[];
}

export interface DeleteTeamsInput {
  ids: string[];
}

export interface DeleteUsersInput {
  ids: string[];
}

export interface DeleteVolumesInput {
  /** @format uuid */
  platformId: string;
  names: string[];
  force: null | boolean;
}

export interface DeploymentApplyError {
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  code: null | number | string;
  message: null | string;
}

export interface DeploymentBackupSourcePreviewView {
  /** @format uuid */
  deploymentId: string;
  deploymentName: string;
  /** @format uuid */
  platformId: string;
  platformName: string;
  platformStatus: PlatformStatus;
  volumes: DeploymentBackupVolumeView[];
  warnings: string[];
}

export interface DeploymentBackupVolumeView {
  name: string;
  kind: StackVolumeKind;
  isExternal: boolean;
  isShared: boolean;
  hasBackupCoverage: boolean;
}

export interface DeploymentCapabilities {
  canViewLogs: boolean;
  canInspect: boolean;
  canOpenTerminal: boolean;
  canPull: boolean;
  canApply: boolean;
  canViewResourceBindings: boolean;
  canRead: boolean;
  canWrite: boolean;
  canExecute: boolean;
}

export interface DeploymentConfigView {
  /** @format uuid */
  id: string;
  name: string;
  /** @format uuid */
  platformId: string;
  description: null | string;
  spec: DeploymentSpec;
}

export interface DeploymentDuplicateDraftView {
  draft: CreateDeploymentInput;
  warnings: DuplicateDraftWarningView[];
}

export interface DeploymentImageInfoBuildImage {
  $type?: "Build";
  /** @format uuid */
  buildProjectId: string;
  /** @default false */
  redeployOnBuild?: boolean;
  resolvedImageReference?: null | string;
  resolvedDigest?: null | string;
}

export interface DeploymentImageInfoExternalImage {
  $type?: "External";
  /** @format uuid */
  registryId: string;
  imageTag: string;
  resolvedDigest?: null | string;
}

export interface DeploymentImageInfoLocalImage {
  $type?: "Local";
  imageId: string;
}

export interface DeploymentResultSnapshot {
  containerIds?: null | string[];
  message?: null | string;
  resourceBindings?: null | ResourceBindingSnapshot[];
}

export interface DeploymentSnapshot {
  /** @format uuid */
  id: string;
  name: string;
  /** @format uuid */
  platformId: string;
  description?: null | string;
  spec?: null | DeploymentSpec;
}

export interface DeploymentSpec {
  image: DeploymentImageInfo;
  updateBehavior: UpdateBehavior;
  lifeCycleSpec?: null | LifeCycleSpec;
  resourceSpec?: null | ResourceSpec;
  labels?: null | Record<string, string>;
  ports?: null | string[];
  volumes?: null | string[];
  networks?: null | string[];
  command?: null | string[];
  environmentVariables?: null | string[];
}

export interface DeploymentStreamItem {
  id?: null | string;
  status?: null | string;
  stream?: null | string;
  progressMessage?: null | string;
  errorMessage?: null | string;
  progress?: null | ImagePullProgress;
  error?: null | DeploymentApplyError;
}

export interface DeploymentView {
  /** @format uuid */
  id: string;
  name: string;
  description: null | string;
  /** @format uuid */
  platformId: string;
  /** @format date-time */
  createdAt: any;
  /** @format uuid */
  createdByActorId: string;
  status: DeploymentStatus;
  controlState: ResourceControlState;
  autoUpdateState: AutoUpdateState;
  spec: DeploymentSpec;
  platformStatus: PlatformStatus;
  platformName?: null | string;
  imageName?: null | string;
  /** @format uuid */
  imageId?: null | string;
  /** @format uuid */
  containerId?: null | string;
  dockerContainerId?: null | string;
  dockerImageId?: null | string;
  tags?: TagSummaryView[];
  latestActivityView?: null | LatestActivityView;
  capabilities?: null | DeploymentCapabilities;
}

export interface DeploymentsView {
  deployments: DeploymentView[];
  capabilities: ResourceCapabilities;
}

export interface DisableProfileMfaInput {
  password: string;
  code?: null | string;
  recoveryCode?: null | string;
}

export interface DockerHubImageView {
  architecture: string;
  digest: string;
  os: string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  size: number | string;
  status: DockerHubImageStatus;
  lastPulled: string;
}

export interface DockerHubRepositoryInfo {
  name: null | string;
  namespace: null | string;
  /** @format date-time */
  lastUpdated: any;
  isPrivate: boolean;
  isTrusted: boolean;
  isAutomated: boolean;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  pullCount: number | string;
}

export interface DockerHubTagView {
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  id: number | string;
  name: string;
  image: null | DockerHubImageView;
  lastUpdated: string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  fullSize: number | string;
  status: DockerHubTagStatus;
  lastPulled: string;
}

export interface DockerNetworkDetailsView {
  name: string;
  id: string;
  created: string;
  driver: string;
  scope: string;
  enableIPv4: boolean;
  enableIPv6: boolean;
  internal: boolean;
  attachable: boolean;
  ingress: boolean;
  configOnly: boolean;
  configFrom: null | string;
  ipam: null | IpAddressManagementConfig;
  options: Record<string, string>;
  labels: Record<string, string>;
  containers: Record<string, NetworkConnectedContainer>;
  peers: NetworkPeerInfo[];
  capabilities?: null | NetworkCapabilities;
}

export interface DockerNetworkResultView {
  name: string;
  id: string;
  created: string;
  driver: string;
  scope: string;
  enableIPv4: boolean;
  enableIPv6: boolean;
  internal: boolean;
  attachable: boolean;
  ingress: boolean;
  configOnly: boolean;
  inUse: boolean;
  configFrom: null | string;
  ipam: null | IpAddressManagementConfig;
  options: Record<string, string>;
  labels: Record<string, string>;
  capabilities?: null | NetworkCapabilities;
}

export interface DockerVolumeResultView {
  id: string;
  name: string;
  inUse: boolean;
  scope: string;
  driver: string;
  mountpoint: string;
  createdAt: string;
  clusterVolume: null | ClusterVolume;
  usageData: null | VolumeUsageData;
  containers: ContainerVolumeResult[];
  status: Record<string, string>;
  labels: Record<string, string>;
  options: Record<string, string>;
  backupCoverage?: null | BackupCoverageView;
  capabilities?: null | VolumeCapabilities;
}

export interface DriverConfiguration {
  name: null | string;
  options: Record<string, string>;
}

export interface DuplicateDraftWarningView {
  code: string;
  message: string;
  fieldPath?: null | string;
}

export interface DuplicateSourceInput {
  resourceType: ActivityResourceType;
  /** @format uuid */
  resourceId: string;
  resourceName: string;
}

export interface EdgeAgentEnrollmentInstructionsView {
  coreUrl: string;
  environment: Record<string, string>;
  agentImage: string;
  dockerRunCommand: string;
}

export interface EdgeAgentEnrollmentView {
  /** @format uuid */
  enrollmentId: string;
  /** @format uuid */
  platformId: string;
  token: string;
  /** @format date-time */
  expiresAtUtc: any;
  instructions: EdgeAgentEnrollmentInstructionsView;
}

export interface EdgeAgentStatusView {
  connectionStatus: string;
  /** @format date-time */
  lastConnectedAtUtc: any;
  /** @format date-time */
  lastDisconnectedAtUtc: any;
  /** @format date-time */
  lastHeartbeatAtUtc: any;
  lastSeenVersion: null | string;
  lastSeenHostname: null | string;
  agentFingerprint: null | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  protocolVersion: null | number | string;
  /** @format date-time */
  revokedAtUtc: any;
  /** @format date-time */
  enrollmentExpiresAtUtc: any;
}

export interface EndpointIpamConfiguration {
  ipv4Address: null | string;
  ipv6Address: null | string;
  linkLocalIPs: string[];
}

export interface EndpointSettingsInfo {
  ipamConfig: null | EndpointIpamConfiguration;
  links: string[];
  macAddress: null | string;
  aliases: string[];
  networkID: null | string;
  endpointID: null | string;
  gateway: null | string;
  ipAddress: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  ipPrefixLen: null | number | string;
  ipv6Gateway: null | string;
  globalIPv6Address: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  globalIPv6PrefixLen: null | number | string;
  driverOpts: Record<string, string>;
  dnsNames: string[];
}

export interface ExposedPortsResult {
  ports: string[];
}

export interface ExternalSecretTestResultView {
  success: boolean;
  message: string;
}

export interface GitAccountConfigView {
  /** @format uuid */
  id: string;
  name: string;
  domain: string;
  transport: GitTransport;
  authType: GitAuthType;
  configuration: null | GitAuthConfiguration;
}

export interface GitAccountInput {
  name: string;
  domain: string;
  transport: GitTransport;
  authType: GitAuthType;
  configuration: GitAuthConfiguration;
}

export interface GitAccountView {
  /** @format uuid */
  id: string;
  /** @format uuid */
  createdByActorId: string;
  name: string;
  domain: string;
  transport: GitTransport;
  authType: GitAuthType;
  /** @format date-time */
  createdAt: any;
  capabilities?: null | ResourceCapabilities;
}

export interface GitAccountsView {
  gitAccounts: GitAccountView[];
  capabilities: ResourceCapabilities;
}

export interface GitAuthConfigurationBasicAuth {
  $type?: "Basic";
  username: string;
  password: string;
}

export interface GitAuthConfigurationSshKeyAuth {
  $type?: "SshKey";
  username: string;
  privateKey: string;
  passphrase: null | string;
}

export interface GitAuthConfigurationTokenAuth {
  $type?: "Token";
  token: string;
}

export interface GitComposeProjectCandidate {
  workingDirectory: string;
  composePaths: string[];
  envFilePaths: string[];
  suggestedWatchPaths: string[];
}

export interface GitHubCrPackageVersion {
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  id: number | string;
  url: string;
  name: string;
  htmlUrl: null | string;
  createdAt: null | string;
  updatedAt: null | string;
  packageHtmlUrl: null | string;
  metadata: null | GitHubCrPackageVersionMetadata;
}

export interface GitHubCrPackageVersionContainerMetadata {
  tags: null | string[];
}

export interface GitHubCrPackageVersionMetadata {
  container: null | GitHubCrPackageVersionContainerMetadata;
}

export interface GitRepositoriesView {
  gitRepositories: GitRepositoryView[];
  capabilities: ResourceCapabilities;
}

export interface GitRepositoryBranchView {
  branch: string;
  commitSha: string;
}

export interface GitRepositoryBranchesView {
  branches: GitRepositoryBranchView[];
}

export interface GitRepositoryComposeDiscovery {
  /** @format uuid */
  repositoryId: string;
  branch: string;
  resolvedCommitSha: string;
  projects: GitComposeProjectCandidate[];
}

export interface GitRepositoryConfigView {
  /** @format uuid */
  id: string;
  name: string;
  description: null | string;
  url: string;
  defaultBranch: string;
  /** @format uuid */
  gitAccountId: null | string;
  syncMode: GitRepositorySyncMode;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  syncIntervalMinutes: null | number | string;
  webhook: null | RepoWebhookConfig;
  onClone: null | RepoCommand;
  onPull: null | RepoCommand;
}

export interface GitRepositoryRefView {
  /** @format uuid */
  id: string;
  /** @format uuid */
  gitRepositoryId: string;
  branch: string;
  resolvedCommitSha: null | string;
  status: GitReposStatus;
  lastError: null | string;
  /** @format date-time */
  lastSyncedAt: any;
}

export interface GitRepositoryRefsView {
  refs: GitRepositoryRefView[];
}

export interface GitRepositorySnapshot {
  /** @format uuid */
  id: string;
  name: string;
  description: null | string;
  url: string;
  defaultBranch: string;
  /** @format uuid */
  gitAccountId: null | string;
  syncMode: GitRepositorySyncMode;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  syncIntervalMinutes: null | number | string;
  webhook: null | RepoWebhookConfig;
  onClone: null | RepoCommand;
  onPull: null | RepoCommand;
  resolvedCommitSha?: null | string;
}

export interface GitRepositoryView {
  /** @format uuid */
  id: string;
  /** @format uuid */
  createdByActorId: string;
  name: string;
  description: null | string;
  status: GitReposStatus;
  url: string;
  defaultBranch: null | string;
  /** @format uuid */
  gitAccountId: null | string;
  syncMode: GitRepositorySyncMode;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  syncIntervalMinutes: null | number | string;
  webhook: null | RepoWebhookConfig;
  onClone: null | RepoCommand;
  onPull: null | RepoCommand;
  /** @format date-time */
  createdAt: any;
  controlState: ResourceControlState;
  latestActivityView: null | LatestActivityView;
  tags?: TagSummaryView[];
  capabilities?: null | ResourceCapabilities;
}

export interface GraphDriverDataInfo {
  name: null | string;
  data: Record<string, string>;
}

export interface HistoryImageResult {
  id: string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  created: number | string;
  createdBy: string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  size: number | string;
  comment: string;
}

export interface HostConfiguration {
  binds: string[];
  containerIDFile: null | string;
  logConfig: null | LogConfiguration;
  networkMode: null | string;
  portBindings: Record<string, HostPortBinding[]>;
  restartPolicy: null | RestartPolicy;
  autoRemove: null | boolean;
  volumeDriver: null | string;
  volumesFrom: string[];
  mounts: HostMount[];
  consoleSize: (number | string)[];
  annotations: Record<string, string>;
  capAdd: string[];
  capDrop: string[];
  cgroupnsMode: null | string;
  dns: string[];
  dnsOptions: string[];
  dnsSearch: string[];
  extraHosts: string[];
  groupAdd: string[];
  ipcMode: null | string;
  cgroup: null | string;
  links: string[];
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  oomScoreAdj: null | number | string;
  pidMode: null | string;
  privileged: null | boolean;
  publishAllPorts: null | boolean;
  readonlyRootfs: null | boolean;
  securityOpt: string[];
  storageOpt: Record<string, string>;
  tmpfs: Record<string, string>;
  utsMode: null | string;
  usernsMode: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  shmSize: number | string;
  sysctls: Record<string, string>;
  runtime: null | string;
  isolation: null | string;
  maskedPaths: string[];
  readonlyPaths: string[];
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  memorySwap: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  memorySwappiness: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  nanoCpus: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  pidsLimit: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  memory: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  memoryReservation: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  ioMaximumBandwidth: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  cpuPeriod: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  cpuPercent: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  cpuCount: null | number | string;
  ulimits: Ulimit[];
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  kernelMemoryTCP: null | number | string;
}

export interface HostMount {
  target: null | string;
  source: null | string;
  type: null | string;
  readOnly: null | boolean;
  consistency: null | string;
  bindOptions: null | BindOptions;
  volumeOptions: null | VolumeOptions;
}

export interface HostPortBinding {
  hostIP?: null | string;
  hostPort?: null | string;
}

export interface HttpValidationProblemDetails {
  type?: null | string;
  title?: null | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  status?: null | number | string;
  detail?: null | string;
  instance?: null | string;
  errors?: Record<string, string[]>;
}

export interface IImageRepositoryDockerHubRepositoryResponse {
  $type?: "DockerHub";
  name?: string;
  namespace?: null | string;
  /** @format date-time */
  lastUpdated?: any;
  isPrivate?: boolean;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  pullCount?: number | string;
}

export interface IImageRepositoryGitHubPackageResponse {
  $type?: "GitHub";
  id?: string;
  name?: string;
  createdAt?: null | string;
  updatedAt?: null | string;
  url?: null | string;
  htmlUrl?: null | string;
}

export interface IPAMConfigInput {
  subnet: string;
  ipRange: string;
  gateway: string;
}

export interface IPAMInput {
  driver: string;
  config?: null | IPAMConfigInput[];
  options?: null | Record<string, string>;
}

export interface ImageCapabilities {
  canInspect: boolean;
  canPull: boolean;
  canRead: boolean;
  canWrite: boolean;
  canExecute: boolean;
}

export interface ImagePullError {
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  code: null | number | string;
  message: null | string;
}

export interface ImagePullProgress {
  units: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  current: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  total: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  start: null | number | string;
}

export interface ImageUpdateState {
  serviceName: string;
  imageName: string;
  currentDigest: string;
  remoteDigest: null | string;
  /** @format date-time */
  lastCheckedAt: any;
  updateAvailable: boolean;
}

export interface ImageView {
  /** @format uuid */
  id: string;
  tags: string[];
  name: string;
  dockerImageId: string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  size: number | string;
  isInUse: boolean;
  /** @format uuid */
  platformId: string;
  /** @format date-time */
  createdAt: any;
  controlState: ResourceControlState;
  updatedAt?: any;
  /** @format uuid */
  registryId?: null | string;
  registry?: null | RegistryView;
  capabilities?: null | ImageCapabilities;
}

export interface ImagesView {
  images: ImageView[];
  capabilities: ImageCapabilities;
}

export interface InspectImageView {
  id: string;
  name: string;
  tag: string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  size: number | string;
  os: string;
  created: string;
  architecture: string;
  env: string[];
  cmd: string[];
  repoTags: string[];
  volumes: string[];
  exposedPorts: string[];
  layers: HistoryImageResult[];
  labels: Record<string, string>;
  containers: ContainerImageResult[];
  registry: null | RegistryView;
  capabilities?: null | ImageCapabilities;
}

export interface InstallLicenseInput {
  license: string;
}

export interface IpAddressInfo {
  addr: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  prefixLen: null | number | string;
}

export interface IpAddressManagementConfig {
  driver: null | string;
  config: IpamSubnetConfiguration[];
  options: Record<string, string>;
}

export interface IpamSubnetConfiguration {
  subnet: null | string;
  ipRange: null | string;
  gateway: null | string;
}

export interface LatestActivityView {
  /** @format uuid */
  id: string;
  resourceType: ActivityResourceType;
  eventType: ActivityEventType;
  status: ActivityStatus;
  info: ActivityEventInfo;
  /** @format date-time */
  createdAt: any;
}

export interface LicenseActivitySnapshot {
  licenseId: null | string;
  replacedLicenseId: null | string;
  edition: string;
  customerId: null | string;
  customerName: null | string;
  fingerprint: null | string;
  status: LicenseStatus;
  /** @format date-time */
  expiresAt: any;
  /** @format date-time */
  graceUntil: any;
}

export interface LicenseLimitView {
  limit: LicenseLimit;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  current: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  maximum: number | string;
  overQuota: boolean;
}

export interface LicenseRequestView {
  product: string;
  /** @format uuid */
  instanceId: string;
  coreVersion: string;
  /** @format date-time */
  generatedAt: any;
}

export interface LicenseView {
  status: LicenseStatus;
  edition: string;
  /** @format uuid */
  instanceId: string;
  licenseId: null | string;
  replacedLicenseId: null | string;
  customerId: null | string;
  customerName: null | string;
  fingerprint: null | string;
  /** @format date-time */
  issuedAt: any;
  /** @format date-time */
  notBefore: any;
  /** @format date-time */
  expiresAt: any;
  /** @format date-time */
  graceUntil: any;
  limits: LicenseLimitView[];
  warnings: string[];
}

export interface LifeCycleSpec {
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  stopTimeout: null | number | string;
  stopSignal: null | StopSignal;
  restartPolicy: ContainerRestartPolicy;
}

export interface LogConfiguration {
  type: null | string;
  config: Record<string, string>;
}

export interface LoginRequest {
  emailOrName: string;
  password: string;
}

export interface LoginResponse {
  accessToken: null | string;
  nextStep: LoginNextStep;
}

export interface MandatoryMfaSetupCompleteView {
  accessToken: string;
  recoveryCodes: string[];
}

export interface MandatoryMfaSetupView {
  secret: string;
  otpAuthUri: string;
  /** @format date-time */
  expiresAt: any;
}

export interface MfaVerificationInput {
  code?: null | string;
  recoveryCode?: null | string;
}

export interface MfaVerificationView {
  accessToken: string;
}

export interface MountPointInfo {
  type: null | string;
  name: null | string;
  source: null | string;
  destination: null | string;
  driver: null | string;
  mode: null | string;
  rw: null | boolean;
  propagation: null | string;
}

export interface NetworkCapabilities {
  canInspect: boolean;
  canRead: boolean;
  canWrite: boolean;
  canExecute: boolean;
}

export interface NetworkConnectedContainer {
  name: string;
  endpointId: string;
  macAddress: string;
  ipV4Address: string;
  ipv6Address: string;
}

export interface NetworkPeerInfo {
  name: string;
  ip: string;
}

export interface NetworkSettingsInfo {
  bridge: null | string;
  sandboxID: null | string;
  hairpinMode: null | boolean;
  linkLocalIPv6Address: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  linkLocalIPv6PrefixLen: null | number | string;
  sandboxKey: null | string;
  secondaryIPAddresses: IpAddressInfo[];
  secondaryIPv6Addresses: IpAddressInfo[];
  endpointID: null | string;
  gateway: null | string;
  globalIPv6Address: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  globalIPv6PrefixLen: null | number | string;
  ipAddress: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  ipPrefixLen: null | number | string;
  ipv6Gateway: null | string;
  macAddress: null | string;
  ports: Record<string, HostPortBinding[]>;
  networks: Record<string, EndpointSettingsInfo>;
}

export interface NetworksView {
  networks: DockerNetworkResultView[];
  capabilities: ResourceCapabilities;
}

export interface OidcDiscoveryResultView {
  issuer: string;
  authorizationEndpoint: string;
  tokenEndpoint: string;
  jwksUri: string;
}

export interface OidcLoginProviderView {
  /** @format uuid */
  id: string;
  displayName: string;
}

export interface OidcLoginProvidersView {
  providers: OidcLoginProviderView[];
}

export interface OidcProviderActivitySnapshot {
  /** @format uuid */
  id: string;
  name: string;
  description: null | string;
  displayName: string;
  issuer: string;
  clientId: string;
  scopes: string;
  enabled: boolean;
  autoProvisionUsers: boolean;
  allowEmailAutoLink: boolean;
  requireEmailVerified: boolean;
  allowedEmailDomains: null | string;
  requiredClaimName: null | string;
  requiredClaimValues: null | string;
  /** @format uuid */
  defaultRoleId: null | string;
}

export interface OidcProviderInput {
  name: string;
  description: null | string;
  displayName: string;
  issuer: string;
  clientId: string;
  clientSecret: null | string;
  scopes: null | string;
  enabled: boolean;
  autoProvisionUsers: boolean;
  allowEmailAutoLink: boolean;
  requireEmailVerified: boolean;
  allowedEmailDomains: null | string;
  requiredClaimName: null | string;
  requiredClaimValues: null | string;
  /** @format uuid */
  defaultRoleId: null | string;
}

export interface OidcProviderView {
  /** @format uuid */
  id: string;
  name: string;
  description: null | string;
  displayName: string;
  issuer: string;
  clientId: string;
  scopes: string;
  enabled: boolean;
  autoProvisionUsers: boolean;
  allowEmailAutoLink: boolean;
  requireEmailVerified: boolean;
  allowedEmailDomains: null | string;
  requiredClaimName: null | string;
  requiredClaimValues: null | string;
  /** @format uuid */
  defaultRoleId: null | string;
  hasClientSecret: boolean;
  /** @format uuid */
  createdByActorId: string;
  /** @format date-time */
  createdAt: any;
  /** @format date-time */
  updatedAt: any;
}

export interface OidcProvidersView {
  providers: OidcProviderView[];
}

export interface PagedResultViewOfActivityView {
  items: ActivityView[];
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  totalCount: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  page: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  pageSize: number | string;
}

export interface PagedResultViewOfAlertEventView {
  items: AlertEventView[];
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  totalCount: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  page: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  pageSize: number | string;
}

export interface PagedResultViewOfTeamView {
  items: TeamView[];
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  totalCount: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  page: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  pageSize: number | string;
}

export interface PagedResultViewOfUserView {
  items: UserView[];
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  totalCount: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  page: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  pageSize: number | string;
}

export interface PatchActorEnabledInput {
  isEnabled: boolean;
}

export interface PatchAlertRuleInput {
  description: null | string;
  type: AlertType;
  severity: AlertSeverity;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  cooldownSeconds: null | number | string;
  status: AlertRuleStatus;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  requiredMatches?: null | number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  threshold?: null | number | string;
  channelIds?: null | string[];
  limitedTo?: null | AlertRuleLimitedTo[];
  quietHours?: null | AlertRuleQuietHour[];
}

export interface PatchDeploymentInput {
  /** @format uuid */
  platformId: string;
  spec: DeploymentSpec;
}

export interface PatchGitRepositoryInput {
  url: string;
  defaultBranch: string;
  /** @format uuid */
  gitAccountId: null | string;
  syncMode: GitRepositorySyncMode;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  syncIntervalMinutes: null | number | string;
  webhook: null | RepoWebhookConfig;
  onClone: null | RepoCommand;
  onPull: null | RepoCommand;
}

export interface PatchRegistryInput {
  registryHost: string;
  status: RegistryStatus;
  configuration: RegistryConfiguration;
}

export interface PatchResourceMetadata {
  description: string;
  tags: string[];
}

export interface PatchRolePermissionsInput {
  permissions: PermissionInput[];
}

export interface PatchStackInput {
  /** @format uuid */
  platformId: string;
  spec: StackSpec;
  driftPolicy?: null | StackDriftPolicy;
}

export interface PatchTagInput {
  name: null | string;
  color: null | string;
}

export interface PatchTeamInput {
  isEnabled: null | boolean;
  userIds: null | string[];
  roleIds: null | string[];
  resourceAccesses: null | TeamResourceAccessInput[];
}

export interface PatchUserInput {
  email: null | string;
  password: null | string;
  isEnabled: null | boolean;
  teamIds: null | string[];
  roleIds: null | string[];
  resourceAccesses: null | UserResourceAccessInput[];
}

export interface PatchUserPreferencesInput {
  timeZone: null | string;
  dateTimeFormat: null | UserDateTimeFormat;
  theme: null | UserTheme;
}

export interface PermissionInput {
  resourceType: ResourceType;
  permissionLevel: PermissionLevel;
  specificPermissions: null | SpecificPermission[];
}

export interface PermissionMatrixViewItem {
  maximumLevel: string;
  specificPermissions: Record<string, string>;
  label: string;
  specificPermissionLabels: Record<string, string>;
}

export interface PermissionView {
  resourceType: ResourceType;
  permissionLevel: PermissionLevel;
  specificPermissions: SpecificPermission[];
}

export interface PlatformCapabilities {
  canViewLogs: boolean;
  canInspect: boolean;
  canOpenTerminal: boolean;
  canPull: boolean;
  canRead: boolean;
  canWrite: boolean;
  canExecute: boolean;
}

export interface PlatformDescriptorDockerPlatformDescriptor {
  $type?: "Docker";
  daemonId: string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  containerCount: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  containersRunning: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  containersPaused: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  containersStopped: number | string;
  driver?: null | string;
  operatingSystem?: null | string;
  osVersion?: null | string;
  osType?: null | string;
  architecture?: null | string;
}

export interface PlatformDescriptorDockerSwarmPlatformDescriptor {
  $type?: "DockerSwarm";
  nodeID: string;
  nodeAddr: string;
  localNodeState: string;
  controlAvailable: boolean;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  nodes: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  managers: number | string;
  error?: null | string;
  remoteManagers?: null | SwarmPeer[];
  daemonId: string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  containerCount: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  containersRunning: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  containersPaused: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  containersStopped: number | string;
  driver?: null | string;
  operatingSystem?: null | string;
  osVersion?: null | string;
  osType?: null | string;
  architecture?: null | string;
}

export interface PlatformDescriptorKubernetesPlatformDescriptor {
  $type?: "Kubernetes";
  clusterName: null | string;
  clusterVersion: null | string;
  apiServerUrl: null | string;
  namespace: null | string;
}

export interface PlatformInput {
  name: string;
  address: null | string;
  description?: null | string;
  type?: PlatformType;
  connectorType?: PlatformConnectorType;
}

export interface PlatformSnapshot {
  /** @format uuid */
  id: string;
  name: string;
  address: string;
  description: null | string;
  status: PlatformStatus;
  connectorType: PlatformConnectorType;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  networkCount: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  volumeCount: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  imageCount: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  cpuCount: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  memTotal: number | string;
  serverVersion: null | string;
  agentVersion: null | string;
  platformDescriptor: PlatformDescriptor;
}

export interface PlatformStatView {
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  created?: number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  txBytes?: number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  rxBytes?: number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  cpuUsage?: number | string;
  /**
   * @format double
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  memoryUsage?: number | string;
}

export interface PlatformStatsView {
  stats: PlatformStatView[];
}

export interface PlatformView {
  /** @format uuid */
  id: string;
  name: string;
  description: null | string;
  address: string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  networkCount: number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  volumeCount: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  imageCount: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  cpuCount: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  memTotal: number | string;
  agentVersion: null | string;
  serverVersion: null | string;
  type: PlatformType;
  status: PlatformStatus;
  connectorType: PlatformConnectorType;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  deploymentCount: number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  stackCount: number | string;
  stats: null | PlatformStatView[];
  platformDescriptor: null | PlatformDescriptor;
  tags?: TagSummaryView[];
  capabilities?: null | PlatformCapabilities;
}

export interface PlatformsView {
  platforms: PlatformView[];
  capabilities: ResourceCapabilities;
}

export interface ProblemDetails {
  type?: null | string;
  title?: null | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  status?: null | number | string;
  detail?: null | string;
  instance?: null | string;
}

export interface ProfileMfaRecoveryCodesView {
  enabled: boolean;
  recoveryCodes: string[];
}

export interface ProfileMfaSetupView {
  secret: string;
  otpAuthUri: string;
  /** @format date-time */
  expiresAt: any;
}

export interface ProfileMfaStatusView {
  enabled: boolean;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  remainingRecoveryCodes: number | string;
  policy: MfaPolicy;
  canDisable: boolean;
}

export interface ProfileResourceInfoView {
  /** @format uuid */
  id: string;
  name: string;
}

export interface PrunePlatformInput {
  resource: PruneResource;
}

export interface PrunePlatformView {
  resource: PruneResource;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  spaceReclaimed: number | string;
  volumesDeleted: string[];
  networksDeleted: string[];
  imagesDeleted: string[];
  buildCacheDeleted: string[];
}

export interface PullImageInput {
  /** @format uuid */
  platformId: string;
  /** @format uuid */
  registryId: string;
  imageTag: string;
}

export interface PullImageStreamItem {
  id?: null | string;
  from?: null | string;
  stream?: null | string;
  status?: null | string;
  errorMessage?: null | string;
  progressMessage?: null | string;
  dockerImageId?: null | string;
  digest?: null | string;
  progress?: null | ImagePullProgress;
  error?: null | ImagePullError;
}

export interface QueueBackupRunInput {
  trigger?: BackupRunTrigger;
  /** @format uuid */
  triggerSourceId?: null | string;
}

export interface QueueBuildRunInput {
  trigger?: BuildRunTrigger;
  /** @format uuid */
  triggerSourceId?: null | string;
}

export interface RecreateStackOnNewCommitState {
  currentCommitSha: string;
  remoteCommitSha: null | string;
  /** @format date-time */
  lastCheckedAt: any;
}

export interface RecreateStackOnNewImageState {
  autoUpdateStates: ImageUpdateState[];
}

export interface RefreshTokenResponse {
  accessToken: string;
}

export interface RegenerateProfileMfaRecoveryCodesInput {
  password: string;
  code: string;
}

export interface RegistriesView {
  registries: RegistryView[];
  capabilities: ResourceCapabilities;
}

export interface RegistryConfigView {
  /** @format uuid */
  id: string;
  name: string;
  registryHost: string;
  status: RegistryStatus;
  description: string;
  configuration: null | RegistryConfiguration;
  tags: TagSummaryView[];
}

export interface RegistryConfigurationAWSRegistry {
  $type?: "AWS";
  accessKey: string;
  authenticationRequired: boolean;
  secretAccessKey: string;
  region: string;
}

export interface RegistryConfigurationAzureRegistry {
  $type?: "Azure";
  userName: string;
  password: string;
}

export interface RegistryConfigurationCustomRegistry {
  $type?: "Custom";
  /** @default false */
  authEnabled?: null | boolean;
  userName?: null | string;
  password?: null | string;
}

export interface RegistryConfigurationDockerHubRegistry {
  $type?: "DockerHub";
  userName?: null | string;
  pat?: null | string;
}

export interface RegistryConfigurationGitHubRegistry {
  $type?: "GitHub";
  nameSpace: string;
  /** @default false */
  ghcrAuthEnabled?: null | boolean;
  pat?: null | string;
}

export interface RegistryConfigurationGitlabRegistry {
  $type?: "Gitlab";
  userName: string;
  pat: string;
  instanceUrl: string;
}

export interface RegistrySnapshot {
  /** @format uuid */
  id: string;
  name: string;
  description: string;
  registryHost: string;
  status: RegistryStatus;
  configuration: RegistryConfiguration;
}

export interface RegistryView {
  /** @format uuid */
  id: string;
  /** @format uuid */
  createdByActorId: string;
  name: string;
  status: RegistryStatus;
  description: null | string;
  registryHost: string;
  type: RegistryType;
  /** @format date-time */
  createdAt: any;
  tags: TagSummaryView[];
  capabilities?: null | ResourceCapabilities;
  isDefault?: boolean;
}

export interface RemoveTeamResourceAccessInput {
  resourceType: ResourceType;
  /** @format uuid */
  resourceId: string;
  permissionLevel: PermissionLevel;
  specificPermissions: null | SpecificPermission[];
}

export interface RemoveUserResourceAccessInput {
  resourceType: ResourceType;
  /** @format uuid */
  resourceId: string;
  permissionLevel: PermissionLevel;
  specificPermissions: null | SpecificPermission[];
}

export interface RenameResource {
  /** @format uuid */
  id: string;
  name: string;
}

export interface ReplaceResourceTagsInput {
  tagIds: null | string[];
}

export interface RepoCommand {
  commands: string[];
  /** @default "./" */
  path?: string;
}

export interface RepoSyncResultSnapshot {
  commitSha?: null | string;
  message?: null | string;
}

export interface RepoWebhookConfig {
  /** @default false */
  enabled?: boolean;
  provider?: WebhookProvider;
  authScheme?: WebhookAuthScheme;
  secret?: null | string;
  branchFilter?: null | string;
}

export interface ResolveAlertEventsInput {
  ids: string[];
  resolutionNote: null | string;
}

export interface ResourceAccessView {
  resourceType: ResourceType;
  /** @format uuid */
  resourceId: string;
  resourceName: null | string;
  permissionLevel: PermissionLevel;
  specificPermissions: null | SpecificPermission[];
}

export interface ResourceBindingInput {
  name: string;
  kind: ResourceBindingKind;
  value: null | string;
  /** @format uuid */
  secretId: null | string;
  secretDeliveryMode?: any;
  targetPath?: null | string;
}

export interface ResourceBindingSnapshot {
  name: string;
  kind: ResourceBindingKind;
  scope: ResourceBindingScope;
  value: null | string;
  /** @format uuid */
  secretId: null | string;
  secretDeliveryMode: null | SecretDeliveryMode;
  targetPath: null | string;
}

export interface ResourceBindingView {
  /** @format uuid */
  id: string;
  name: string;
  kind: ResourceBindingKind;
  scope: ResourceBindingScope;
  /** @format uuid */
  resourceId: null | string;
  value: null | string;
  /** @format uuid */
  secretId: null | string;
  secretDeliveryMode: null | SecretDeliveryMode;
  targetPath: null | string;
  isInherited: boolean;
}

export interface ResourceBindingsView {
  entries: ResourceBindingView[];
  effectiveEntries: ResourceBindingView[];
  capabilities?: null | ResourceCapabilities;
}

export interface ResourceCapabilities {
  canRead: boolean;
  canWrite: boolean;
  canExecute: boolean;
}

export interface ResourceInfo {
  /** @format uuid */
  id: string;
  name: string;
}

export interface ResourceSpec {
  /**
   * @format float
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  nanoCpus: null | number | string;
  /**
   * @format float
   * @pattern ^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?$
   */
  memoryLimit: null | number | string;
}

export interface ResourceTagsView {
  tags: TagSummaryView[];
}

export interface RestartPolicy {
  name: null | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  maximumRetryCount: null | number | string;
}

export interface RestoreVolumeInput {
  /** @format uuid */
  targetPlatformId: string;
  targetVolumeName: string;
  overwriteExisting: boolean;
}

export interface RevokeOtherProfileSessionsView {
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  count: number | string;
}

export interface RoleInput {
  name: string;
  permissions: PermissionInput[];
}

export interface RoleView {
  /** @format uuid */
  id: string;
  name: string;
  roleType: RoleType;
  permissions: PermissionView[];
}

export interface RolesView {
  roles: RoleView[];
  capabilities: ResourceCapabilities;
}

export interface RollbackStackInput {
  /** @format uuid */
  stackId: string;
  /** @format uuid */
  releaseId: string;
}

export interface RunAutomationActionInput {
  argsJson: null | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  timeoutSeconds: null | number | string;
}

export interface SecretDefinitionView {
  /** @format uuid */
  id: string;
  name: string;
  providerType: SecretProviderType;
  /** @format uuid */
  providerId: null | string;
  externalPath: null | string;
  externalKey: null | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  externalVersion: null | number | string;
  /** @format date-time */
  createdAt: any;
}

export interface SecretDefinitionsView {
  secrets: SecretDefinitionView[];
  capabilities: ResourceCapabilities;
}

export type SecretDeliveryMode = any;

export interface SecretProviderConnectionTestResultView {
  success: boolean;
  message: string;
}

export interface SecretProviderView {
  /** @format uuid */
  id: string;
  name: string;
  providerType: SecretProviderType;
  address: string;
  mountPath: string;
  /** @format date-time */
  createdAt: any;
}

export interface SecretProvidersView {
  providers: SecretProviderView[];
}

export interface StackBackupSourcePreviewView {
  /** @format uuid */
  stackId: string;
  stackName: string;
  /** @format uuid */
  platformId: string;
  platformName: string;
  platformStatus: PlatformStatus;
  volumes: StackBackupVolumeView[];
  warnings: string[];
}

export interface StackBackupVolumeView {
  name: string;
  kind: StackVolumeKind;
  isExternal: boolean;
  isShared: boolean;
  hasBackupCoverage: boolean;
}

export interface StackBuildImageBinding {
  serviceName: string;
  /** @format uuid */
  buildProjectId: string;
  /** @default false */
  redeployOnBuild?: boolean;
  resolvedImageReference?: null | string;
  resolvedDigest?: null | string;
}

export interface StackCapabilities {
  canViewLogs: boolean;
  canInspect: boolean;
  canOpenTerminal: boolean;
  canPull: boolean;
  canApply: boolean;
  canViewResourceBindings: boolean;
  canViewReleases: boolean;
  canRead: boolean;
  canWrite: boolean;
  canExecute: boolean;
}

export interface StackCommand {
  commands: string[];
  /** @default "./" */
  path?: string;
}

export interface StackConfigView {
  /** @format uuid */
  id: string;
  name: string;
  /** @format uuid */
  platformId: string;
  description: null | string;
  stackSource: StackSource;
  spec: StackSpec;
  stackUpdateState: StackUpdateState;
  driftPolicy: StackDriftPolicy;
}

export interface StackContainerStatsView {
  containerId: string;
  containerName: string;
  stats: ContainerStatView[];
}

export interface StackDriftConfigHashMismatch {
  $type?: "ConfigHashMismatch";
  serviceName: string;
  expectedHash: null | string;
  actualHash: null | string;
}

export interface StackDriftContainerPaused {
  $type?: "ContainerPaused";
  containerId: string;
  serviceName: string;
}

export interface StackDriftContainerStopped {
  $type?: "ContainerStopped";
  containerId: string;
  serviceName: string;
}

export interface StackDriftContainerUnhealthy {
  $type?: "ContainerUnhealthy";
  containerId: string;
  serviceName: string;
  healthStatus: null | string;
}

export interface StackDriftExtraContainer {
  $type?: "ExtraContainer";
  containerId: string;
  serviceName: string;
}

export interface StackDriftImageMismatch {
  $type?: "ImageMismatch";
  serviceName: string;
  expectedImage: string;
  actualImage: string;
}

export interface StackDriftMissingContainer {
  $type?: "MissingContainer";
  serviceName: string;
}

export interface StackDriftPolicy {
  mode: StackDriftMode;
  alertOnDrift: boolean;
  markDegraded: boolean;
  autoStartStoppedContainers: boolean;
  autoResumePausedContainers: boolean;
  removeExtraContainers: boolean;
}

export interface StackDriftPolicyInput {
  mode?: any;
  alertOnDrift?: null | boolean;
  markDegraded?: null | boolean;
  autoStartStoppedContainers?: null | boolean;
  autoResumePausedContainers?: null | boolean;
  removeExtraContainers?: null | boolean;
}

export interface StackDriftReport {
  /** @format uuid */
  stackId: string;
  /** @format uuid */
  platformId: string;
  hasDrift: boolean;
  hasAutoFixableDrift: boolean;
  hasStructuralDrift: boolean;
  drifts: StackDrift[];
}

export interface StackDuplicateDraftView {
  draft: CreateStackInput;
  warnings: DuplicateDraftWarningView[];
}

export interface StackImageUpdateItem {
  serviceName: string;
  imageName: string;
  currentDigest: string;
  latestDigest: string;
}

export interface StackReconciliationAction {
  containerId: string;
  serviceName: string;
  action: StackReconciliationActionType;
  succeeded: boolean;
  errorMessage?: null | string;
}

export interface StackReconciliationResult {
  /** @format uuid */
  stackId: string;
  status: StackReconciliationStatus;
  beforeReport: StackDriftReport;
  afterReport: null | StackDriftReport;
  actions: StackReconciliationAction[];
}

export interface StackReleaseSnapshot {
  /** @format uuid */
  platformId: string;
  spec: StackSpec;
  /** @format uuid */
  createdByActorId: string;
  version: null | string;
  source?: null | StackReleaseSource;
  resourceBindings?: null | ResourceBindingSnapshot[];
}

export interface StackReleaseSource {
  sourceType: StackSource;
  /** @format uuid */
  gitRepositoryId: null | string;
  gitRepositoryName: null | string;
  branch: null | string;
  requestedCommitSha: null | string;
  resolvedCommitSha: string;
  composePaths: string[];
  envFilePaths: string[];
  gitRepositoryUrl?: null | string;
  workingDirectory?: null | string;
  watchPaths?: null | string[];
  composeEnvFilesFromRepo?: null | string[];
  composeDigest?: null | string;
}

export interface StackReleaseView {
  /** @format uuid */
  id: string;
  /** @format uuid */
  stackId: string;
  /** @format uuid */
  platformId: string;
  status: StackReleaseStatus;
  version: string;
  spec: StackSpec;
  source: null | StackReleaseSource;
  resourceBindings: null | ResourceBindingSnapshot[];
  /** @format date-time */
  createdAt: any;
  /** @format uuid */
  createdByActorId: string;
  actorName: string;
  actorType: ActorType;
  platformStatus?: PlatformStatus;
  platformName?: null | string;
}

export interface StackReleasesView {
  releases: StackReleaseView[];
}

export interface StackResultSnapshot {
  containerIds?: null | string[];
  message?: null | string;
  resourceBindings?: null | ResourceBindingSnapshot[];
}

export interface StackSnapshot {
  /** @format uuid */
  id: string;
  name: string;
  description: null | string;
  stackSource: StackSource;
  driftPolicy: StackDriftPolicy;
  stackRelease: null | StackReleaseSnapshot;
}

export interface StackSpecGitStack {
  $type?: "Git";
  /** @format uuid */
  gitRepoId: string;
  branch: string;
  commitSha: null | string;
  updateBehavior: StackUpdateBehavior;
  webhook?: null | StackWebhookConfig;
  composePaths?: null | string[];
  workingDirectory?: null | string;
  composeEnvFilesFromRepo?: null | string[];
  watchPaths?: null | string[];
  additionalEnvFileFromRepo?: null | string[];
  projectName?: null | string;
  preDeploy?: null | StackCommand;
  postDeploy?: null | StackCommand;
  envFilePath?: null | string;
  /** @format uuid */
  registryId?: null | string;
  /** @default true */
  destroyBeforeDeploy?: boolean;
  buildImageBindings?: null | StackBuildImageBinding[];
}

export interface StackSpecManualStack {
  $type?: "WebEditor";
  composeFile: string;
  updateBehavior: StackUpdateBehavior;
  projectName?: null | string;
  preDeploy?: null | StackCommand;
  postDeploy?: null | StackCommand;
  envFilePath?: null | string;
  /** @format uuid */
  registryId?: null | string;
  /** @default true */
  destroyBeforeDeploy?: boolean;
  buildImageBindings?: null | StackBuildImageBinding[];
}

export interface StackStatsView {
  containers: StackContainerStatsView[];
}

export interface StackStreamItem {
  type: StackApplyEventType;
  /** @format date-time */
  timestamp: any;
  progressMessage?: null | string;
  message?: null | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  exitCode?: null | number | string;
  stackStatus?: any;
  severity?: null | string;
}

export interface StackUpdateStateGitStackUpdateState {
  $type?: "Git";
  recreateStackOnNewImageState: RecreateStackOnNewImageState;
  recreateStackOnNewCommitState: RecreateStackOnNewCommitState;
}

export interface StackUpdateStateManualStackUpdateState {
  $type?: "WebEditor";
  recreateStackOnNewImageState: RecreateStackOnNewImageState;
}

export interface StackView {
  /** @format uuid */
  id: string;
  name: string;
  description: null | string;
  stackSource: StackSource;
  stackUpdateState: StackUpdateState;
  driftPolicy: StackDriftPolicy;
  status: StackReleaseStatus;
  /** @format date-time */
  createdAt: any;
  /** @format uuid */
  createdByActorId: string;
  controlState: ResourceControlState;
  /** @format uuid */
  currentStackReleaseId: string;
  /** @format uuid */
  platformId?: null | string;
  version?: null | string;
  spec?: null | StackSpec;
  source?: null | StackReleaseSource;
  resourceBindings?: null | ResourceBindingSnapshot[];
  platformStatus?: PlatformStatus;
  platformName?: null | string;
  tags?: TagSummaryView[];
  latestActivityView?: null | LatestActivityView;
  capabilities?: null | StackCapabilities;
}

export interface StackWebhookConfig {
  /** @default false */
  forceDeploy?: boolean;
  /** @default false */
  enabled?: boolean;
  provider?: WebhookProvider;
  authScheme?: WebhookAuthScheme;
  secret?: null | string;
  branchFilter?: null | string;
}

export interface StacksView {
  stacks: StackView[];
  capabilities: ResourceCapabilities;
}

export interface StartProfileMfaSetupInput {
  password: string;
}

export interface SwarmPeer {
  nodeID: null | string;
  addr: null | string;
}

export interface TagSummaryView {
  /** @format uuid */
  id: string;
  name: string;
  color: string;
}

export interface TagView {
  /** @format uuid */
  id: string;
  name: string;
  normalizedName: string;
  color: string;
  /** @format uuid */
  createdByActorId: string;
  /** @format date-time */
  createdAt: any;
  /** @format date-time */
  updatedAt: any;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  usageCount: number | string;
}

export interface TagsView {
  tags: TagView[];
}

export interface TeamResourceAccessInput {
  resourceType: ResourceType;
  /** @format uuid */
  resourceId: string;
  permissionLevel: PermissionLevel;
  specificPermissions: null | SpecificPermission[];
}

export interface TeamSearchItemView {
  /** @format uuid */
  id: string;
  name: string;
}

export interface TeamView {
  /** @format uuid */
  id: string;
  name: string;
  /** @format uuid */
  actorId: string;
  isEnabled: boolean;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  totalMembers: number | string;
  users?: null | ResourceInfo[];
  roles?: null | ResourceInfo[];
  resourceAccesses?: null | ResourceAccessView[];
}

export interface TeamsView {
  pagedResult: PagedResultViewOfTeamView;
  capabilities: ResourceCapabilities;
}

export interface TestAutomationActionInput {
  code: string;
  argsJson: null | string;
  defaultArgsJson: null | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  timeoutSeconds: null | number | string;
  /** @format uuid */
  runAsActorId: null | string;
}

export interface TestExternalSecretInput {
  /** @format uuid */
  providerId: string;
  externalPath: string;
  externalKey: string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  externalVersion: null | number | string;
}

export interface TestOidcProviderDiscoveryInput {
  /** @format uuid */
  providerId: null | string;
  issuer: null | string;
}

export interface TestVaultKvV2SecretProviderConnectionInput {
  /** @format uuid */
  providerId: null | string;
  name: null | string;
  address: string;
  mountPath: string;
  token: null | string;
}

export interface TimeZoneInfo {
  /** @pattern ^-?(\d+\.)?\d{2}:\d{2}:\d{2}(\.\d{1,7})?$ */
  baseUtcOffset?: string;
  daylightName?: null | string;
  displayName?: null | string;
  hasIanaId?: boolean;
  id?: null | string;
  standardName?: null | string;
  supportsDaylightSavingTime?: boolean;
}

export interface TopologyEntry {
  labels: Record<string, string>;
}

export interface Ulimit {
  name: null | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  soft: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  hard: null | number | string;
}

export interface UnresolvedAlertsCountView {
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  count: number | string;
}

export interface UpdateAutomationActionInput {
  description?: null | string;
  code?: null | string;
  defaultArgsJson?: null | string;
  enabled?: null | boolean;
  scheduleEnabled?: null | boolean;
  scheduleCron?: null | string;
  scheduleTimeZone?: null | string;
  webhook?: null | AutomationWebhookConfig;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  timeoutSeconds?: null | number | string;
  alertOnFailure?: null | boolean;
  /** @format uuid */
  runAsActorId?: null | string;
}

export interface UpdateBackupPolicyInput {
  description?: null | string;
  source?: null | BackupSourceSpec;
  /** @format uuid */
  backupRepositoryId?: null | string;
  enabled?: null | boolean;
  cron?: null | string;
  timeZone?: null | string;
  webhook?: null | BackupWebhookConfig;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  keepLastSuccessful?: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  timeoutSeconds?: null | number | string;
  alertOnFailure?: null | boolean;
  /** @format uuid */
  runAsActorId?: null | string;
}

export interface UpdateBackupRepositoryInput {
  description?: null | string;
  spec?: null | BackupRepositorySpec;
}

export interface UpdateBuildAgentPoolInput {
  description?: null | string;
  enabled?: null | boolean;
  providerSpec?: null | BuildAgentPoolProviderSpec;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  maxActiveBuilders?: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  queueTimeoutSeconds?: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  provisioningTimeoutSeconds?: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  registrationTimeoutSeconds?: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  heartbeatTimeoutSeconds?: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  cleanupTimeoutSeconds?: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  maximumInstanceLifetimeSeconds?: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  failureRetentionMinutes?: null | number | string;
}

export interface UpdateBuildProjectInput {
  description?: null | string;
  enabled?: null | boolean;
  /** @format uuid */
  gitRepositoryId?: null | string;
  branch?: null | string;
  contextPath?: null | string;
  dockerfilePath?: null | string;
  target?: null | string;
  buildArgs?: null | BuildArgSpec[];
  buildSecrets?: null | BuildSecretSpec[];
  builderKind?: any;
  /** @format uuid */
  platformId?: null | string;
  /** @format uuid */
  buildAgentPoolId?: null | string;
  /** @format uuid */
  registryId?: null | string;
  imageRepository?: null | string;
  tagTemplates?: null | string[];
  webhook?: null | BuildWebhookConfig;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  timeoutSeconds?: null | number | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  retentionRunCount?: null | number | string;
}

export interface UpdateCurrentProfileInput {
  displayName: string;
}

export interface UpdateExternalSecretInput {
  name: null | string;
  /** @format uuid */
  providerId: null | string;
  externalPath: null | string;
  externalKey: null | string;
  /**
   * @format int32
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  externalVersion: null | number | string;
}

export interface UpdateOidcProviderInput {
  name?: null | string;
  description?: null | string;
  displayName?: null | string;
  issuer?: null | string;
  clientId?: null | string;
  clientSecret?: null | string;
  scopes?: null | string;
  enabled?: null | boolean;
  autoProvisionUsers?: null | boolean;
  allowEmailAutoLink?: null | boolean;
  requireEmailVerified?: null | boolean;
  allowedEmailDomains?: null | string;
  requiredClaimName?: null | string;
  requiredClaimValues?: null | string;
  /** @format uuid */
  defaultRoleId?: null | string;
}

export interface UpdateResourceBindingInput {
  /** @format uuid */
  id: string;
  name: string;
  kind: ResourceBindingKind;
  value: null | string;
  /** @format uuid */
  secretId: null | string;
  secretDeliveryMode?: any;
  targetPath?: null | string;
}

export interface UpdateVaultKvV2SecretProviderInput {
  name: null | string;
  address: null | string;
  mountPath: null | string;
  token: null | string;
}

export interface UserPreferencesView {
  timeZone: null | string;
  dateTimeFormat: UserDateTimeFormat;
  theme: UserTheme;
  isPersisted: boolean;
}

export interface UserResourceAccessInput {
  resourceType: ResourceType;
  /** @format uuid */
  resourceId: string;
  permissionLevel: PermissionLevel;
  specificPermissions: null | SpecificPermission[];
}

export interface UserSearchItemView {
  /** @format uuid */
  id: string;
  name: string;
  email: string;
}

export interface UserSessionSummaryView {
  /** @format uuid */
  id: string;
  displayName: string;
  userAgent: null | string;
  ipAddress: null | string;
  /** @format date-time */
  createdAt: any;
  /** @format date-time */
  lastSeenAt: any;
  /** @format date-time */
  expiresAt: any;
  isCurrent: boolean;
}

export interface UserSessionsView {
  sessions: UserSessionSummaryView[];
  canRevokeOtherSessions: boolean;
}

export interface UserView {
  /** @format uuid */
  id: string;
  name: string;
  email: string;
  /** @format uuid */
  actorId: string;
  isEnabled: boolean;
  teams?: null | ResourceInfo[];
  roles?: null | ResourceInfo[];
  resourceAccesses?: null | ResourceAccessView[];
}

export interface UsersView {
  pagedResult: PagedResultViewOfUserView;
  capabilities: ResourceCapabilities;
}

export interface ValidateBackupRepositoryInput {
  location: BackupExecutionLocation;
  /** @format uuid */
  platformId: null | string;
}

export interface VerifyAlertChannelInput {
  alertDestination: AlertDestination;
  name: string;
  url: string;
}

export interface VolumeAccessMode {
  scope: VolumeScope;
  sharing: VolumeSharing;
  secrets: VolumeSecret[];
  capacityRange: null | VolumeCapacityRange;
  availability: string;
}

export interface VolumeCapabilities {
  canInspect: boolean;
  canBrowse: boolean;
  canDownload: boolean;
  canRead: boolean;
  canWrite: boolean;
  canExecute: boolean;
}

export interface VolumeCapacityRange {
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  requiredBytes: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  limitBytes: null | number | string;
}

export interface VolumeDirectoryView {
  /** @format uuid */
  platformId: string;
  volumeName: string;
  path: string;
  entries: VolumeFileEntryView[];
  isTruncated: boolean;
}

export interface VolumeFileEntryView {
  name: string;
  path: string;
  type: VolumeFileEntryType;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  size: null | number | string;
  /** @format date-time */
  modifiedAt: any;
  linkTarget?: null | string;
}

export interface VolumeOptions {
  noCopy: null | boolean;
  labels: Record<string, string>;
  driverConfig: null | DriverConfiguration;
  subpath: null | string;
}

export interface VolumePublishStatus {
  nodeID: string;
  state: string;
  publishContext: Record<string, string>;
}

export interface VolumeSecret {
  key: string;
  secret: string;
}

export interface VolumeSpecification {
  group: string;
  accessMode: null | VolumeAccessMode;
}

export interface VolumeUsageData {
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  size: null | number | string;
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  refCount: null | number | string;
}

export interface VolumeVersionInfo {
  /**
   * @format int64
   * @pattern ^-?(?:0|[1-9]\d*)$
   */
  index: null | number | string;
}

export interface VolumesView {
  volumes: DockerVolumeResultView[];
  capabilities: ResourceCapabilities;
}

type BaseStackUpdateState = object;

type BaseStackUpdateStateTypeMapping<Key, Type> = {
  $type: Key;
} & Type;

type BaseStackSpec = object;

type BaseStackSpecTypeMapping<Key, Type> = {
  $type: Key;
} & Type;

type BaseStackDrift = object;

type BaseStackDriftTypeMapping<Key, Type> = {
  $type: Key;
} & Type;

type BaseRegistryConfiguration = object;

type BaseRegistryConfigurationTypeMapping<Key, Type> = {
  $type: Key;
} & Type;

type BasePlatformDescriptor = object;

type BasePlatformDescriptorTypeMapping<Key, Type> = {
  $type: Key;
} & Type;

type BaseIImageRepository = object;

type BaseIImageRepositoryTypeMapping<Key, Type> = {
  $type: Key;
} & Type;

type BaseGitAuthConfiguration = object;

type BaseGitAuthConfigurationTypeMapping<Key, Type> = {
  $type: Key;
} & Type;

type BaseDeploymentImageInfo = object;

type BaseDeploymentImageInfoTypeMapping<Key, Type> = {
  $type: Key;
} & Type;

type BaseBuildAgentPoolProviderSpec = object;

type BaseBuildAgentPoolProviderSpecTypeMapping<Key, Type> = {
  $type: Key;
} & Type;

type BaseBackupSourceSpec = object;

type BaseBackupSourceSpecTypeMapping<Key, Type> = {
  $type: Key;
} & Type;

type BaseBackupRepositorySpec = object;

type BaseBackupRepositorySpecTypeMapping<Key, Type> = {
  $type: Key;
} & Type;

type BaseAlertRuleQuietHour = object;

type BaseAlertRuleQuietHourTypeMapping<Key, Type> = {
  $type: Key;
} & Type;

type BaseAlertEventInfo = object;

type BaseAlertEventInfoTypeMapping<Key, Type> = {
  $type: Key;
} & Type;

type BaseActivityEventInfo = object;

type BaseActivityEventInfoTypeMapping<Key, Type> = {
  $type: Key;
} & Type;

export type QueryParamsType = Record<string | number, any>;
export type ResponseFormat = keyof Omit<Body, "body" | "bodyUsed">;

export interface FullRequestParams extends Omit<RequestInit, "body"> {
  /** set parameter to `true` for call `securityWorker` for this request */
  secure?: boolean;
  /** request path */
  path: string;
  /** content type of request body */
  type?: ContentType;
  /** query params */
  query?: QueryParamsType;
  /** format of response (i.e. response.json() -> format: "json") */
  format?: ResponseFormat;
  /** request body */
  body?: unknown;
  /** base url */
  baseUrl?: string;
  /** request cancellation token */
  cancelToken?: CancelToken;
}

export type RequestParams = Omit<
  FullRequestParams,
  "body" | "method" | "query" | "path"
>;

export interface ApiConfig<SecurityDataType = unknown> {
  baseUrl?: string;
  baseApiParams?: Omit<RequestParams, "baseUrl" | "cancelToken" | "signal">;
  securityWorker?: (
    securityData: SecurityDataType | null,
  ) => Promise<RequestParams | void> | RequestParams | void;
  customFetch?: typeof fetch;
}

export interface HttpResponse<D extends unknown, E extends unknown = unknown>
  extends Response {
  data: D;
  error: E;
}

type CancelToken = Symbol | string | number;

export enum ContentType {
  Json = "application/json",
  JsonApi = "application/vnd.api+json",
  FormData = "multipart/form-data",
  UrlEncoded = "application/x-www-form-urlencoded",
  Text = "text/plain",
}

export class HttpClient<SecurityDataType = unknown> {
  public baseUrl: string = "";
  private securityData: SecurityDataType | null = null;
  private securityWorker?: ApiConfig<SecurityDataType>["securityWorker"];
  private abortControllers = new Map<CancelToken, AbortController>();
  private customFetch = (...fetchParams: Parameters<typeof fetch>) =>
    fetch(...fetchParams);

  private baseApiParams: RequestParams = {
    credentials: "same-origin",
    headers: {},
    redirect: "follow",
    referrerPolicy: "no-referrer",
  };

  constructor(apiConfig: ApiConfig<SecurityDataType> = {}) {
    Object.assign(this, apiConfig);
  }

  public setSecurityData = (data: SecurityDataType | null) => {
    this.securityData = data;
  };

  protected encodeQueryParam(key: string, value: any) {
    const encodedKey = encodeURIComponent(key);
    return `${encodedKey}=${encodeURIComponent(typeof value === "number" ? value : `${value}`)}`;
  }

  protected addQueryParam(query: QueryParamsType, key: string) {
    return this.encodeQueryParam(key, query[key]);
  }

  protected addArrayQueryParam(query: QueryParamsType, key: string) {
    const value = query[key];
    return value.map((v: any) => this.encodeQueryParam(key, v)).join("&");
  }

  protected toQueryString(rawQuery?: QueryParamsType): string {
    const query = rawQuery || {};
    const keys = Object.keys(query).filter(
      (key) => "undefined" !== typeof query[key],
    );
    return keys
      .map((key) =>
        Array.isArray(query[key])
          ? this.addArrayQueryParam(query, key)
          : this.addQueryParam(query, key),
      )
      .join("&");
  }

  protected addQueryParams(rawQuery?: QueryParamsType): string {
    const queryString = this.toQueryString(rawQuery);
    return queryString ? `?${queryString}` : "";
  }

  private contentFormatters: Record<ContentType, (input: any) => any> = {
    [ContentType.Json]: (input: any) =>
      input !== null && (typeof input === "object" || typeof input === "string")
        ? JSON.stringify(input)
        : input,
    [ContentType.JsonApi]: (input: any) =>
      input !== null && (typeof input === "object" || typeof input === "string")
        ? JSON.stringify(input)
        : input,
    [ContentType.Text]: (input: any) =>
      input !== null && typeof input !== "string"
        ? JSON.stringify(input)
        : input,
    [ContentType.FormData]: (input: any) => {
      if (input instanceof FormData) {
        return input;
      }

      return Object.keys(input || {}).reduce((formData, key) => {
        const property = input[key];
        formData.append(
          key,
          property instanceof Blob
            ? property
            : typeof property === "object" && property !== null
              ? JSON.stringify(property)
              : `${property}`,
        );
        return formData;
      }, new FormData());
    },
    [ContentType.UrlEncoded]: (input: any) => this.toQueryString(input),
  };

  protected mergeRequestParams(
    params1: RequestParams,
    params2?: RequestParams,
  ): RequestParams {
    return {
      ...this.baseApiParams,
      ...params1,
      ...(params2 || {}),
      headers: {
        ...(this.baseApiParams.headers || {}),
        ...(params1.headers || {}),
        ...((params2 && params2.headers) || {}),
      },
    };
  }

  protected createAbortSignal = (
    cancelToken: CancelToken,
  ): AbortSignal | undefined => {
    if (this.abortControllers.has(cancelToken)) {
      const abortController = this.abortControllers.get(cancelToken);
      if (abortController) {
        return abortController.signal;
      }
      return void 0;
    }

    const abortController = new AbortController();
    this.abortControllers.set(cancelToken, abortController);
    return abortController.signal;
  };

  public abortRequest = (cancelToken: CancelToken) => {
    const abortController = this.abortControllers.get(cancelToken);

    if (abortController) {
      abortController.abort();
      this.abortControllers.delete(cancelToken);
    }
  };

  public request = async <T = any, E = any>({
    body,
    secure,
    path,
    type,
    query,
    format,
    baseUrl,
    cancelToken,
    ...params
  }: FullRequestParams): Promise<HttpResponse<T, E>> => {
    const secureParams =
      ((typeof secure === "boolean" ? secure : this.baseApiParams.secure) &&
        this.securityWorker &&
        (await this.securityWorker(this.securityData))) ||
      {};
    const requestParams = this.mergeRequestParams(params, secureParams);
    const queryString = query && this.toQueryString(query);
    const payloadFormatter = this.contentFormatters[type || ContentType.Json];
    const responseFormat = format || requestParams.format;

    return this.customFetch(
      `${baseUrl || this.baseUrl || ""}${path}${queryString ? `?${queryString}` : ""}`,
      {
        ...requestParams,
        headers: {
          ...(requestParams.headers || {}),
          ...(type && type !== ContentType.FormData
            ? { "Content-Type": type }
            : {}),
        },
        signal:
          (cancelToken
            ? this.createAbortSignal(cancelToken)
            : requestParams.signal) || null,
        body:
          typeof body === "undefined" || body === null
            ? null
            : payloadFormatter(body),
      },
    ).then(async (response) => {
      const r = response as HttpResponse<T, E>;
      r.data = null as unknown as T;
      r.error = null as unknown as E;

      const responseToParse = responseFormat ? response.clone() : response;
      const data = !responseFormat
        ? r
        : await responseToParse[responseFormat]()
            .then((data) => {
              if (r.ok) {
                r.data = data;
              } else {
                r.error = data;
              }
              return r;
            })
            .catch((e) => {
              r.error = e;
              return r;
            });

      if (cancelToken) {
        this.abortControllers.delete(cancelToken);
      }

      if (!response.ok) throw data;
      return data;
    });
  };
}

/**
 * @title Citadel.WebApi | v1
 * @version 1.0.0
 */
export class Api<
  SecurityDataType extends unknown,
> extends HttpClient<SecurityDataType> {
  api = {
    /**
     * No description
     *
     * @tags Authentication
     * @name RefreshToken
     * @summary Request a new access token
     * @request GET:/api/v1/authentication/refresh
     * @response `200` `RefreshTokenResponse` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    refreshToken: (params: RequestParams = {}) =>
      this.request<
        RefreshTokenResponse,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/authentication/refresh`,
        method: "GET",
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Authentication
     * @name Login
     * @summary Check user credentials and issue an access token on successful login
     * @request POST:/api/v1/authentication/login
     * @response `200` `LoginResponse` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    login: (data: LoginRequest, params: RequestParams = {}) =>
      this.request<
        LoginResponse,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/authentication/login`,
        method: "POST",
        body: data,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Authentication
     * @name VerifyAuthenticationMfa
     * @summary Verify an MFA login challenge
     * @request POST:/api/v1/authentication/mfa/verify
     * @secure
     * @response `200` `MfaVerificationView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    verifyAuthenticationMfa: (
      data: MfaVerificationInput,
      params: RequestParams = {},
    ) =>
      this.request<
        MfaVerificationView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/authentication/mfa/verify`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Authentication
     * @name GetAuthenticationMfaSetup
     * @summary Get mandatory MFA setup
     * @request GET:/api/v1/authentication/mfa/setup
     * @secure
     * @response `200` `MandatoryMfaSetupView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getAuthenticationMfaSetup: (params: RequestParams = {}) =>
      this.request<MandatoryMfaSetupView, ProblemDetails>({
        path: `/api/v1/authentication/mfa/setup`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Authentication
     * @name ConfirmAuthenticationMfaSetup
     * @summary Confirm mandatory MFA setup
     * @request POST:/api/v1/authentication/mfa/setup/confirm
     * @secure
     * @response `200` `MandatoryMfaSetupCompleteView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    confirmAuthenticationMfaSetup: (
      data: ConfirmMandatoryMfaSetupInput,
      params: RequestParams = {},
    ) =>
      this.request<
        MandatoryMfaSetupCompleteView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/authentication/mfa/setup/confirm`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Authentication
     * @name Logout
     * @summary Log out
     * @request POST:/api/v1/authentication/logout
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    logout: (params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/authentication/logout`,
        method: "POST",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Authentication
     * @name ListOidcLoginProviders
     * @summary List enabled OIDC login providers
     * @request GET:/api/v1/authentication/oidc/providers
     * @response `200` `OidcLoginProvidersView` OK
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listOidcLoginProviders: (params: RequestParams = {}) =>
      this.request<OidcLoginProvidersView, ProblemDetails>({
        path: `/api/v1/authentication/oidc/providers`,
        method: "GET",
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Authentication
     * @name BeginOidcLogin
     * @summary Start an OIDC login flow
     * @request GET:/api/v1/authentication/oidc/{id}/login
     * @response `302` `void` Found
     * @response `400` `ProblemDetails` Bad Request
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    beginOidcLogin: (
      id: string,
      query?: {
        returnUrl?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<any, void | ProblemDetails>({
        path: `/api/v1/authentication/oidc/${id}/login`,
        method: "GET",
        query: query,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Authentication
     * @name CompleteOidcLogin
     * @summary Complete an OIDC login flow
     * @request GET:/api/v1/authentication/oidc/{id}/callback
     * @response `200` `void`
     * @response `302` `void` Found
     * @response `400` `ProblemDetails` Bad Request
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    completeOidcLogin: (
      id: string,
      query?: {
        code?: string;
        state?: string;
        error?: string;
        error_description?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<void, void | ProblemDetails>({
        path: `/api/v1/authentication/oidc/${id}/callback`,
        method: "GET",
        query: query,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Application
     * @name GetApplicationInfo
     * @summary Get application information
     * @request GET:/api/v1/application/info
     * @secure
     * @response `200` `ApplicationInfoView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getApplicationInfo: (params: RequestParams = {}) =>
      this.request<ApplicationInfoView, ProblemDetails>({
        path: `/api/v1/application/info`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Profile
     * @name GetCurrentProfile
     * @summary Get current profile
     * @request GET:/api/v1/profile
     * @secure
     * @response `200` `CurrentProfileView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getCurrentProfile: (params: RequestParams = {}) =>
      this.request<CurrentProfileView, ProblemDetails>({
        path: `/api/v1/profile`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Profile
     * @name UpdateCurrentProfile
     * @summary Update current profile
     * @request PATCH:/api/v1/profile
     * @secure
     * @response `200` `CurrentProfileView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateCurrentProfile: (
      data: UpdateCurrentProfileInput,
      params: RequestParams = {},
    ) =>
      this.request<
        CurrentProfileView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/profile`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Profile
     * @name GetProfilePreferences
     * @summary Get current profile preferences
     * @request GET:/api/v1/profile/preferences
     * @secure
     * @response `200` `UserPreferencesView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getProfilePreferences: (params: RequestParams = {}) =>
      this.request<UserPreferencesView, ProblemDetails>({
        path: `/api/v1/profile/preferences`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Profile
     * @name PatchProfilePreferences
     * @summary Patch current profile preferences
     * @request PATCH:/api/v1/profile/preferences
     * @secure
     * @response `200` `UserPreferencesView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    patchProfilePreferences: (
      data: PatchUserPreferencesInput,
      params: RequestParams = {},
    ) =>
      this.request<
        UserPreferencesView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/profile/preferences`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Profile
     * @name ChangeCurrentPassword
     * @summary Change current password
     * @request POST:/api/v1/profile/change-password
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    changeCurrentPassword: (
      data: ChangeCurrentPasswordInput,
      params: RequestParams = {},
    ) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/profile/change-password`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Profile
     * @name ListProfileSessions
     * @summary List current profile sessions
     * @request GET:/api/v1/profile/sessions
     * @secure
     * @response `200` `UserSessionsView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listProfileSessions: (params: RequestParams = {}) =>
      this.request<UserSessionsView, ProblemDetails>({
        path: `/api/v1/profile/sessions`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Profile
     * @name RevokeOtherProfileSessions
     * @summary Revoke other profile sessions
     * @request DELETE:/api/v1/profile/sessions
     * @secure
     * @response `200` `RevokeOtherProfileSessionsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    revokeOtherProfileSessions: (params: RequestParams = {}) =>
      this.request<
        RevokeOtherProfileSessionsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/profile/sessions`,
        method: "DELETE",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Profile
     * @name RevokeProfileSession
     * @summary Revoke profile session
     * @request DELETE:/api/v1/profile/sessions/{sessionId}
     * @secure
     * @response `204` `void` No Content
     * @response `401` `ProblemDetails` Unauthorized
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    revokeProfileSession: (sessionId: string, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/profile/sessions/${sessionId}`,
        method: "DELETE",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Profile
     * @name GetProfileMfaStatus
     * @summary Get current profile MFA status
     * @request GET:/api/v1/profile/mfa
     * @secure
     * @response `200` `ProfileMfaStatusView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getProfileMfaStatus: (params: RequestParams = {}) =>
      this.request<ProfileMfaStatusView, ProblemDetails>({
        path: `/api/v1/profile/mfa`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Profile
     * @name StartProfileMfaSetup
     * @summary Start current profile MFA setup
     * @request POST:/api/v1/profile/mfa/setup
     * @secure
     * @response `200` `ProfileMfaSetupView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    startProfileMfaSetup: (
      data: StartProfileMfaSetupInput,
      params: RequestParams = {},
    ) =>
      this.request<
        ProfileMfaSetupView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/profile/mfa/setup`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Profile
     * @name ConfirmProfileMfaSetup
     * @summary Confirm current profile MFA setup
     * @request POST:/api/v1/profile/mfa/setup/confirm
     * @secure
     * @response `200` `ProfileMfaRecoveryCodesView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    confirmProfileMfaSetup: (
      data: ConfirmProfileMfaSetupInput,
      params: RequestParams = {},
    ) =>
      this.request<
        ProfileMfaRecoveryCodesView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/profile/mfa/setup/confirm`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Profile
     * @name DisableProfileMfa
     * @summary Disable current profile MFA
     * @request POST:/api/v1/profile/mfa/disable
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    disableProfileMfa: (
      data: DisableProfileMfaInput,
      params: RequestParams = {},
    ) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/profile/mfa/disable`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Profile
     * @name RegenerateProfileMfaRecoveryCodes
     * @summary Regenerate current profile MFA recovery codes
     * @request POST:/api/v1/profile/mfa/recovery-codes
     * @secure
     * @response `200` `ProfileMfaRecoveryCodesView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    regenerateProfileMfaRecoveryCodes: (
      data: RegenerateProfileMfaRecoveryCodesInput,
      params: RequestParams = {},
    ) =>
      this.request<
        ProfileMfaRecoveryCodesView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/profile/mfa/recovery-codes`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags License
     * @name GetLicense
     * @summary Get license status
     * @request GET:/api/v1/license
     * @secure
     * @response `200` `LicenseView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getLicense: (params: RequestParams = {}) =>
      this.request<LicenseView, ProblemDetails>({
        path: `/api/v1/license`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags License
     * @name InstallLicense
     * @summary Install or replace license
     * @request POST:/api/v1/license
     * @secure
     * @response `200` `LicenseView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    installLicense: (data: InstallLicenseInput, params: RequestParams = {}) =>
      this.request<LicenseView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/license`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags License
     * @name RemoveLicense
     * @summary Remove installed license
     * @request DELETE:/api/v1/license
     * @secure
     * @response `200` `LicenseView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    removeLicense: (params: RequestParams = {}) =>
      this.request<LicenseView, ProblemDetails>({
        path: `/api/v1/license`,
        method: "DELETE",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags License
     * @name GetLicenseRequest
     * @summary Get license request details
     * @request GET:/api/v1/license/request
     * @secure
     * @response `200` `LicenseRequestView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getLicenseRequest: (params: RequestParams = {}) =>
      this.request<LicenseRequestView, ProblemDetails>({
        path: `/api/v1/license/request`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Actors
     * @name GetActor
     * @summary Get actor by ID
     * @request GET:/api/v1/actors/{id}
     * @secure
     * @response `200` `ActorView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getActor: (id: string, params: RequestParams = {}) =>
      this.request<ActorView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/actors/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Actors
     * @name PatchActorEnabled
     * @summary Enable or disable an actor
     * @request PATCH:/api/v1/actors/{id}/enabled
     * @secure
     * @response `200` `ActorView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    patchActorEnabled: (
      id: string,
      data: PatchActorEnabledInput,
      params: RequestParams = {},
    ) =>
      this.request<ActorView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/actors/${id}/enabled`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Users
     * @name ListUsers
     * @summary Get all users
     * @request GET:/api/v1/users
     * @secure
     * @response `200` `UsersView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listUsers: (
      query?: {
        Name?: string;
        /**
         * @format int32
         * @default 1
         * @pattern ^-?(?:0|[1-9]\d*)$
         */
        Page?: number | string;
        /**
         * @format int32
         * @default 50
         * @pattern ^-?(?:0|[1-9]\d*)$
         */
        PageSize?: number | string;
      },
      params: RequestParams = {},
    ) =>
      this.request<UsersView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/users`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Users
     * @name CreateUser
     * @summary Create a user
     * @request POST:/api/v1/users
     * @secure
     * @response `200` `UserView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createUser: (data: CreateUserInput, params: RequestParams = {}) =>
      this.request<UserView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/users`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Users
     * @name DeleteUsers
     * @summary Delete users
     * @request DELETE:/api/v1/users
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteUsers: (data: DeleteUsersInput, params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/users`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Users
     * @name SearchUsers
     * @summary Search users for assignment
     * @request GET:/api/v1/users/search
     * @secure
     * @response `200` `(UserSearchItemView)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    searchUsers: (
      query?: {
        Query?: string;
        /**
         * @format int32
         * @default 20
         * @pattern ^-?(?:0|[1-9]\d*)$
         */
        Limit?: number | string;
      },
      params: RequestParams = {},
    ) =>
      this.request<
        UserSearchItemView[],
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/users/search`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Users
     * @name GetUser
     * @summary Get user by ID
     * @request GET:/api/v1/users/{id}
     * @secure
     * @response `200` `UserView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getUser: (id: string, params: RequestParams = {}) =>
      this.request<UserView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/users/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Users
     * @name UpdateUser
     * @summary Update a user
     * @request PATCH:/api/v1/users/{id}
     * @secure
     * @response `200` `UserView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateUser: (
      id: string,
      data: PatchUserInput,
      params: RequestParams = {},
    ) =>
      this.request<UserView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/users/${id}`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Users
     * @name AddUserRole
     * @summary Assign a role to a user
     * @request POST:/api/v1/users/{id}/roles
     * @secure
     * @response `200` `UserView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    addUserRole: (
      id: string,
      data: AddUserRoleInput,
      params: RequestParams = {},
    ) =>
      this.request<UserView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/users/${id}/roles`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Users
     * @name RemoveUserRole
     * @summary Remove a role from a user
     * @request DELETE:/api/v1/users/{id}/roles/{roleId}
     * @secure
     * @response `200` `UserView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    removeUserRole: (id: string, roleId: string, params: RequestParams = {}) =>
      this.request<UserView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/users/${id}/roles/${roleId}`,
        method: "DELETE",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Users
     * @name AddUserResourceAccess
     * @summary Add resource access override for a user
     * @request POST:/api/v1/users/{id}/resource-accesses
     * @secure
     * @response `200` `UserView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    addUserResourceAccess: (
      id: string,
      data: AddUserResourceAccessInput,
      params: RequestParams = {},
    ) =>
      this.request<UserView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/users/${id}/resource-accesses`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Users
     * @name RemoveUserResourceAccess
     * @summary Remove resource access override for a user
     * @request DELETE:/api/v1/users/{id}/resource-accesses
     * @secure
     * @response `200` `UserView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    removeUserResourceAccess: (
      id: string,
      data: RemoveUserResourceAccessInput,
      params: RequestParams = {},
    ) =>
      this.request<UserView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/users/${id}/resource-accesses`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Users
     * @name RenameUser
     * @summary Rename a user
     * @request POST:/api/v1/users/rename
     * @secure
     * @response `200` `UserView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    renameUser: (data: RenameResource, params: RequestParams = {}) =>
      this.request<UserView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/users/rename`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Users
     * @name ResetUserMfa
     * @summary Reset user MFA
     * @request DELETE:/api/v1/users/{id}/mfa
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    resetUserMfa: (id: string, params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/users/${id}/mfa`,
        method: "DELETE",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Teams
     * @name ListTeams
     * @summary Get all teams
     * @request GET:/api/v1/teams
     * @secure
     * @response `200` `TeamsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listTeams: (
      query?: {
        Name?: string;
        /**
         * @format int32
         * @default 1
         * @pattern ^-?(?:0|[1-9]\d*)$
         */
        Page?: number | string;
        /**
         * @format int32
         * @default 50
         * @pattern ^-?(?:0|[1-9]\d*)$
         */
        PageSize?: number | string;
      },
      params: RequestParams = {},
    ) =>
      this.request<TeamsView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/teams`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Teams
     * @name CreateTeam
     * @summary Create a team
     * @request POST:/api/v1/teams
     * @secure
     * @response `200` `TeamView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createTeam: (data: CreateTeamInput, params: RequestParams = {}) =>
      this.request<TeamView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/teams`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Teams
     * @name DeleteTeams
     * @summary Delete teams
     * @request DELETE:/api/v1/teams
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteTeams: (data: DeleteTeamsInput, params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/teams`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Teams
     * @name SearchTeams
     * @summary Search teams for assignment
     * @request GET:/api/v1/teams/search
     * @secure
     * @response `200` `(TeamSearchItemView)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    searchTeams: (
      query?: {
        Query?: string;
        /**
         * @format int32
         * @default 20
         * @pattern ^-?(?:0|[1-9]\d*)$
         */
        Limit?: number | string;
      },
      params: RequestParams = {},
    ) =>
      this.request<
        TeamSearchItemView[],
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/teams/search`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Teams
     * @name GetTeam
     * @summary Get team by ID
     * @request GET:/api/v1/teams/{id}
     * @secure
     * @response `200` `TeamView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getTeam: (id: string, params: RequestParams = {}) =>
      this.request<TeamView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/teams/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Teams
     * @name UpdateTeam
     * @summary Update a team
     * @request PATCH:/api/v1/teams/{id}
     * @secure
     * @response `200` `TeamView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateTeam: (
      id: string,
      data: PatchTeamInput,
      params: RequestParams = {},
    ) =>
      this.request<TeamView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/teams/${id}`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Teams
     * @name AddTeamRole
     * @summary Assign a role to a team
     * @request POST:/api/v1/teams/{id}/roles
     * @secure
     * @response `200` `TeamView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    addTeamRole: (
      id: string,
      data: AddTeamRoleInput,
      params: RequestParams = {},
    ) =>
      this.request<TeamView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/teams/${id}/roles`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Teams
     * @name RemoveTeamRole
     * @summary Remove a role from a team
     * @request DELETE:/api/v1/teams/{id}/roles/{roleId}
     * @secure
     * @response `200` `TeamView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    removeTeamRole: (id: string, roleId: string, params: RequestParams = {}) =>
      this.request<TeamView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/teams/${id}/roles/${roleId}`,
        method: "DELETE",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Teams
     * @name AddTeamMember
     * @summary Add a member to a team
     * @request POST:/api/v1/teams/{id}/members
     * @secure
     * @response `200` `TeamView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    addTeamMember: (
      id: string,
      data: AddTeamMemberInput,
      params: RequestParams = {},
    ) =>
      this.request<TeamView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/teams/${id}/members`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Teams
     * @name RemoveTeamMember
     * @summary Remove a member from a team
     * @request DELETE:/api/v1/teams/{id}/members/{userId}
     * @secure
     * @response `200` `TeamView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    removeTeamMember: (
      id: string,
      userId: string,
      params: RequestParams = {},
    ) =>
      this.request<TeamView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/teams/${id}/members/${userId}`,
        method: "DELETE",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Teams
     * @name AddTeamResourceAccess
     * @summary Add resource access override for a team
     * @request POST:/api/v1/teams/{id}/resource-accesses
     * @secure
     * @response `200` `TeamView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    addTeamResourceAccess: (
      id: string,
      data: AddTeamResourceAccessInput,
      params: RequestParams = {},
    ) =>
      this.request<TeamView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/teams/${id}/resource-accesses`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Teams
     * @name RemoveTeamResourceAccess
     * @summary Remove resource access override for a team
     * @request DELETE:/api/v1/teams/{id}/resource-accesses
     * @secure
     * @response `200` `TeamView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    removeTeamResourceAccess: (
      id: string,
      data: RemoveTeamResourceAccessInput,
      params: RequestParams = {},
    ) =>
      this.request<TeamView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/teams/${id}/resource-accesses`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Teams
     * @name RenameTeam
     * @summary Rename a team
     * @request POST:/api/v1/teams/rename
     * @secure
     * @response `200` `TeamView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    renameTeam: (data: RenameResource, params: RequestParams = {}) =>
      this.request<TeamView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/teams/rename`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Roles
     * @name ListRoles
     * @summary Get all roles
     * @request GET:/api/v1/roles
     * @secure
     * @response `200` `RolesView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listRoles: (params: RequestParams = {}) =>
      this.request<RolesView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/roles`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Roles
     * @name CreateRole
     * @summary Create a role
     * @request POST:/api/v1/roles
     * @secure
     * @response `200` `RoleView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createRole: (data: RoleInput, params: RequestParams = {}) =>
      this.request<RoleView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/roles`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Roles
     * @name DeleteRoles
     * @summary Delete roles
     * @request DELETE:/api/v1/roles
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteRoles: (data: DeleteRolesInput, params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/roles`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Roles
     * @name GetRole
     * @summary Get role by ID
     * @request GET:/api/v1/roles/{id}
     * @secure
     * @response `200` `RoleView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getRole: (id: string, params: RequestParams = {}) =>
      this.request<RoleView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/roles/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Roles
     * @name UpdateRolePermissions
     * @summary Update role permissions
     * @request PATCH:/api/v1/roles/{id}/permissions
     * @secure
     * @response `200` `RoleView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateRolePermissions: (
      id: string,
      data: PatchRolePermissionsInput,
      params: RequestParams = {},
    ) =>
      this.request<RoleView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/roles/${id}/permissions`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Roles
     * @name RenameRole
     * @summary Rename a role
     * @request POST:/api/v1/roles/rename
     * @secure
     * @response `200` `RoleView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    renameRole: (data: RenameResource, params: RequestParams = {}) =>
      this.request<RoleView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/roles/rename`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Roles
     * @name GetPermissionMatrix
     * @summary Get the permission matrix (all valid resource capabilities and minimum levels)
     * @request GET:/api/v1/roles/permissions/matrix
     * @response `200` `Record<string,PermissionMatrixViewItem>` OK
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getPermissionMatrix: (params: RequestParams = {}) =>
      this.request<Record<string, PermissionMatrixViewItem>, ProblemDetails>({
        path: `/api/v1/roles/permissions/matrix`,
        method: "GET",
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name GetContainer
     * @summary Get container by Id
     * @request GET:/api/v1/containers/{id}
     * @secure
     * @response `200` `ContainerView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getContainer: (id: string, params: RequestParams = {}) =>
      this.request<
        ContainerView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/containers/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name GetContainerInfo
     * @summary Get basic container details
     * @request GET:/api/v1/containers/{id}/info
     * @secure
     * @response `200` `ContainerInfoView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getContainerInfo: (id: string, params: RequestParams = {}) =>
      this.request<
        ContainerInfoView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/containers/${id}/info`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name GetContainerData
     * @summary Get container data
     * @request GET:/api/v1/containers/{id}/data
     * @secure
     * @response `200` `ContainerDataView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getContainerData: (id: string, params: RequestParams = {}) =>
      this.request<
        ContainerDataView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/containers/${id}/data`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name GetContainerStats
     * @summary Get container stats
     * @request GET:/api/v1/containers/{id}/stats
     * @secure
     * @response `200` `ContainerStatsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getContainerStats: (
      id: string,
      query?: {
        /**
         * Stats lookback window in hours. Supported values: 24, 48, 72.
         * @format int32
         * @default 24
         * @pattern ^-?(?:0|[1-9]\d*)$
         */
        hours?: number | string;
      },
      params: RequestParams = {},
    ) =>
      this.request<
        ContainerStatsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/containers/${id}/stats`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name InspectContainer
     * @summary Inspect a container
     * @request GET:/api/v1/containers/{id}/inspect
     * @secure
     * @response `200` `ContainerInspectView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    inspectContainer: (id: string, params: RequestParams = {}) =>
      this.request<
        ContainerInspectView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/containers/${id}/inspect`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name StartContainers
     * @summary Starts the given container(s)
     * @request PATCH:/api/v1/containers/start
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    startContainers: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/containers/start`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name StopContainers
     * @summary Stops the given container(s)
     * @request PATCH:/api/v1/containers/stop
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    stopContainers: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/containers/stop`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name PauseContainers
     * @summary Pause the given container(s)
     * @request PATCH:/api/v1/containers/pause
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    pauseContainers: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/containers/pause`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name RestartContainers
     * @summary Restarts the given container(s)
     * @request PATCH:/api/v1/containers/restart
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    restartContainers: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/containers/restart`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name UnpauseContainers
     * @summary Resume a container(s) which has been paused
     * @request PATCH:/api/v1/containers/unpause
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    unpauseContainers: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/containers/unpause`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name DeleteContainers
     * @summary Delete the given container(s)
     * @request DELETE:/api/v1/containers
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteContainers: (
      data: DeleteContainersRequest,
      params: RequestParams = {},
    ) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/containers`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name ListPlatforms
     * @summary List all platforms
     * @request GET:/api/v1/platforms
     * @secure
     * @response `200` `PlatformsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listPlatforms: (
      query?: {
        tags?: string[];
      },
      params: RequestParams = {},
    ) =>
      this.request<
        PlatformsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/platforms`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name CreatePlatform
     * @summary Create a platform
     * @request POST:/api/v1/platforms
     * @secure
     * @response `200` `PlatformView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createPlatform: (data: CreatePlatformInput, params: RequestParams = {}) =>
      this.request<PlatformView, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/platforms`,
          method: "POST",
          body: data,
          secure: true,
          type: ContentType.Json,
          format: "json",
          ...params,
        },
      ),

    /**
     * No description
     *
     * @tags Platforms
     * @name DeletePlatforms
     * @summary Delete platforms
     * @request DELETE:/api/v1/platforms
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deletePlatforms: (data: DeletePlatformsInput, params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/platforms`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name GetAgentSetup
     * @summary Get regular Agent setup instructions
     * @request GET:/api/v1/platforms/agent/setup
     * @secure
     * @response `200` `AgentSetupView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getAgentSetup: (params: RequestParams = {}) =>
      this.request<
        AgentSetupView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/platforms/agent/setup`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name RotateAgentHubKey
     * @summary Rotate regular Agent hub key pair
     * @request POST:/api/v1/platforms/agent/setup/rotate-key
     * @secure
     * @response `200` `AgentSetupView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    rotateAgentHubKey: (params: RequestParams = {}) =>
      this.request<
        AgentSetupView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/platforms/agent/setup/rotate-key`,
        method: "POST",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name GetPlatfom
     * @summary Get platform by Id
     * @request GET:/api/v1/platforms/{id}
     * @secure
     * @response `200` `PlatformView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getPlatfom: (id: string, params: RequestParams = {}) =>
      this.request<PlatformView, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/platforms/${id}`,
          method: "GET",
          secure: true,
          format: "json",
          ...params,
        },
      ),

    /**
     * No description
     *
     * @tags Platforms
     * @name UpdatePlatform
     * @summary Patch a platform
     * @request PATCH:/api/v1/platforms/{id}
     * @secure
     * @response `200` `PlatformView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updatePlatform: (
      id: string,
      data: PlatformInput,
      params: RequestParams = {},
    ) =>
      this.request<PlatformView, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/platforms/${id}`,
          method: "PATCH",
          body: data,
          secure: true,
          type: ContentType.Json,
          format: "json",
          ...params,
        },
      ),

    /**
     * No description
     *
     * @tags Platforms
     * @name GetPlatformTags
     * @summary Get platform tags
     * @request GET:/api/v1/platforms/{id}/tags
     * @secure
     * @response `200` `ResourceTagsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getPlatformTags: (id: string, params: RequestParams = {}) =>
      this.request<
        ResourceTagsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/platforms/${id}/tags`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name ReplacePlatformTags
     * @summary Replace platform tags
     * @request PUT:/api/v1/platforms/{id}/tags
     * @secure
     * @response `200` `ResourceTagsView` OK
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    replacePlatformTags: (
      id: string,
      data: ReplaceResourceTagsInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceTagsView, ProblemDetails>({
        path: `/api/v1/platforms/${id}/tags`,
        method: "PUT",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name ListContainers
     * @summary Returns the list of containers of the given platform
     * @request GET:/api/v1/platforms/{id}/containers
     * @secure
     * @response `200` `ContainersView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listContainers: (id: string, params: RequestParams = {}) =>
      this.request<
        ContainersView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/platforms/${id}/containers`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name ListVolumeDirectory
     * @summary List volume directory contents
     * @request GET:/api/v1/platforms/{platformId}/volumes/{name}/files
     * @secure
     * @response `200` `VolumeDirectoryView` OK
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listVolumeDirectory: (
      platformId: string,
      name: string,
      query?: {
        path?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<VolumeDirectoryView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/volumes/${name}/files`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name DownloadVolumePath
     * @summary Download a volume file or directory archive
     * @request GET:/api/v1/platforms/{platformId}/volumes/{name}/files/download
     * @secure
     * @response `200` `void` OK
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    downloadVolumePath: (
      platformId: string,
      name: string,
      query?: {
        path?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/volumes/${name}/files/download`,
        method: "GET",
        query: query,
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name GetPlatformStats
     * @summary Get platform stats
     * @request GET:/api/v1/platforms/{id}/stats
     * @secure
     * @response `200` `PlatformStatsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getPlatformStats: (
      id: string,
      query?: {
        /**
         * Stats lookback window in hours. Supported values: 24, 48, 72.
         * @format int32
         * @default 24
         * @pattern ^-?(?:0|[1-9]\d*)$
         */
        hours?: number | string;
      },
      params: RequestParams = {},
    ) =>
      this.request<
        PlatformStatsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/platforms/${id}/stats`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name PrunePlatform
     * @summary Delete unused Docker resources on a platform
     * @request POST:/api/v1/platforms/{id}/prune
     * @secure
     * @response `200` `PrunePlatformView` OK
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    prunePlatform: (
      id: string,
      data: PrunePlatformInput,
      params: RequestParams = {},
    ) =>
      this.request<PrunePlatformView, ProblemDetails>({
        path: `/api/v1/platforms/${id}/prune`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name CreateEdgeAgentEnrollment
     * @summary Create an Edge Agent enrollment token for a platform
     * @request POST:/api/v1/platforms/{id}/edge/enrollments
     * @secure
     * @response `200` `EdgeAgentEnrollmentView` OK
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createEdgeAgentEnrollment: (id: string, params: RequestParams = {}) =>
      this.request<EdgeAgentEnrollmentView, ProblemDetails>({
        path: `/api/v1/platforms/${id}/edge/enrollments`,
        method: "POST",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name GetEdgeAgentStatus
     * @summary Get Edge Agent connection status for a platform
     * @request GET:/api/v1/platforms/{id}/edge/status
     * @secure
     * @response `200` `EdgeAgentStatusView` OK
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getEdgeAgentStatus: (id: string, params: RequestParams = {}) =>
      this.request<EdgeAgentStatusView, ProblemDetails>({
        path: `/api/v1/platforms/${id}/edge/status`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name RevokeEdgeAgent
     * @summary Revoke an Edge Agent binding for a platform
     * @request POST:/api/v1/platforms/{id}/edge/revoke
     * @secure
     * @response `204` `void` No Content
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    revokeEdgeAgent: (id: string, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/platforms/${id}/edge/revoke`,
        method: "POST",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name RenamePlatform
     * @summary Rename a platform
     * @request POST:/api/v1/platforms/rename
     * @secure
     * @response `200` `PlatformView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    renamePlatform: (data: RenameResource, params: RequestParams = {}) =>
      this.request<PlatformView, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/platforms/rename`,
          method: "POST",
          body: data,
          secure: true,
          type: ContentType.Json,
          format: "json",
          ...params,
        },
      ),

    /**
     * No description
     *
     * @tags Platforms
     * @name UpdatePlatformMetadata
     * @summary Patch platform metadata
     * @request PATCH:/api/v1/platforms/{id}/_metadata
     * @secure
     * @response `200` `PlatformView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updatePlatformMetadata: (
      id: string,
      data: PatchResourceMetadata,
      params: RequestParams = {},
    ) =>
      this.request<PlatformView, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/platforms/${id}/_metadata`,
          method: "PATCH",
          body: data,
          secure: true,
          type: ContentType.Json,
          format: "json",
          ...params,
        },
      ),

    /**
     * No description
     *
     * @tags Registries
     * @name ListRegistries
     * @summary Get all registries
     * @request GET:/api/v1/registries
     * @secure
     * @response `200` `RegistriesView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listRegistries: (
      query?: {
        includeDisabled?: boolean;
        tags?: string[];
      },
      params: RequestParams = {},
    ) =>
      this.request<
        RegistriesView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/registries`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * @description A discriminator should be provided in the request, this discriminator is based on RegistryType enum
     *
     * @tags Registries
     * @name CreateRegistry
     * @summary Create a registry
     * @request POST:/api/v1/registries
     * @secure
     * @response `200` `RegistryView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createRegistry: (data: CreateRegistryInput, params: RequestParams = {}) =>
      this.request<RegistryView, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/registries`,
          method: "POST",
          body: data,
          secure: true,
          type: ContentType.Json,
          format: "json",
          ...params,
        },
      ),

    /**
     * No description
     *
     * @tags Registries
     * @name DeleteRegistries
     * @summary Delete registries
     * @request DELETE:/api/v1/registries
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteRegistries: (
      data: DeleteRegistriesInput,
      params: RequestParams = {},
    ) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/registries`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Registries
     * @name GetRegistry
     * @summary Get registry by ID
     * @request GET:/api/v1/registries/{id}
     * @secure
     * @response `200` `RegistryView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getRegistry: (id: string, params: RequestParams = {}) =>
      this.request<RegistryView, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/registries/${id}`,
          method: "GET",
          secure: true,
          format: "json",
          ...params,
        },
      ),

    /**
     * @description A discriminator should be provided in the request, this discriminator is based on RegistryType enum
     *
     * @tags Registries
     * @name UpdateRegistry
     * @summary Update a registry
     * @request PATCH:/api/v1/registries/{id}
     * @secure
     * @response `200` `RegistryView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateRegistry: (
      id: string,
      data: PatchRegistryInput,
      params: RequestParams = {},
    ) =>
      this.request<RegistryView, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/registries/${id}`,
          method: "PATCH",
          body: data,
          secure: true,
          type: ContentType.Json,
          format: "json",
          ...params,
        },
      ),

    /**
     * No description
     *
     * @tags Registries
     * @name GetRegistryConfig
     * @summary Get registry configuration
     * @request GET:/api/v1/registries/{id}/_cfg
     * @secure
     * @response `200` `RegistryConfigView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getRegistryConfig: (id: string, params: RequestParams = {}) =>
      this.request<
        RegistryConfigView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/registries/${id}/_cfg`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Registries
     * @name UpdateRegistryMetadata
     * @summary Update registry metadata
     * @request PATCH:/api/v1/registries/{id}/_metadata
     * @secure
     * @response `200` `RegistryView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateRegistryMetadata: (
      id: string,
      data: PatchResourceMetadata,
      params: RequestParams = {},
    ) =>
      this.request<RegistryView, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/registries/${id}/_metadata`,
          method: "PATCH",
          body: data,
          secure: true,
          type: ContentType.Json,
          format: "json",
          ...params,
        },
      ),

    /**
     * No description
     *
     * @tags Registries
     * @name RenameRegistry
     * @summary Rename registry
     * @request POST:/api/v1/registries/rename
     * @secure
     * @response `200` `RegistryView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    renameRegistry: (data: RenameResource, params: RequestParams = {}) =>
      this.request<RegistryView, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/registries/rename`,
          method: "POST",
          body: data,
          secure: true,
          type: ContentType.Json,
          format: "json",
          ...params,
        },
      ),

    /**
     * No description
     *
     * @tags Registries
     * @name GetRegistryTags
     * @summary Get registry tags
     * @request GET:/api/v1/registries/{id}/tags
     * @secure
     * @response `200` `ResourceTagsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getRegistryTags: (id: string, params: RequestParams = {}) =>
      this.request<
        ResourceTagsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/registries/${id}/tags`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Registries
     * @name ReplaceRegistryTags
     * @summary Replace registry tags
     * @request PUT:/api/v1/registries/{id}/tags
     * @secure
     * @response `200` `ResourceTagsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    replaceRegistryTags: (
      id: string,
      data: ReplaceResourceTagsInput,
      params: RequestParams = {},
    ) =>
      this.request<
        ResourceTagsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/registries/${id}/tags`,
        method: "PUT",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags GitAccounts
     * @name ListGitAccounts
     * @summary Get all git accounts
     * @request GET:/api/v1/gitAccounts
     * @secure
     * @response `200` `GitAccountsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listGitAccounts: (params: RequestParams = {}) =>
      this.request<
        GitAccountsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/gitAccounts`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags GitAccounts
     * @name CreateGitAccount
     * @summary Create a git account
     * @request POST:/api/v1/gitAccounts
     * @secure
     * @response `200` `GitAccountView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createGitAccount: (data: GitAccountInput, params: RequestParams = {}) =>
      this.request<
        GitAccountView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/gitAccounts`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags GitAccounts
     * @name DeleteGitAccounts
     * @summary Delete git accounts
     * @request DELETE:/api/v1/gitAccounts
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteGitAccounts: (
      data: DeleteGitAccountsInput,
      params: RequestParams = {},
    ) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/gitAccounts`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags GitAccounts
     * @name GetGitAccount
     * @summary Get git account by ID
     * @request GET:/api/v1/gitAccounts/{id}
     * @secure
     * @response `200` `GitAccountView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getGitAccount: (id: string, params: RequestParams = {}) =>
      this.request<
        GitAccountView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/gitAccounts/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags GitAccounts
     * @name UpdateGitAccount
     * @summary Update a git account
     * @request PATCH:/api/v1/gitAccounts/{id}
     * @secure
     * @response `200` `GitAccountView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateGitAccount: (
      id: string,
      data: GitAccountInput,
      params: RequestParams = {},
    ) =>
      this.request<
        GitAccountView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/gitAccounts/${id}`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags GitAccounts
     * @name GetGitAccountConfig
     * @summary Get git account configuration
     * @request GET:/api/v1/gitAccounts/{id}/_cfg
     * @secure
     * @response `200` `GitAccountConfigView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getGitAccountConfig: (id: string, params: RequestParams = {}) =>
      this.request<
        GitAccountConfigView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/gitAccounts/${id}/_cfg`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags GitRepositories
     * @name ListGitRepositories
     * @summary Get all git repositories
     * @request GET:/api/v1/gitRepositories
     * @secure
     * @response `200` `GitRepositoriesView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listGitRepositories: (
      query?: {
        tags?: string[];
      },
      params: RequestParams = {},
    ) =>
      this.request<
        GitRepositoriesView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/gitRepositories`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags GitRepositories
     * @name CreateGitRepository
     * @summary Create a git repository
     * @request POST:/api/v1/gitRepositories
     * @secure
     * @response `200` `GitRepositoryView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createGitRepository: (
      data: CreateGitRepositoryInput,
      params: RequestParams = {},
    ) =>
      this.request<
        GitRepositoryView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/gitRepositories`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags GitRepositories
     * @name DeleteGitRepositories
     * @summary Delete git repositories
     * @request DELETE:/api/v1/gitRepositories
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteGitRepositories: (
      data: DeleteGitRepositoriesInput,
      params: RequestParams = {},
    ) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/gitRepositories`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags GitRepositories
     * @name GetGitRepository
     * @summary Get git repository by ID
     * @request GET:/api/v1/gitRepositories/{id}
     * @secure
     * @response `200` `GitRepositoryView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getGitRepository: (id: string, params: RequestParams = {}) =>
      this.request<
        GitRepositoryView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/gitRepositories/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags GitRepositories
     * @name UpdateGitRepository
     * @summary Update a git repository
     * @request PATCH:/api/v1/gitRepositories/{id}
     * @secure
     * @response `200` `GitRepositoryView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateGitRepository: (
      id: string,
      data: PatchGitRepositoryInput,
      params: RequestParams = {},
    ) =>
      this.request<
        GitRepositoryView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/gitRepositories/${id}`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags GitRepositories
     * @name GetGitRepositoryTags
     * @summary Get git repository tags
     * @request GET:/api/v1/gitRepositories/{id}/tags
     * @secure
     * @response `200` `ResourceTagsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getGitRepositoryTags: (id: string, params: RequestParams = {}) =>
      this.request<
        ResourceTagsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/gitRepositories/${id}/tags`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags GitRepositories
     * @name ReplaceGitRepositoryTags
     * @summary Replace git repository tags
     * @request PUT:/api/v1/gitRepositories/{id}/tags
     * @secure
     * @response `200` `ResourceTagsView` OK
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    replaceGitRepositoryTags: (
      id: string,
      data: ReplaceResourceTagsInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceTagsView, ProblemDetails>({
        path: `/api/v1/gitRepositories/${id}/tags`,
        method: "PUT",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags GitRepositories
     * @name GetGitRepositoryConfig
     * @summary Get Git repo configuration
     * @request GET:/api/v1/gitRepositories/{id}/_cfg
     * @secure
     * @response `200` `GitRepositoryConfigView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getGitRepositoryConfig: (id: string, params: RequestParams = {}) =>
      this.request<
        GitRepositoryConfigView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/gitRepositories/${id}/_cfg`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags GitRepositories
     * @name GetGitRepositoryRefs
     * @summary Get synced Git repository refs
     * @request GET:/api/v1/gitRepositories/{id}/refs
     * @secure
     * @response `200` `GitRepositoryRefsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getGitRepositoryRefs: (id: string, params: RequestParams = {}) =>
      this.request<
        GitRepositoryRefsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/gitRepositories/${id}/refs`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags GitRepositories
     * @name DiscoverGitRepositoryBranches
     * @summary Discover remote Git repository branches
     * @request GET:/api/v1/gitRepositories/{id}/branches
     * @secure
     * @response `200` `GitRepositoryBranchesView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    discoverGitRepositoryBranches: (id: string, params: RequestParams = {}) =>
      this.request<
        GitRepositoryBranchesView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/gitRepositories/${id}/branches`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags GitRepositories
     * @name DiscoverGitRepositoryComposeProjects
     * @summary Discover compose projects in a Git repository branch
     * @request GET:/api/v1/gitRepositories/{id}/compose-projects
     * @secure
     * @response `200` `GitRepositoryComposeDiscovery` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    discoverGitRepositoryComposeProjects: (
      id: string,
      query?: {
        branch?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<
        GitRepositoryComposeDiscovery,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/gitRepositories/${id}/compose-projects`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags GitRepositories
     * @name UpdateGitRepositoryMetadata
     * @summary Update git repository metadata
     * @request PATCH:/api/v1/gitRepositories/{id}/_metadata
     * @secure
     * @response `200` `GitRepositoryView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateGitRepositoryMetadata: (
      id: string,
      data: PatchResourceMetadata,
      params: RequestParams = {},
    ) =>
      this.request<
        GitRepositoryView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/gitRepositories/${id}/_metadata`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags GitRepositories
     * @name SyncGitRepository
     * @summary Sync a git repository
     * @request POST:/api/v1/gitRepositories/{id}/sync
     * @secure
     * @response `200` `GitRepositoryView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    syncGitRepository: (
      id: string,
      query?: {
        branch?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<
        GitRepositoryView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/gitRepositories/${id}/sync`,
        method: "POST",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags GitRepositories
     * @name RenameGitRepository
     * @summary Rename a git repository
     * @request POST:/api/v1/gitRepositories/rename
     * @secure
     * @response `200` `GitRepositoryView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    renameGitRepository: (data: RenameResource, params: RequestParams = {}) =>
      this.request<
        GitRepositoryView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/gitRepositories/rename`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name ListImages
     * @summary Get all local images for the given platform
     * @request GET:/api/v1/images/{platformId}
     * @secure
     * @response `200` `ImagesView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listImages: (platformId: string, params: RequestParams = {}) =>
      this.request<ImagesView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/images/${platformId}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name GetExternalRepositories
     * @summary List external repositories of the given registry
     * @request GET:/api/v1/images/{registryName}/repositories
     * @secure
     * @response `200` `(IImageRepository)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getExternalRepositories: (
      registryName: string,
      params: RequestParams = {},
    ) =>
      this.request<
        IImageRepository[],
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/images/${registryName}/repositories`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name GetGhcrPackageVersions
     * @summary List versions of GHCR package
     * @request GET:/api/v1/images/ghcr/{registryName}/{packageName}/versions
     * @secure
     * @response `200` `(GitHubCrPackageVersion)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getGhcrPackageVersions: (
      registryName: string,
      packageName: string,
      params: RequestParams = {},
    ) =>
      this.request<
        GitHubCrPackageVersion[],
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/images/ghcr/${registryName}/${packageName}/versions`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name GetDockerHubRepositories
     * @summary List DockerHub repositories
     * @request GET:/api/v1/images/dockerhub/{registryName}/repositories
     * @secure
     * @response `200` `(DockerHubRepositoryInfo)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getDockerHubRepositories: (
      registryName: string,
      params: RequestParams = {},
    ) =>
      this.request<
        DockerHubRepositoryInfo[],
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/images/dockerhub/${registryName}/repositories`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name GetDockerHubRepositoryTags
     * @summary List DockerHub repository tags
     * @request GET:/api/v1/images/dockerhub/{registryName}/{repositoryName}/tags
     * @secure
     * @response `200` `(DockerHubTagView)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getDockerHubRepositoryTags: (
      registryName: string,
      repositoryName: string,
      params: RequestParams = {},
    ) =>
      this.request<
        DockerHubTagView[],
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/images/dockerhub/${registryName}/${repositoryName}/tags`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name InspectImage
     * @summary Inspect an image
     * @request GET:/api/v1/images/{platformId}/{imageId}
     * @secure
     * @response `200` `InspectImageView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    inspectImage: (
      platformId: string,
      imageId: string,
      params: RequestParams = {},
    ) =>
      this.request<
        InspectImageView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/images/${platformId}/${imageId}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name GetExposedPorts
     * @summary Get image ports
     * @request GET:/api/v1/images/{platformId}/{imageId}/_ports
     * @secure
     * @response `200` `ExposedPortsResult` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getExposedPorts: (
      platformId: string,
      imageId: string,
      params: RequestParams = {},
    ) =>
      this.request<
        ExposedPortsResult,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/images/${platformId}/${imageId}/_ports`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name PullImage
     * @summary Pull an image from a registry and streams execution logs in real time.
     * @request POST:/api/v1/images/pull
     * @secure
     * @response `200` `(PullImageStreamItem)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    pullImage: (data: PullImageInput, params: RequestParams = {}) =>
      this.request<
        PullImageStreamItem[],
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/images/pull`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name DeleteImages
     * @summary Remove an image(s), along with any untagged parent images that were referenced by that image
     * @request DELETE:/api/v1/images
     * @secure
     * @response `200` `DeleteImageResult` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteImages: (data: DeleteImagesRequest, params: RequestParams = {}) =>
      this.request<
        DeleteImageResult,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/images`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Networks
     * @name ListNetworks
     * @summary List all networks
     * @request GET:/api/v1/networks/{platformId}
     * @secure
     * @response `200` `NetworksView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listNetworks: (
      platformId: string,
      query?: {
        Dangling?: boolean;
        Driver?: string;
        Id?: string;
        Name?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<NetworksView, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/networks/${platformId}`,
          method: "GET",
          query: query,
          secure: true,
          format: "json",
          ...params,
        },
      ),

    /**
     * No description
     *
     * @tags Networks
     * @name InspectNetwork
     * @summary Inspect a network
     * @request GET:/api/v1/networks/{platformId}/{networkId}
     * @secure
     * @response `200` `DockerNetworkDetailsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    inspectNetwork: (
      platformId: string,
      networkId: string,
      params: RequestParams = {},
    ) =>
      this.request<
        DockerNetworkDetailsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/networks/${platformId}/${networkId}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Networks
     * @name CreateNetwork
     * @summary Create a network
     * @request POST:/api/v1/networks
     * @secure
     * @response `200` `CreateNetworkView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createNetwork: (data: CreateNetworkInput, params: RequestParams = {}) =>
      this.request<
        CreateNetworkView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/networks`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Networks
     * @name DeleteNetworks
     * @summary Delete a network(s)
     * @request DELETE:/api/v1/networks
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteNetworks: (data: DeleteNetworksInput, params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/networks`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Volumes
     * @name ListVolumes
     * @summary List all volumes
     * @request GET:/api/v1/volumes/{platformId}
     * @secure
     * @response `200` `VolumesView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listVolumes: (
      platformId: string,
      query?: {
        Dangling?: boolean;
        Driver?: string;
        Name?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<VolumesView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/volumes/${platformId}`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Volumes
     * @name InspectVolume
     * @summary Inspect a volume
     * @request GET:/api/v1/volumes/{platformId}/{name}
     * @secure
     * @response `200` `DockerVolumeResultView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    inspectVolume: (
      platformId: string,
      name: string,
      params: RequestParams = {},
    ) =>
      this.request<
        DockerVolumeResultView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/volumes/${platformId}/${name}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Volumes
     * @name CreateVolume
     * @summary Create a volume
     * @request POST:/api/v1/volumes
     * @secure
     * @response `200` `DockerVolumeResultView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createVolume: (data: CreateVolumeInput, params: RequestParams = {}) =>
      this.request<
        DockerVolumeResultView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/volumes`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Volumes
     * @name DeleteVolumes
     * @summary Delete a volume(s)
     * @request DELETE:/api/v1/volumes
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteVolumes: (data: DeleteVolumesInput, params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/volumes`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name ListDeployments
     * @summary List all deployments
     * @request GET:/api/v1/deployments
     * @secure
     * @response `200` `DeploymentsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listDeployments: (
      query?: {
        tags?: string[];
        /** @format uuid */
        platformId?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<
        DeploymentsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/deployments`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name CreateDeployment
     * @summary Create a deployment
     * @request POST:/api/v1/deployments
     * @secure
     * @response `200` `DeploymentView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createDeployment: (
      data: CreateDeploymentInput,
      params: RequestParams = {},
    ) =>
      this.request<
        DeploymentView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/deployments`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name DeleteDeployments
     * @summary Delete deployments
     * @request DELETE:/api/v1/deployments
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteDeployments: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/deployments`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name GetDeployment
     * @summary Get deployment by Id
     * @request GET:/api/v1/deployments/{deploymentId}
     * @secure
     * @response `200` `DeploymentView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getDeployment: (deploymentId: string, params: RequestParams = {}) =>
      this.request<
        DeploymentView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/deployments/${deploymentId}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name GetDeploymentTags
     * @summary Get deployment tags
     * @request GET:/api/v1/deployments/{deploymentId}/tags
     * @secure
     * @response `200` `ResourceTagsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getDeploymentTags: (deploymentId: string, params: RequestParams = {}) =>
      this.request<
        ResourceTagsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/deployments/${deploymentId}/tags`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name ReplaceDeploymentTags
     * @summary Replace deployment tags
     * @request PUT:/api/v1/deployments/{deploymentId}/tags
     * @secure
     * @response `200` `ResourceTagsView` OK
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    replaceDeploymentTags: (
      deploymentId: string,
      data: ReplaceResourceTagsInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceTagsView, ProblemDetails>({
        path: `/api/v1/deployments/${deploymentId}/tags`,
        method: "PUT",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name GetDeploymentStats
     * @summary Get deployment stats
     * @request GET:/api/v1/deployments/{id}/stats
     * @secure
     * @response `200` `ContainerStatsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getDeploymentStats: (
      id: string,
      query?: {
        /**
         * Stats lookback window in hours. Supported values: 24, 48, 72.
         * @format int32
         * @default 24
         * @pattern ^-?(?:0|[1-9]\d*)$
         */
        hours?: number | string;
      },
      params: RequestParams = {},
    ) =>
      this.request<
        ContainerStatsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/deployments/${id}/stats`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name GetDeploymentConfig
     * @summary Get deployment configuration
     * @request GET:/api/v1/deployments/{deploymentId}/_cfg
     * @secure
     * @response `200` `DeploymentConfigView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getDeploymentConfig: (deploymentId: string, params: RequestParams = {}) =>
      this.request<
        DeploymentConfigView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/deployments/${deploymentId}/_cfg`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name GetDeploymentDuplicateDraft
     * @summary Get deployment duplicate draft
     * @request GET:/api/v1/deployments/{deploymentId}/duplicate-draft
     * @secure
     * @response `200` `DeploymentDuplicateDraftView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getDeploymentDuplicateDraft: (
      deploymentId: string,
      params: RequestParams = {},
    ) =>
      this.request<
        DeploymentDuplicateDraftView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/deployments/${deploymentId}/duplicate-draft`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name GetDeploymentBackupSourcePreview
     * @summary Preview deployment backup source volumes
     * @request GET:/api/v1/deployments/{deploymentId}/backup-source-preview
     * @secure
     * @response `200` `DeploymentBackupSourcePreviewView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getDeploymentBackupSourcePreview: (
      deploymentId: string,
      params: RequestParams = {},
    ) =>
      this.request<
        DeploymentBackupSourcePreviewView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/deployments/${deploymentId}/backup-source-preview`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name GetDeploymentContainerInfo
     * @summary Get basic container details
     * @request GET:/api/v1/deployments/{id}/info
     * @secure
     * @response `200` `ContainerInfoView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getDeploymentContainerInfo: (id: string, params: RequestParams = {}) =>
      this.request<
        ContainerInfoView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/deployments/${id}/info`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name RenameDeployment
     * @summary Rename deployment
     * @request POST:/api/v1/deployments/rename
     * @secure
     * @response `200` `DeploymentView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    renameDeployment: (data: RenameResource, params: RequestParams = {}) =>
      this.request<
        DeploymentView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/deployments/rename`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name UpdateDeployment
     * @summary Update a deployment
     * @request PATCH:/api/v1/deployments/{id}
     * @secure
     * @response `200` `DeploymentView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateDeployment: (
      id: string,
      data: PatchDeploymentInput,
      params: RequestParams = {},
    ) =>
      this.request<
        DeploymentView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/deployments/${id}`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name UpdateDeploymentMetadata
     * @summary Update deployment metadata
     * @request PATCH:/api/v1/deployments/{id}/_metadata
     * @secure
     * @response `200` `DeploymentView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateDeploymentMetadata: (
      id: string,
      data: PatchResourceMetadata,
      params: RequestParams = {},
    ) =>
      this.request<
        DeploymentView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/deployments/${id}/_metadata`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name ApplyDeployment
     * @summary Apply a deployment and streams execution logs in real time.
     * @request POST:/api/v1/deployments/apply
     * @secure
     * @response `200` `(DeploymentStreamItem)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    applyDeployment: (data: ApplyDeploymentInput, params: RequestParams = {}) =>
      this.request<
        DeploymentStreamItem[],
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/deployments/apply`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name ResumeDeployments
     * @summary Resume deployments
     * @request POST:/api/v1/deployments/resume
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    resumeDeployments: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/deployments/resume`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name PauseDeployments
     * @summary Pause deployments
     * @request POST:/api/v1/deployments/pause
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    pauseDeployments: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/deployments/pause`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name RestartDeployments
     * @summary Restart deployments
     * @request POST:/api/v1/deployments/restart
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    restartDeployments: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/deployments/restart`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name StopDeployments
     * @summary Stop deployments
     * @request POST:/api/v1/deployments/stop
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    stopDeployments: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/deployments/stop`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name StartDeployments
     * @summary Start deployments
     * @request POST:/api/v1/deployments/start
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    startDeployments: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/deployments/start`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name InspectDeployment
     * @summary Inspect a deployment
     * @request GET:/api/v1/deployments/{id}/inspect
     * @secure
     * @response `200` `ContainerInspectView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    inspectDeployment: (id: string, params: RequestParams = {}) =>
      this.request<
        ContainerInspectView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/deployments/${id}/inspect`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name ListStacks
     * @summary List all stacks
     * @request GET:/api/v1/stacks
     * @secure
     * @response `200` `StacksView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listStacks: (
      query?: {
        tags?: string[];
        /** @format uuid */
        platformId?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<StacksView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/stacks`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name CreateStack
     * @summary Create a stack
     * @request POST:/api/v1/stacks
     * @secure
     * @response `200` `StackView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createStack: (data: CreateStackInput, params: RequestParams = {}) =>
      this.request<StackView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/stacks`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name DeleteStacks
     * @summary Delete stacks
     * @request DELETE:/api/v1/stacks
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteStacks: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/stacks`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name GetStack
     * @summary Get stack by Id
     * @request GET:/api/v1/stacks/{stackId}
     * @secure
     * @response `200` `StackView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getStack: (stackId: string, params: RequestParams = {}) =>
      this.request<StackView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/stacks/${stackId}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name GetStackTags
     * @summary Get stack tags
     * @request GET:/api/v1/stacks/{stackId}/tags
     * @secure
     * @response `200` `ResourceTagsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getStackTags: (stackId: string, params: RequestParams = {}) =>
      this.request<
        ResourceTagsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/stacks/${stackId}/tags`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name ReplaceStackTags
     * @summary Replace stack tags
     * @request PUT:/api/v1/stacks/{stackId}/tags
     * @secure
     * @response `200` `ResourceTagsView` OK
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    replaceStackTags: (
      stackId: string,
      data: ReplaceResourceTagsInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceTagsView, ProblemDetails>({
        path: `/api/v1/stacks/${stackId}/tags`,
        method: "PUT",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name GetStackConfig
     * @summary Get stack configuration
     * @request GET:/api/v1/stacks/{stackId}/_cfg
     * @secure
     * @response `200` `StackConfigView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getStackConfig: (stackId: string, params: RequestParams = {}) =>
      this.request<
        StackConfigView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/stacks/${stackId}/_cfg`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name GetStackBackupSourcePreview
     * @summary Preview stack backup source volumes
     * @request GET:/api/v1/stacks/{stackId}/backup-source-preview
     * @secure
     * @response `200` `StackBackupSourcePreviewView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getStackBackupSourcePreview: (
      stackId: string,
      params: RequestParams = {},
    ) =>
      this.request<
        StackBackupSourcePreviewView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/stacks/${stackId}/backup-source-preview`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name GetStackDuplicateDraft
     * @summary Get stack duplicate draft
     * @request GET:/api/v1/stacks/{stackId}/duplicate-draft
     * @secure
     * @response `200` `StackDuplicateDraftView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getStackDuplicateDraft: (stackId: string, params: RequestParams = {}) =>
      this.request<
        StackDuplicateDraftView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/stacks/${stackId}/duplicate-draft`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name ListStackReleases
     * @summary List stack releases
     * @request GET:/api/v1/stacks/{stackId}/releases
     * @secure
     * @response `200` `StackReleasesView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listStackReleases: (stackId: string, params: RequestParams = {}) =>
      this.request<
        StackReleasesView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/stacks/${stackId}/releases`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name RenameStack
     * @summary Rename a stack
     * @request POST:/api/v1/stacks/rename
     * @secure
     * @response `200` `StackView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    renameStack: (data: RenameResource, params: RequestParams = {}) =>
      this.request<StackView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/stacks/rename`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name UpdateStack
     * @summary Update a stack
     * @request PATCH:/api/v1/stacks/{id}
     * @secure
     * @response `200` `StackView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateStack: (
      id: string,
      data: PatchStackInput,
      params: RequestParams = {},
    ) =>
      this.request<StackView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/stacks/${id}`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name UpdateStackMetadata
     * @summary Update stack metadata
     * @request PATCH:/api/v1/stacks/{id}/_metadata
     * @secure
     * @response `200` `StackView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateStackMetadata: (
      id: string,
      data: PatchResourceMetadata,
      params: RequestParams = {},
    ) =>
      this.request<StackView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/stacks/${id}/_metadata`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name ApplyStack
     * @summary Apply a stack and streams execution logs in real time.
     * @request POST:/api/v1/stacks/apply
     * @secure
     * @response `200` `(StackStreamItem)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    applyStack: (data: ApplyStackInput, params: RequestParams = {}) =>
      this.request<
        StackStreamItem[],
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/stacks/apply`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name RollbackStack
     * @summary Rollback a stack to a previous release and stream execution logs in real time.
     * @request POST:/api/v1/stacks/rollback
     * @secure
     * @response `200` `(StackStreamItem)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    rollbackStack: (data: RollbackStackInput, params: RequestParams = {}) =>
      this.request<
        StackStreamItem[],
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/stacks/rollback`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name StopStacks
     * @summary Stop stacks
     * @request POST:/api/v1/stacks/stop
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    stopStacks: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/stacks/stop`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name StartStacks
     * @summary Start stacks
     * @request POST:/api/v1/stacks/start
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    startStacks: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/stacks/start`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name PauseStacks
     * @summary Pause stacks
     * @request POST:/api/v1/stacks/pause
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    pauseStacks: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/stacks/pause`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name ResumeStacks
     * @summary Resume stacks
     * @request POST:/api/v1/stacks/resume
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    resumeStacks: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/stacks/resume`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name RestartStacks
     * @summary Restart stacks
     * @request POST:/api/v1/stacks/restart
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    restartStacks: (data: string[], params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/stacks/restart`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name GetContainersData
     * @summary Get containers data
     * @request GET:/api/v1/stacks/{stackId}/data
     * @secure
     * @response `200` `ContainersDataView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getContainersData: (stackId: string, params: RequestParams = {}) =>
      this.request<
        ContainersDataView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/stacks/${stackId}/data`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name GetStackStats
     * @summary Get stack stats
     * @request GET:/api/v1/stacks/{stackId}/stats
     * @secure
     * @response `200` `StackStatsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getStackStats: (
      stackId: string,
      query?: {
        /**
         * Stats lookback window in hours. Supported values: 24, 48, 72.
         * @format int32
         * @default 24
         * @pattern ^-?(?:0|[1-9]\d*)$
         */
        hours?: number | string;
      },
      params: RequestParams = {},
    ) =>
      this.request<
        StackStatsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/stacks/${stackId}/stats`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name GetStackDrift
     * @summary Get stack drift report
     * @request GET:/api/v1/stacks/{stackId}/drift
     * @secure
     * @response `200` `StackDriftReport` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getStackDrift: (stackId: string, params: RequestParams = {}) =>
      this.request<
        StackDriftReport,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/stacks/${stackId}/drift`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name UpdateStackDriftPolicy
     * @summary Update stack drift policy
     * @request PUT:/api/v1/stacks/{stackId}/drift-policy
     * @secure
     * @response `200` `StackView` OK
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateStackDriftPolicy: (
      stackId: string,
      data: StackDriftPolicyInput,
      params: RequestParams = {},
    ) =>
      this.request<StackView, ProblemDetails>({
        path: `/api/v1/stacks/${stackId}/drift-policy`,
        method: "PUT",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name ReconcileStack
     * @summary Reconcile safe stack drift
     * @request POST:/api/v1/stacks/{stackId}/reconcile
     * @secure
     * @response `200` `StackReconciliationResult` OK
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    reconcileStack: (stackId: string, params: RequestParams = {}) =>
      this.request<StackReconciliationResult, ProblemDetails>({
        path: `/api/v1/stacks/${stackId}/reconcile`,
        method: "POST",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name InspectStackContainer
     * @summary Inspect a stack container
     * @request GET:/api/v1/stacks/{stackId}/containers/{containerId}/inspect
     * @secure
     * @response `200` `ContainerInspectView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    inspectStackContainer: (
      stackId: string,
      containerId: string,
      params: RequestParams = {},
    ) =>
      this.request<
        ContainerInspectView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/stacks/${stackId}/containers/${containerId}/inspect`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name GetGlobalResourceBindings
     * @summary Get global bindings
     * @request GET:/api/v1/resourceBindings/global
     * @response `200` `ResourceBindingsView` OK
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getGlobalResourceBindings: (params: RequestParams = {}) =>
      this.request<ResourceBindingsView, ProblemDetails>({
        path: `/api/v1/resourceBindings/global`,
        method: "GET",
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name CreateGlobalResourceBinding
     * @summary Create a global binding
     * @request POST:/api/v1/resourceBindings/global
     * @response `200` `ResourceBindingsView` OK
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createGlobalResourceBinding: (
      data: ResourceBindingInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceBindingsView, ProblemDetails>({
        path: `/api/v1/resourceBindings/global`,
        method: "POST",
        body: data,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name UpdateGlobalResourceBinding
     * @summary Update a global binding
     * @request PATCH:/api/v1/resourceBindings/global
     * @response `200` `ResourceBindingsView` OK
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateGlobalResourceBinding: (
      data: UpdateResourceBindingInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceBindingsView, ProblemDetails>({
        path: `/api/v1/resourceBindings/global`,
        method: "PATCH",
        body: data,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name DeleteGlobalResourceBinding
     * @summary Delete a global binding
     * @request DELETE:/api/v1/resourceBindings/global/{id}
     * @response `200` `ResourceBindingsView` OK
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteGlobalResourceBinding: (id: string, params: RequestParams = {}) =>
      this.request<ResourceBindingsView, ProblemDetails>({
        path: `/api/v1/resourceBindings/global/${id}`,
        method: "DELETE",
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name GetResourceBindings
     * @summary Get resource bindings
     * @request GET:/api/v1/resourceBindings/{scope}/{resourceId}
     * @response `200` `ResourceBindingsView` OK
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getResourceBindings: (
      scope: ResourceBindingScope,
      resourceId: string,
      params: RequestParams = {},
    ) =>
      this.request<ResourceBindingsView, ProblemDetails>({
        path: `/api/v1/resourceBindings/${scope}/${resourceId}`,
        method: "GET",
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name CreateResourceBinding
     * @summary Create a resource binding
     * @request POST:/api/v1/resourceBindings/{scope}/{resourceId}
     * @response `200` `ResourceBindingsView` OK
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createResourceBinding: (
      scope: ResourceBindingScope,
      resourceId: string,
      data: ResourceBindingInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceBindingsView, ProblemDetails>({
        path: `/api/v1/resourceBindings/${scope}/${resourceId}`,
        method: "POST",
        body: data,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name UpdateResourceBinding
     * @summary Update a resource binding
     * @request PATCH:/api/v1/resourceBindings/{scope}/{resourceId}
     * @response `200` `ResourceBindingsView` OK
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateResourceBinding: (
      scope: ResourceBindingScope,
      resourceId: string,
      data: UpdateResourceBindingInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceBindingsView, ProblemDetails>({
        path: `/api/v1/resourceBindings/${scope}/${resourceId}`,
        method: "PATCH",
        body: data,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name DeleteResourceBinding
     * @summary Delete a resource binding
     * @request DELETE:/api/v1/resourceBindings/{scope}/{resourceId}/{id}
     * @response `200` `ResourceBindingsView` OK
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteResourceBinding: (
      scope: ResourceBindingScope,
      resourceId: string,
      id: string,
      params: RequestParams = {},
    ) =>
      this.request<ResourceBindingsView, ProblemDetails>({
        path: `/api/v1/resourceBindings/${scope}/${resourceId}/${id}`,
        method: "DELETE",
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name ListSecretDefinitions
     * @summary List secret definitions
     * @request GET:/api/v1/resourceBindings/secrets
     * @response `200` `SecretDefinitionsView` OK
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listSecretDefinitions: (
      query?: {
        /** Optional resource binding scope */
        scope?: ResourceBindingScope;
        /**
         * Optional resource ID
         * @format uuid
         */
        resourceId?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<SecretDefinitionsView, ProblemDetails>({
        path: `/api/v1/resourceBindings/secrets`,
        method: "GET",
        query: query,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name CreateInternalSecret
     * @summary Create an internal encrypted secret
     * @request POST:/api/v1/resourceBindings/secrets
     * @response `200` `SecretDefinitionView` OK
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createInternalSecret: (
      data: CreateInternalSecretInput,
      params: RequestParams = {},
    ) =>
      this.request<SecretDefinitionView, ProblemDetails>({
        path: `/api/v1/resourceBindings/secrets`,
        method: "POST",
        body: data,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name CreateExternalSecret
     * @summary Create an external secret definition
     * @request POST:/api/v1/resourceBindings/secrets/external
     * @response `200` `SecretDefinitionView` OK
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createExternalSecret: (
      data: CreateExternalSecretInput,
      params: RequestParams = {},
    ) =>
      this.request<SecretDefinitionView, ProblemDetails>({
        path: `/api/v1/resourceBindings/secrets/external`,
        method: "POST",
        body: data,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name UpdateExternalSecret
     * @summary Update an external secret definition
     * @request PATCH:/api/v1/resourceBindings/secrets/external/{id}
     * @response `200` `SecretDefinitionView` OK
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateExternalSecret: (
      id: string,
      data: UpdateExternalSecretInput,
      params: RequestParams = {},
    ) =>
      this.request<SecretDefinitionView, ProblemDetails>({
        path: `/api/v1/resourceBindings/secrets/external/${id}`,
        method: "PATCH",
        body: data,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name DeleteSecretDefinition
     * @summary Delete an unused stored secret definition
     * @request DELETE:/api/v1/resourceBindings/secrets/{id}
     * @response `204` `void` No Content
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteSecretDefinition: (id: string, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/resourceBindings/secrets/${id}`,
        method: "DELETE",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name TestExternalSecret
     * @summary Test an external secret reference
     * @request POST:/api/v1/resourceBindings/secrets/external/test
     * @response `200` `ExternalSecretTestResultView` OK
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    testExternalSecret: (
      data: TestExternalSecretInput,
      params: RequestParams = {},
    ) =>
      this.request<ExternalSecretTestResultView, ProblemDetails>({
        path: `/api/v1/resourceBindings/secrets/external/test`,
        method: "POST",
        body: data,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name TestVaultKvV2SecretProviderConnection
     * @summary Test a Vault-compatible KV v2 secret provider connection
     * @request POST:/api/v1/resourceBindings/secret-providers/vault-kv2/test
     * @response `200` `SecretProviderConnectionTestResultView` OK
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    testVaultKvV2SecretProviderConnection: (
      data: TestVaultKvV2SecretProviderConnectionInput,
      params: RequestParams = {},
    ) =>
      this.request<SecretProviderConnectionTestResultView, ProblemDetails>({
        path: `/api/v1/resourceBindings/secret-providers/vault-kv2/test`,
        method: "POST",
        body: data,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name ListSecretProviders
     * @summary List secret providers
     * @request GET:/api/v1/resourceBindings/secret-providers
     * @response `200` `SecretProvidersView` OK
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listSecretProviders: (params: RequestParams = {}) =>
      this.request<SecretProvidersView, ProblemDetails>({
        path: `/api/v1/resourceBindings/secret-providers`,
        method: "GET",
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name CreateVaultKvV2SecretProvider
     * @summary Create a Vault-compatible KV v2 secret provider
     * @request POST:/api/v1/resourceBindings/secret-providers/vault-kv2
     * @response `200` `SecretProviderView` OK
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createVaultKvV2SecretProvider: (
      data: CreateVaultKvV2SecretProviderInput,
      params: RequestParams = {},
    ) =>
      this.request<SecretProviderView, ProblemDetails>({
        path: `/api/v1/resourceBindings/secret-providers/vault-kv2`,
        method: "POST",
        body: data,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name UpdateVaultKvV2SecretProvider
     * @summary Update a Vault-compatible KV v2 secret provider
     * @request PATCH:/api/v1/resourceBindings/secret-providers/vault-kv2/{id}
     * @response `200` `SecretProviderView` OK
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateVaultKvV2SecretProvider: (
      id: string,
      data: UpdateVaultKvV2SecretProviderInput,
      params: RequestParams = {},
    ) =>
      this.request<SecretProviderView, ProblemDetails>({
        path: `/api/v1/resourceBindings/secret-providers/vault-kv2/${id}`,
        method: "PATCH",
        body: data,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name DeleteSecretProvider
     * @summary Delete a secret provider
     * @request DELETE:/api/v1/resourceBindings/secret-providers/{id}
     * @response `204` `void` No Content
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteSecretProvider: (id: string, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/resourceBindings/secret-providers/${id}`,
        method: "DELETE",
        ...params,
      }),

    /**
     * No description
     *
     * @tags OidcProviders
     * @name ListOidcProviders
     * @summary List OIDC providers
     * @request GET:/api/v1/oidcProviders
     * @secure
     * @response `200` `OidcProvidersView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listOidcProviders: (params: RequestParams = {}) =>
      this.request<
        OidcProvidersView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/oidcProviders`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags OidcProviders
     * @name CreateOidcProvider
     * @summary Create OIDC provider
     * @request POST:/api/v1/oidcProviders
     * @secure
     * @response `200` `OidcProviderView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createOidcProvider: (data: OidcProviderInput, params: RequestParams = {}) =>
      this.request<
        OidcProviderView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/oidcProviders`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags OidcProviders
     * @name GetOidcProvider
     * @summary Get OIDC provider
     * @request GET:/api/v1/oidcProviders/{id}
     * @secure
     * @response `200` `OidcProviderView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getOidcProvider: (id: string, params: RequestParams = {}) =>
      this.request<
        OidcProviderView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/oidcProviders/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags OidcProviders
     * @name UpdateOidcProvider
     * @summary Update OIDC provider
     * @request PATCH:/api/v1/oidcProviders/{id}
     * @secure
     * @response `200` `OidcProviderView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateOidcProvider: (
      id: string,
      data: UpdateOidcProviderInput,
      params: RequestParams = {},
    ) =>
      this.request<
        OidcProviderView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/oidcProviders/${id}`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags OidcProviders
     * @name DeleteOidcProvider
     * @summary Delete OIDC provider
     * @request DELETE:/api/v1/oidcProviders/{id}
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteOidcProvider: (id: string, params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/oidcProviders/${id}`,
        method: "DELETE",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags OidcProviders
     * @name RenameOidcProvider
     * @summary Rename OIDC provider
     * @request POST:/api/v1/oidcProviders/rename
     * @secure
     * @response `200` `OidcProviderView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    renameOidcProvider: (data: RenameResource, params: RequestParams = {}) =>
      this.request<
        OidcProviderView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/oidcProviders/rename`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags OidcProviders
     * @name UpdateOidcProviderMetadata
     * @summary Update OIDC provider metadata
     * @request PATCH:/api/v1/oidcProviders/{id}/_metadata
     * @secure
     * @response `200` `OidcProviderView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateOidcProviderMetadata: (
      id: string,
      data: PatchResourceMetadata,
      params: RequestParams = {},
    ) =>
      this.request<
        OidcProviderView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/oidcProviders/${id}/_metadata`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags OidcProviders
     * @name TestOidcProviderDiscovery
     * @summary Test OIDC provider discovery
     * @request POST:/api/v1/oidcProviders/{id}/testDiscovery
     * @secure
     * @response `200` `OidcDiscoveryResultView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    testOidcProviderDiscovery: (id: string, params: RequestParams = {}) =>
      this.request<
        OidcDiscoveryResultView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/oidcProviders/${id}/testDiscovery`,
        method: "POST",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags OidcProviders
     * @name TestOidcDiscovery
     * @summary Test OIDC discovery
     * @request POST:/api/v1/oidcProviders/testDiscovery
     * @secure
     * @response `200` `OidcDiscoveryResultView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    testOidcDiscovery: (
      data: TestOidcProviderDiscoveryInput,
      params: RequestParams = {},
    ) =>
      this.request<
        OidcDiscoveryResultView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/oidcProviders/testDiscovery`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AutomationActions
     * @name ListAutomationActions
     * @summary List automation actions
     * @request GET:/api/v1/automation/actions
     * @secure
     * @response `200` `AutomationActionsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listAutomationActions: (
      query?: {
        tags?: string[];
      },
      params: RequestParams = {},
    ) =>
      this.request<
        AutomationActionsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/automation/actions`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AutomationActions
     * @name CreateAutomationAction
     * @summary Create automation action
     * @request POST:/api/v1/automation/actions
     * @secure
     * @response `200` `AutomationActionView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createAutomationAction: (
      data: AutomationActionInput,
      params: RequestParams = {},
    ) =>
      this.request<
        AutomationActionView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/automation/actions`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AutomationActions
     * @name GetAutomationAction
     * @summary Get automation action
     * @request GET:/api/v1/automation/actions/{id}
     * @secure
     * @response `200` `AutomationActionView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getAutomationAction: (id: string, params: RequestParams = {}) =>
      this.request<
        AutomationActionView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/automation/actions/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AutomationActions
     * @name UpdateAutomationAction
     * @summary Update automation action
     * @request PATCH:/api/v1/automation/actions/{id}
     * @secure
     * @response `200` `AutomationActionView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateAutomationAction: (
      id: string,
      data: UpdateAutomationActionInput,
      params: RequestParams = {},
    ) =>
      this.request<
        AutomationActionView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/automation/actions/${id}`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AutomationActions
     * @name DeleteAutomationAction
     * @summary Delete automation action
     * @request DELETE:/api/v1/automation/actions/{id}
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteAutomationAction: (id: string, params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/automation/actions/${id}`,
        method: "DELETE",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags AutomationActions
     * @name GetAutomationActionTags
     * @summary Get automation action tags
     * @request GET:/api/v1/automation/actions/{id}/tags
     * @secure
     * @response `200` `ResourceTagsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getAutomationActionTags: (id: string, params: RequestParams = {}) =>
      this.request<
        ResourceTagsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/automation/actions/${id}/tags`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AutomationActions
     * @name ReplaceAutomationActionTags
     * @summary Replace automation action tags
     * @request PUT:/api/v1/automation/actions/{id}/tags
     * @secure
     * @response `200` `ResourceTagsView` OK
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    replaceAutomationActionTags: (
      id: string,
      data: ReplaceResourceTagsInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceTagsView, ProblemDetails>({
        path: `/api/v1/automation/actions/${id}/tags`,
        method: "PUT",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AutomationActions
     * @name RenameAutomationAction
     * @summary Rename automation action
     * @request POST:/api/v1/automation/actions/rename
     * @secure
     * @response `200` `AutomationActionView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    renameAutomationAction: (
      data: RenameResource,
      params: RequestParams = {},
    ) =>
      this.request<
        AutomationActionView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/automation/actions/rename`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AutomationActions
     * @name UpdateAutomationActionMetadata
     * @summary Update automation action metadata
     * @request PATCH:/api/v1/automation/actions/{id}/_metadata
     * @secure
     * @response `200` `AutomationActionView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateAutomationActionMetadata: (
      id: string,
      data: PatchResourceMetadata,
      params: RequestParams = {},
    ) =>
      this.request<
        AutomationActionView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/automation/actions/${id}/_metadata`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AutomationActions
     * @name RunAutomationAction
     * @summary Queue automation action run
     * @request POST:/api/v1/automation/actions/{id}/run
     * @secure
     * @response `200` `(AutomationActionRunStreamItem)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    runAutomationAction: (
      id: string,
      data: RunAutomationActionInput,
      params: RequestParams = {},
    ) =>
      this.request<
        AutomationActionRunStreamItem[],
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/automation/actions/${id}/run`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AutomationActions
     * @name TestAutomationAction
     * @summary Queue automation action test run
     * @request POST:/api/v1/automation/actions/{id}/test
     * @secure
     * @response `200` `(AutomationActionRunStreamItem)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    testAutomationAction: (
      id: string,
      data: TestAutomationActionInput,
      params: RequestParams = {},
    ) =>
      this.request<
        AutomationActionRunStreamItem[],
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/automation/actions/${id}/test`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AutomationActions
     * @name CancelAutomationActionRun
     * @summary Cancel automation action run
     * @request POST:/api/v1/automation/actions/{id}/runs/{runId}/cancel
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    cancelAutomationActionRun: (
      id: string,
      runId: string,
      params: RequestParams = {},
    ) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/automation/actions/${id}/runs/${runId}/cancel`,
        method: "POST",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags AutomationActions
     * @name ListAutomationActionRuns
     * @summary List automation action runs
     * @request GET:/api/v1/automation/actions/{id}/runs
     * @secure
     * @response `200` `AutomationActionRunsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listAutomationActionRuns: (
      id: string,
      query?: {
        /**
         * @format int32
         * @pattern ^-?(?:0|[1-9]\d*)$
         */
        limit?: number | string;
      },
      params: RequestParams = {},
    ) =>
      this.request<
        AutomationActionRunsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/automation/actions/${id}/runs`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AutomationActions
     * @name GetAutomationActionRun
     * @summary Get automation action run
     * @request GET:/api/v1/automation/actions/{id}/runs/{runId}
     * @secure
     * @response `200` `AutomationActionRunView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getAutomationActionRun: (
      id: string,
      runId: string,
      params: RequestParams = {},
    ) =>
      this.request<
        AutomationActionRunView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/automation/actions/${id}/runs/${runId}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AutomationActions
     * @name GetAutomationActionRunLogs
     * @summary Get automation action run logs
     * @request GET:/api/v1/automation/actions/{id}/runs/{runId}/logs
     * @secure
     * @response `200` `AutomationActionRunLogsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getAutomationActionRunLogs: (
      id: string,
      runId: string,
      params: RequestParams = {},
    ) =>
      this.request<
        AutomationActionRunLogsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/automation/actions/${id}/runs/${runId}/logs`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRepositories
     * @name ListBackupRepositories
     * @summary List backup repositories
     * @request GET:/api/v1/backupRepositories
     * @secure
     * @response `200` `BackupRepositoriesView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listBackupRepositories: (params: RequestParams = {}) =>
      this.request<BackupRepositoriesView, ProblemDetails>({
        path: `/api/v1/backupRepositories`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRepositories
     * @name CreateBackupRepository
     * @summary Create backup repository
     * @request POST:/api/v1/backupRepositories
     * @secure
     * @response `200` `BackupRepositoryView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createBackupRepository: (
      data: BackupRepositoryInput,
      params: RequestParams = {},
    ) =>
      this.request<
        BackupRepositoryView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/backupRepositories`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRepositories
     * @name GetBackupRepository
     * @summary Get backup repository
     * @request GET:/api/v1/backupRepositories/{id}
     * @secure
     * @response `200` `BackupRepositoryView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getBackupRepository: (id: string, params: RequestParams = {}) =>
      this.request<BackupRepositoryView, ProblemDetails>({
        path: `/api/v1/backupRepositories/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRepositories
     * @name UpdateBackupRepository
     * @summary Update backup repository
     * @request PATCH:/api/v1/backupRepositories/{id}
     * @secure
     * @response `200` `BackupRepositoryView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateBackupRepository: (
      id: string,
      data: UpdateBackupRepositoryInput,
      params: RequestParams = {},
    ) =>
      this.request<
        BackupRepositoryView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/backupRepositories/${id}`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRepositories
     * @name ArchiveBackupRepository
     * @summary Archive backup repository
     * @request DELETE:/api/v1/backupRepositories/{id}
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    archiveBackupRepository: (id: string, params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/backupRepositories/${id}`,
        method: "DELETE",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRepositories
     * @name ValidateBackupRepository
     * @summary Validate backup repository
     * @request POST:/api/v1/backupRepositories/{id}/validate
     * @secure
     * @response `200` `BackupRepositoryValidationView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    validateBackupRepository: (
      id: string,
      data: ValidateBackupRepositoryInput,
      params: RequestParams = {},
    ) =>
      this.request<
        BackupRepositoryValidationView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/backupRepositories/${id}/validate`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRepositories
     * @name InitializeBackupRepository
     * @summary Initialize backup repository
     * @request POST:/api/v1/backupRepositories/{id}/initialize
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    initializeBackupRepository: (
      id: string,
      data: ValidateBackupRepositoryInput,
      params: RequestParams = {},
    ) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/backupRepositories/${id}/initialize`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRepositories
     * @name CheckBackupRepository
     * @summary Check backup repository
     * @request POST:/api/v1/backupRepositories/{id}/check
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    checkBackupRepository: (
      id: string,
      data: ValidateBackupRepositoryInput,
      params: RequestParams = {},
    ) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/backupRepositories/${id}/check`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRepositories
     * @name PruneBackupRepository
     * @summary Prune backup repository
     * @request POST:/api/v1/backupRepositories/{id}/prune
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    pruneBackupRepository: (
      id: string,
      data: ValidateBackupRepositoryInput,
      params: RequestParams = {},
    ) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/backupRepositories/${id}/prune`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupPolicies
     * @name ListBackupPolicies
     * @summary List backup policies
     * @request GET:/api/v1/backupPolicies
     * @secure
     * @response `200` `BackupPoliciesView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listBackupPolicies: (
      query?: {
        tags?: string[];
      },
      params: RequestParams = {},
    ) =>
      this.request<BackupPoliciesView, ProblemDetails>({
        path: `/api/v1/backupPolicies`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupPolicies
     * @name CreateBackupPolicy
     * @summary Create backup policy
     * @request POST:/api/v1/backupPolicies
     * @secure
     * @response `200` `BackupPolicyView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createBackupPolicy: (data: BackupPolicyInput, params: RequestParams = {}) =>
      this.request<
        BackupPolicyView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/backupPolicies`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupPolicies
     * @name GetBackupPolicy
     * @summary Get backup policy
     * @request GET:/api/v1/backupPolicies/{id}
     * @secure
     * @response `200` `BackupPolicyView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getBackupPolicy: (id: string, params: RequestParams = {}) =>
      this.request<BackupPolicyView, ProblemDetails>({
        path: `/api/v1/backupPolicies/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupPolicies
     * @name UpdateBackupPolicy
     * @summary Update backup policy
     * @request PATCH:/api/v1/backupPolicies/{id}
     * @secure
     * @response `200` `BackupPolicyView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateBackupPolicy: (
      id: string,
      data: UpdateBackupPolicyInput,
      params: RequestParams = {},
    ) =>
      this.request<
        BackupPolicyView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/backupPolicies/${id}`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupPolicies
     * @name ArchiveBackupPolicy
     * @summary Archive backup policy
     * @request DELETE:/api/v1/backupPolicies/{id}
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    archiveBackupPolicy: (id: string, params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/backupPolicies/${id}`,
        method: "DELETE",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupPolicies
     * @name GetBackupPolicyTags
     * @summary Get backup policy tags
     * @request GET:/api/v1/backupPolicies/{id}/tags
     * @secure
     * @response `200` `ResourceTagsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getBackupPolicyTags: (id: string, params: RequestParams = {}) =>
      this.request<
        ResourceTagsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/backupPolicies/${id}/tags`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupPolicies
     * @name ReplaceBackupPolicyTags
     * @summary Replace backup policy tags
     * @request PUT:/api/v1/backupPolicies/{id}/tags
     * @secure
     * @response `200` `ResourceTagsView` OK
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    replaceBackupPolicyTags: (
      id: string,
      data: ReplaceResourceTagsInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceTagsView, ProblemDetails>({
        path: `/api/v1/backupPolicies/${id}/tags`,
        method: "PUT",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupPolicies
     * @name RenameBackupPolicy
     * @summary Rename backup policy
     * @request POST:/api/v1/backupPolicies/rename
     * @secure
     * @response `200` `BackupPolicyView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    renameBackupPolicy: (data: RenameResource, params: RequestParams = {}) =>
      this.request<
        BackupPolicyView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/backupPolicies/rename`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupPolicies
     * @name UpdateBackupPolicyMetadata
     * @summary Update backup policy metadata
     * @request PATCH:/api/v1/backupPolicies/{id}/_metadata
     * @secure
     * @response `200` `BackupPolicyView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateBackupPolicyMetadata: (
      id: string,
      data: PatchResourceMetadata,
      params: RequestParams = {},
    ) =>
      this.request<
        BackupPolicyView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/backupPolicies/${id}/_metadata`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupPolicies
     * @name QueueBackupRun
     * @summary Queue backup policy run
     * @request POST:/api/v1/backupPolicies/{id}/runs
     * @secure
     * @response `200` `BackupRunView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    queueBackupRun: (
      id: string,
      data: QueueBackupRunInput,
      params: RequestParams = {},
    ) =>
      this.request<
        BackupRunView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/backupPolicies/${id}/runs`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupPolicies
     * @name RunBackupPolicy
     * @summary Run backup policy and stream execution logs in real time
     * @request POST:/api/v1/backupPolicies/{id}/run
     * @secure
     * @response `200` `(BackupRunStreamItem)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    runBackupPolicy: (
      id: string,
      data: QueueBackupRunInput,
      params: RequestParams = {},
    ) =>
      this.request<
        BackupRunStreamItem[],
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/backupPolicies/${id}/run`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRuns
     * @name ListBackupRuns
     * @summary List backup runs
     * @request GET:/api/v1/backupRuns
     * @secure
     * @response `200` `BackupRunsView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listBackupRuns: (
      query?: {
        /** @format uuid */
        policyId?: string;
        /**
         * @format int32
         * @pattern ^-?(?:0|[1-9]\d*)$
         */
        limit?: number | string;
      },
      params: RequestParams = {},
    ) =>
      this.request<BackupRunsView, ProblemDetails>({
        path: `/api/v1/backupRuns`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRuns
     * @name GetBackupRun
     * @summary Get backup run
     * @request GET:/api/v1/backupRuns/{id}
     * @secure
     * @response `200` `BackupRunView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getBackupRun: (id: string, params: RequestParams = {}) =>
      this.request<BackupRunView, ProblemDetails>({
        path: `/api/v1/backupRuns/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRuns
     * @name GetBackupRunLogs
     * @summary Get backup run logs
     * @request GET:/api/v1/backupRuns/{id}/logs
     * @secure
     * @response `200` `BackupLogsView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getBackupRunLogs: (id: string, params: RequestParams = {}) =>
      this.request<BackupLogsView, ProblemDetails>({
        path: `/api/v1/backupRuns/${id}/logs`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRuns
     * @name GetBackupRunEvents
     * @summary Get backup run events
     * @request GET:/api/v1/backupRuns/{id}/events
     * @secure
     * @response `200` `BackupEventsView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getBackupRunEvents: (id: string, params: RequestParams = {}) =>
      this.request<BackupEventsView, ProblemDetails>({
        path: `/api/v1/backupRuns/${id}/events`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRuns
     * @name CancelBackupRun
     * @summary Cancel backup run
     * @request POST:/api/v1/backupRuns/{id}/cancel
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    cancelBackupRun: (id: string, params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/backupRuns/${id}/cancel`,
        method: "POST",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRuns
     * @name RestoreBackupVolume
     * @summary Queue backup volume restore
     * @request POST:/api/v1/backupRuns/{id}/restoreVolume
     * @secure
     * @response `200` `BackupRestoreRunView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    restoreBackupVolume: (
      id: string,
      data: RestoreVolumeInput,
      params: RequestParams = {},
    ) =>
      this.request<
        BackupRestoreRunView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/backupRuns/${id}/restoreVolume`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRuns
     * @name RunBackupRestoreVolume
     * @summary Run backup volume restore
     * @request POST:/api/v1/backupRuns/{id}/restoreVolume/run
     * @secure
     * @response `200` `(BackupRestoreRunStreamItem)[]` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    runBackupRestoreVolume: (
      id: string,
      data: RestoreVolumeInput,
      params: RequestParams = {},
    ) =>
      this.request<
        BackupRestoreRunStreamItem[],
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/backupRuns/${id}/restoreVolume/run`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRestoreRuns
     * @name ListBackupRestoreRuns
     * @summary List backup restore runs
     * @request GET:/api/v1/backupRestoreRuns
     * @secure
     * @response `200` `BackupRestoreRunsView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listBackupRestoreRuns: (
      query?: {
        /** @format uuid */
        backupRunId?: string;
        /** @format uuid */
        policyId?: string;
        /**
         * @format int32
         * @pattern ^-?(?:0|[1-9]\d*)$
         */
        limit?: number | string;
      },
      params: RequestParams = {},
    ) =>
      this.request<BackupRestoreRunsView, ProblemDetails>({
        path: `/api/v1/backupRestoreRuns`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRestoreRuns
     * @name GetBackupRestoreRun
     * @summary Get backup restore run
     * @request GET:/api/v1/backupRestoreRuns/{id}
     * @secure
     * @response `200` `BackupRestoreRunView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getBackupRestoreRun: (id: string, params: RequestParams = {}) =>
      this.request<BackupRestoreRunView, ProblemDetails>({
        path: `/api/v1/backupRestoreRuns/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRestoreRuns
     * @name GetBackupRestoreRunLogs
     * @summary Get backup restore run logs
     * @request GET:/api/v1/backupRestoreRuns/{id}/logs
     * @secure
     * @response `200` `BackupLogsView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getBackupRestoreRunLogs: (id: string, params: RequestParams = {}) =>
      this.request<BackupLogsView, ProblemDetails>({
        path: `/api/v1/backupRestoreRuns/${id}/logs`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRestoreRuns
     * @name GetBackupRestoreRunEvents
     * @summary Get backup restore run events
     * @request GET:/api/v1/backupRestoreRuns/{id}/events
     * @secure
     * @response `200` `BackupEventsView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getBackupRestoreRunEvents: (id: string, params: RequestParams = {}) =>
      this.request<BackupEventsView, ProblemDetails>({
        path: `/api/v1/backupRestoreRuns/${id}/events`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRestoreRuns
     * @name CancelBackupRestoreRun
     * @summary Cancel backup restore run
     * @request POST:/api/v1/backupRestoreRuns/{id}/cancel
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    cancelBackupRestoreRun: (id: string, params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/backupRestoreRuns/${id}/cancel`,
        method: "POST",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildProjects
     * @name ListBuildProjects
     * @summary List build projects
     * @request GET:/api/v1/buildProjects
     * @secure
     * @response `200` `BuildProjectsView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listBuildProjects: (
      query?: {
        tags?: string[];
      },
      params: RequestParams = {},
    ) =>
      this.request<BuildProjectsView, ProblemDetails>({
        path: `/api/v1/buildProjects`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildProjects
     * @name CreateBuildProject
     * @summary Create build project
     * @request POST:/api/v1/buildProjects
     * @secure
     * @response `200` `BuildProjectView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createBuildProject: (data: BuildProjectInput, params: RequestParams = {}) =>
      this.request<
        BuildProjectView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/buildProjects`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildProjects
     * @name GetBuildProject
     * @summary Get build project
     * @request GET:/api/v1/buildProjects/{id}
     * @secure
     * @response `200` `BuildProjectView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getBuildProject: (id: string, params: RequestParams = {}) =>
      this.request<BuildProjectView, ProblemDetails>({
        path: `/api/v1/buildProjects/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildProjects
     * @name UpdateBuildProject
     * @summary Update build project
     * @request PATCH:/api/v1/buildProjects/{id}
     * @secure
     * @response `200` `BuildProjectView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateBuildProject: (
      id: string,
      data: UpdateBuildProjectInput,
      params: RequestParams = {},
    ) =>
      this.request<
        BuildProjectView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/buildProjects/${id}`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildProjects
     * @name ArchiveBuildProject
     * @summary Archive build project
     * @request DELETE:/api/v1/buildProjects/{id}
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    archiveBuildProject: (id: string, params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/buildProjects/${id}`,
        method: "DELETE",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildProjects
     * @name GetBuildTags
     * @summary Get build project tags
     * @request GET:/api/v1/buildProjects/{id}/tags
     * @secure
     * @response `200` `ResourceTagsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getBuildTags: (id: string, params: RequestParams = {}) =>
      this.request<
        ResourceTagsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/buildProjects/${id}/tags`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildProjects
     * @name ReplaceBuildTags
     * @summary Replace build project tags
     * @request PUT:/api/v1/buildProjects/{id}/tags
     * @secure
     * @response `200` `ResourceTagsView` OK
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    replaceBuildTags: (
      id: string,
      data: ReplaceResourceTagsInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceTagsView, ProblemDetails>({
        path: `/api/v1/buildProjects/${id}/tags`,
        method: "PUT",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildProjects
     * @name RenameBuild
     * @summary Rename build project
     * @request POST:/api/v1/buildProjects/rename
     * @secure
     * @response `200` `BuildProjectView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    renameBuild: (data: RenameResource, params: RequestParams = {}) =>
      this.request<
        BuildProjectView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/buildProjects/rename`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildProjects
     * @name UpdateBuildMetadata
     * @summary Update build project metadata
     * @request PATCH:/api/v1/buildProjects/{id}/_metadata
     * @secure
     * @response `200` `BuildProjectView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateBuildMetadata: (
      id: string,
      data: PatchResourceMetadata,
      params: RequestParams = {},
    ) =>
      this.request<
        BuildProjectView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/buildProjects/${id}/_metadata`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildProjects
     * @name QueueBuildRun
     * @summary Queue build run
     * @request POST:/api/v1/buildProjects/{id}/runs
     * @secure
     * @response `200` `BuildRunView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    queueBuildRun: (
      id: string,
      data: QueueBuildRunInput,
      params: RequestParams = {},
    ) =>
      this.request<BuildRunView, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/buildProjects/${id}/runs`,
          method: "POST",
          body: data,
          secure: true,
          type: ContentType.Json,
          format: "json",
          ...params,
        },
      ),

    /**
     * No description
     *
     * @tags BuildAgentPools
     * @name ListBuildAgentPools
     * @summary List build pools
     * @request GET:/api/v1/buildAgentPools
     * @secure
     * @response `200` `BuildAgentPoolsView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listBuildAgentPools: (
      query?: {
        tags?: string[];
      },
      params: RequestParams = {},
    ) =>
      this.request<BuildAgentPoolsView, ProblemDetails>({
        path: `/api/v1/buildAgentPools`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildAgentPools
     * @name CreateBuildAgentPool
     * @summary Create build pool
     * @request POST:/api/v1/buildAgentPools
     * @secure
     * @response `200` `BuildAgentPoolView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createBuildAgentPool: (
      data: BuildAgentPoolInput,
      params: RequestParams = {},
    ) =>
      this.request<
        BuildAgentPoolView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/buildAgentPools`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildAgentPools
     * @name GetBuildAgentPool
     * @summary Get build pool
     * @request GET:/api/v1/buildAgentPools/{id}
     * @secure
     * @response `200` `BuildAgentPoolView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getBuildAgentPool: (id: string, params: RequestParams = {}) =>
      this.request<BuildAgentPoolView, ProblemDetails>({
        path: `/api/v1/buildAgentPools/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildAgentPools
     * @name UpdateBuildAgentPool
     * @summary Update build pool
     * @request PATCH:/api/v1/buildAgentPools/{id}
     * @secure
     * @response `200` `BuildAgentPoolView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateBuildAgentPool: (
      id: string,
      data: UpdateBuildAgentPoolInput,
      params: RequestParams = {},
    ) =>
      this.request<
        BuildAgentPoolView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/buildAgentPools/${id}`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildAgentPools
     * @name ArchiveBuildAgentPool
     * @summary Archive build pool
     * @request DELETE:/api/v1/buildAgentPools/{id}
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    archiveBuildAgentPool: (id: string, params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/buildAgentPools/${id}`,
        method: "DELETE",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildAgentPools
     * @name GetBuildAgentPoolTags
     * @summary Get build pool tags
     * @request GET:/api/v1/buildAgentPools/{id}/tags
     * @secure
     * @response `200` `ResourceTagsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getBuildAgentPoolTags: (id: string, params: RequestParams = {}) =>
      this.request<
        ResourceTagsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/buildAgentPools/${id}/tags`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildAgentPools
     * @name ReplaceBuildAgentPoolTags
     * @summary Replace build pool tags
     * @request PUT:/api/v1/buildAgentPools/{id}/tags
     * @secure
     * @response `200` `ResourceTagsView` OK
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    replaceBuildAgentPoolTags: (
      id: string,
      data: ReplaceResourceTagsInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceTagsView, ProblemDetails>({
        path: `/api/v1/buildAgentPools/${id}/tags`,
        method: "PUT",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildAgentPools
     * @name RenameBuildAgentPool
     * @summary Rename build pool
     * @request POST:/api/v1/buildAgentPools/rename
     * @secure
     * @response `200` `BuildAgentPoolView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    renameBuildAgentPool: (data: RenameResource, params: RequestParams = {}) =>
      this.request<
        BuildAgentPoolView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/buildAgentPools/rename`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildAgentPools
     * @name UpdateBuildAgentPoolMetadata
     * @summary Update build pool metadata
     * @request PATCH:/api/v1/buildAgentPools/{id}/_metadata
     * @secure
     * @response `200` `BuildAgentPoolView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateBuildAgentPoolMetadata: (
      id: string,
      data: PatchResourceMetadata,
      params: RequestParams = {},
    ) =>
      this.request<
        BuildAgentPoolView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/buildAgentPools/${id}/_metadata`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildAgentPools
     * @name TestBuildAgentPool
     * @summary Test build pool
     * @request POST:/api/v1/buildAgentPools/{id}/test
     * @secure
     * @response `200` `BuildAgentPoolView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    testBuildAgentPool: (id: string, params: RequestParams = {}) =>
      this.request<
        BuildAgentPoolView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/buildAgentPools/${id}/test`,
        method: "POST",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildAgentPools
     * @name CreateBuildAgentPoolEdgeEnrollment
     * @summary Create build pool Edge Agent enrollment
     * @request POST:/api/v1/buildAgentPools/{id}/edge/enrollments
     * @secure
     * @response `200` `EdgeAgentEnrollmentView` OK
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createBuildAgentPoolEdgeEnrollment: (
      id: string,
      params: RequestParams = {},
    ) =>
      this.request<EdgeAgentEnrollmentView, ProblemDetails>({
        path: `/api/v1/buildAgentPools/${id}/edge/enrollments`,
        method: "POST",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildAgentPools
     * @name GetBuildAgentPoolEdgeStatus
     * @summary Get build pool Edge Agent status
     * @request GET:/api/v1/buildAgentPools/{id}/edge/status
     * @secure
     * @response `200` `EdgeAgentStatusView` OK
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getBuildAgentPoolEdgeStatus: (id: string, params: RequestParams = {}) =>
      this.request<EdgeAgentStatusView, ProblemDetails>({
        path: `/api/v1/buildAgentPools/${id}/edge/status`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildAgentPools
     * @name RevokeBuildAgentPoolEdgeAgent
     * @summary Revoke build pool Edge Agent
     * @request POST:/api/v1/buildAgentPools/{id}/edge/revoke
     * @secure
     * @response `204` `void` No Content
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    revokeBuildAgentPoolEdgeAgent: (id: string, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/buildAgentPools/${id}/edge/revoke`,
        method: "POST",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildRuns
     * @name ListBuildRuns
     * @summary List build runs
     * @request GET:/api/v1/buildRuns
     * @secure
     * @response `200` `BuildRunsView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listBuildRuns: (
      query?: {
        /** @format uuid */
        projectId?: string;
        /**
         * @format int32
         * @pattern ^-?(?:0|[1-9]\d*)$
         */
        limit?: number | string;
      },
      params: RequestParams = {},
    ) =>
      this.request<BuildRunsView, ProblemDetails>({
        path: `/api/v1/buildRuns`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildRuns
     * @name GetBuildRun
     * @summary Get build run
     * @request GET:/api/v1/buildRuns/{id}
     * @secure
     * @response `200` `BuildRunView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getBuildRun: (id: string, params: RequestParams = {}) =>
      this.request<BuildRunView, ProblemDetails>({
        path: `/api/v1/buildRuns/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildRuns
     * @name GetBuildRunLogs
     * @summary Get build run logs
     * @request GET:/api/v1/buildRuns/{id}/logs
     * @secure
     * @response `200` `BuildLogsView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getBuildRunLogs: (id: string, params: RequestParams = {}) =>
      this.request<BuildLogsView, ProblemDetails>({
        path: `/api/v1/buildRuns/${id}/logs`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildRuns
     * @name CancelBuildRun
     * @summary Cancel build run
     * @request POST:/api/v1/buildRuns/{id}/cancel
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    cancelBuildRun: (id: string, params: RequestParams = {}) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/buildRuns/${id}/cancel`,
        method: "POST",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Tags
     * @name ListTags
     * @summary List resource tags
     * @request GET:/api/v1/tags
     * @response `200` `TagsView` OK
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listTags: (params: RequestParams = {}) =>
      this.request<TagsView, ProblemDetails>({
        path: `/api/v1/tags`,
        method: "GET",
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Tags
     * @name CreateTag
     * @summary Create a resource tag
     * @request POST:/api/v1/tags
     * @secure
     * @response `200` `TagView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createTag: (data: CreateTagInput, params: RequestParams = {}) =>
      this.request<TagView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/tags`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Tags
     * @name PatchTag
     * @summary Update a resource tag
     * @request PATCH:/api/v1/tags/{id}
     * @secure
     * @response `200` `TagView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    patchTag: (id: string, data: PatchTagInput, params: RequestParams = {}) =>
      this.request<TagView, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/tags/${id}`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Tags
     * @name DeleteTag
     * @summary Delete a resource tag
     * @request DELETE:/api/v1/tags/{id}
     * @secure
     * @response `204` `void` No Content
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteTag: (id: string, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/tags/${id}`,
        method: "DELETE",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Activities
     * @name GetActivity
     * @summary Get activity by id
     * @request GET:/api/v1/activities/{id}
     * @secure
     * @response `200` `ActivityView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getActivity: (id: string, params: RequestParams = {}) =>
      this.request<ActivityView, HttpValidationProblemDetails | ProblemDetails>(
        {
          path: `/api/v1/activities/${id}`,
          method: "GET",
          secure: true,
          format: "json",
          ...params,
        },
      ),

    /**
     * No description
     *
     * @tags Activities
     * @name ListActivities
     * @summary List activity events
     * @request GET:/api/v1/activities
     * @secure
     * @response `200` `ActivitiesView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listActivities: (
      query?: {
        /** @format uuid */
        ResourceId?: string;
        ResourceType?: ActivityResourceType;
        EventType?: ActivityEventType;
        /**
         * @format int32
         * @default 1
         * @pattern ^-?(?:0|[1-9]\d*)$
         */
        Page?: number | string;
        /**
         * @format int32
         * @default 50
         * @pattern ^-?(?:0|[1-9]\d*)$
         */
        PageSize?: number | string;
      },
      params: RequestParams = {},
    ) =>
      this.request<
        ActivitiesView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/activities`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AlertEvents
     * @name GetAlertEvent
     * @summary Get alert event by id
     * @request GET:/api/v1/alertEvents/{id}
     * @secure
     * @response `200` `AlertEventView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getAlertEvent: (id: string, params: RequestParams = {}) =>
      this.request<
        AlertEventView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/alertEvents/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AlertEvents
     * @name ListAlertEvents
     * @summary List alert events
     * @request GET:/api/v1/alertEvents
     * @secure
     * @response `200` `AlertEventsView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listAlertEvents: (
      query?: {
        /** @format uuid */
        ResourceId?: string;
        AlertType?: AlertType;
        ResourceType?: AlertResourceType;
        UnresolvedOnly?: boolean;
        /**
         * @format int32
         * @default 1
         * @pattern ^-?(?:0|[1-9]\d*)$
         */
        Page?: number | string;
        /**
         * @format int32
         * @default 50
         * @pattern ^-?(?:0|[1-9]\d*)$
         */
        PageSize?: number | string;
      },
      params: RequestParams = {},
    ) =>
      this.request<
        AlertEventsView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/alertEvents`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AlertEvents
     * @name GetUnresolvedAlertEventsCount
     * @summary Get unresolved alert event count
     * @request GET:/api/v1/alertEvents/unresolved-count
     * @secure
     * @response `200` `UnresolvedAlertsCountView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getUnresolvedAlertEventsCount: (params: RequestParams = {}) =>
      this.request<UnresolvedAlertsCountView, ProblemDetails>({
        path: `/api/v1/alertEvents/unresolved-count`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AlertEvents
     * @name AcknowledgeAlertEvents
     * @summary Acknowledge alert events in bulk
     * @request POST:/api/v1/alertEvents/acknowledge
     * @secure
     * @response `204` `void` No Content
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    acknowledgeAlertEvents: (
      data: AcknowledgeAlertEventsInput,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/alertEvents/acknowledge`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags AlertEvents
     * @name ResolveAlertEvents
     * @summary Resolve alert events in bulk
     * @request POST:/api/v1/alertEvents/resolve
     * @secure
     * @response `204` `void` No Content
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    resolveAlertEvents: (
      data: ResolveAlertEventsInput,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/alertEvents/resolve`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags AlertRules
     * @name GetAlertRule
     * @summary Get alert rule by id
     * @request GET:/api/v1/alertRules/{id}
     * @secure
     * @response `200` `AlertRuleView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getAlertRule: (id: string, params: RequestParams = {}) =>
      this.request<
        AlertRuleView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/alertRules/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AlertRules
     * @name UpdateAlertRule
     * @summary Update an alert rule
     * @request PATCH:/api/v1/alertRules/{id}
     * @secure
     * @response `200` `AlertRuleView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateAlertRule: (
      id: string,
      data: PatchAlertRuleInput,
      params: RequestParams = {},
    ) =>
      this.request<
        AlertRuleView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/alertRules/${id}`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AlertRules
     * @name GetAlertRuleConfig
     * @summary Get alert rule configuration
     * @request GET:/api/v1/alertRules/{id}/_cfg
     * @secure
     * @response `200` `AlertRuleConfigView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getAlertRuleConfig: (id: string, params: RequestParams = {}) =>
      this.request<
        AlertRuleConfigView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/alertRules/${id}/_cfg`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AlertRules
     * @name ListAlertRules
     * @summary List alert rules
     * @request GET:/api/v1/alertRules
     * @secure
     * @response `200` `AlertRulesView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listAlertRules: (params: RequestParams = {}) =>
      this.request<
        AlertRulesView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/alertRules`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AlertRules
     * @name CreateAlertRule
     * @summary Create an alert rule
     * @request POST:/api/v1/alertRules
     * @secure
     * @response `200` `AlertRuleView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createAlertRule: (data: CreateAlertRuleInput, params: RequestParams = {}) =>
      this.request<
        AlertRuleView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/alertRules`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AlertRules
     * @name DeleteAlertRules
     * @summary Delete alert rules
     * @request DELETE:/api/v1/alertRules
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteAlertRules: (
      data: DeleteAlertRulesInput,
      params: RequestParams = {},
    ) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/alertRules`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags AlertRules
     * @name RenameAlertRule
     * @summary Rename an alert rule
     * @request POST:/api/v1/alertRules/rename
     * @secure
     * @response `200` `AlertRuleView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    renameAlertRule: (data: RenameResource, params: RequestParams = {}) =>
      this.request<
        AlertRuleView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/alertRules/rename`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AlertRules
     * @name UpdateAlertRuleMetadata
     * @summary Update alert rule metadata
     * @request PATCH:/api/v1/alertRules/{id}/_metadata
     * @secure
     * @response `200` `AlertRuleView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateAlertRuleMetadata: (
      id: string,
      data: PatchResourceMetadata,
      params: RequestParams = {},
    ) =>
      this.request<
        AlertRuleView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/alertRules/${id}/_metadata`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AlertRules
     * @name GetAlertChannel
     * @summary Get alert channel by id
     * @request GET:/api/v1/alertRules/channels/{id}
     * @secure
     * @response `200` `AlertChannelView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    getAlertChannel: (id: string, params: RequestParams = {}) =>
      this.request<
        AlertChannelView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/alertRules/channels/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AlertRules
     * @name UpdateAlertChannel
     * @summary Update an alert channel
     * @request PATCH:/api/v1/alertRules/channels/{id}
     * @secure
     * @response `200` `AlertChannelView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    updateAlertChannel: (
      id: string,
      data: AlertChannelInput,
      params: RequestParams = {},
    ) =>
      this.request<
        AlertChannelView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/alertRules/channels/${id}`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AlertRules
     * @name ListAlertChannels
     * @summary List alert channels
     * @request GET:/api/v1/alertRules/channels
     * @secure
     * @response `200` `AlertChannelsView` OK
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    listAlertChannels: (params: RequestParams = {}) =>
      this.request<AlertChannelsView, ProblemDetails>({
        path: `/api/v1/alertRules/channels`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AlertRules
     * @name CreateAlertChannel
     * @summary Create an alert channel
     * @request POST:/api/v1/alertRules/channels
     * @secure
     * @response `200` `AlertChannelView` OK
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    createAlertChannel: (data: AlertChannelInput, params: RequestParams = {}) =>
      this.request<
        AlertChannelView,
        HttpValidationProblemDetails | ProblemDetails
      >({
        path: `/api/v1/alertRules/channels`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AlertRules
     * @name DeleteAlertChannels
     * @summary Delete alert channels
     * @request DELETE:/api/v1/alertRules/channels
     * @secure
     * @response `204` `void` No Content
     * @response `400` `HttpValidationProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    deleteAlertChannels: (
      data: DeleteAlertChannelsInput,
      params: RequestParams = {},
    ) =>
      this.request<void, HttpValidationProblemDetails | ProblemDetails>({
        path: `/api/v1/alertRules/channels`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags AlertRules
     * @name VerifyAlertChannel
     * @summary Verify an alert channel URL
     * @request POST:/api/v1/alertRules/channels/verify
     * @secure
     * @response `204` `void` No Content
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    verifyAlertChannel: (
      data: VerifyAlertChannelInput,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/alertRules/channels/verify`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Lookup
     * @name Lookup
     * @summary Lookup resources
     * @request GET:/api/v1/lookup
     * @secure
     * @response `200` `(ResourceInfo)[]` OK
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     */
    lookup: (
      query: {
        TargetResourceType: LookupResourceType;
        SourceResourceType?: LookupResourceType;
        /** @format uuid */
        SourceResourceId?: string;
        /** @format uuid */
        PlatformId?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<ResourceInfo[], ProblemDetails>({
        path: `/api/v1/lookup`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),
  };
  listener = {
    /**
     * No description
     *
     * @tags WebhookListener
     * @name ReceiveWebhook
     * @summary Receive a provider webhook delivery
     * @request POST:/listener/{authType}/{resourceType}/{id}/{execution}
     * @response `200` `void` OK
     * @response `429` `ProblemDetails` Too Many Requests
     */
    receiveWebhook: (
      authType: string,
      resourceType: string,
      id: string,
      execution: string,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
        path: `/listener/${authType}/${resourceType}/${id}/${execution}`,
        method: "POST",
        ...params,
      }),
  };
}
