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

export enum WebhookProvider {
  GitHub = "GitHub",
  GitLab = "GitLab",
  Generic = "Generic",
}

export enum WebhookAuthScheme {
  GitHubHmacSha256 = "GitHubHmacSha256",
  GitLabSignedToken = "GitLabSignedToken",
  GitLabLegacyToken = "GitLabLegacyToken",
  BearerToken = "BearerToken",
}

export enum VolumeEntryType {
  File = "File",
  Directory = "Directory",
  Symlink = "Symlink",
  Other = "Other",
}

export enum VolumeBackupConsistency {
  Live = "Live",
  StopAttachedContainers = "StopAttachedContainers",
}

export enum UserUiRadius {
  None = "None",
  Small = "Small",
  Medium = "Medium",
  Large = "Large",
}

export enum UserUiFont {
  Geist = "Geist",
  Inter = "Inter",
  IbmPlexSans = "IbmPlexSans",
  SourceSans3 = "SourceSans3",
  System = "System",
}

export enum UserUiDensity {
  Compact = "Compact",
  Comfortable = "Comfortable",
}

export enum UserThemeColor {
  Neutral = "Neutral",
  Blue = "Blue",
  Indigo = "Indigo",
  Violet = "Violet",
  Emerald = "Emerald",
  Yellow = "Yellow",
  Orange = "Orange",
  Rose = "Rose",
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

export enum UserContentLayout {
  Compact = "Compact",
  Wide = "Wide",
  Full = "Full",
}

export enum UpdateOrder {
  StopFirst = "StopFirst",
  StartFirst = "StartFirst",
}

export enum UpdateFailureAction {
  Continue = "Continue",
  Pause = "Pause",
  Rollback = "Rollback",
}

export enum UpdateBehavior {
  Disabled = "Disabled",
  Notify = "Notify",
  AutoDeploy = "AutoDeploy",
}

export enum SwarmStackCompatibilitySeverity {
  Warning = "Warning",
  Error = "Error",
}

export enum SwarmServiceSynchronizationState {
  NeverApplied = "NeverApplied",
  DesiredChangesPending = "DesiredChangesPending",
  InSync = "InSync",
  Drifted = "Drifted",
  RuntimeMissing = "RuntimeMissing",
  OutcomeUnknown = "OutcomeUnknown",
  OwnershipConflict = "OwnershipConflict",
}

export enum SwarmServiceOwnership {
  Unmanaged = "Unmanaged",
  DockerStackExternal = "DockerStackExternal",
  CitadelService = "CitadelService",
  CitadelStack = "CitadelStack",
  OwnershipConflict = "OwnershipConflict",
  System = "System",
}

export enum SwarmServiceOperationState {
  Prepared = "Prepared",
  Canceled = "Canceled",
  PendingAcceptance = "PendingAcceptance",
  Accepted = "Accepted",
  Rejected = "Rejected",
  NotAccepted = "NotAccepted",
  OutcomeUnknown = "OutcomeUnknown",
  Completed = "Completed",
  OwnershipConflict = "OwnershipConflict",
}

export enum SwarmServiceOperationKind {
  Apply = "Apply",
  Scale = "Scale",
  ForceUpdate = "ForceUpdate",
  Delete = "Delete",
}

export enum SwarmServiceHealth {
  Unknown = "Unknown",
  Healthy = "Healthy",
  Progressing = "Progressing",
  Degraded = "Degraded",
  Failed = "Failed",
  Created = "Created",
  Stopped = "Stopped",
}

export enum SwarmQuorumState {
  Unknown = "Unknown",
  Lost = "Lost",
  Degraded = "Degraded",
  Healthy = "Healthy",
}

export enum StopSignal {
  SIGTERM = "SIGTERM",
  SIGKILL = "SIGKILL",
  SIGINT = "SIGINT",
  SIGQUIT = "SIGQUIT",
}

/** @default 24 */
export enum StatsHours {
  Value24 = 24,
  Value48 = 48,
  Value72 = 72,
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
  TimedOut = "TimedOut",
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

export enum StackImportKind {
  ComposeProject = "ComposeProject",
  SwarmStack = "SwarmStack",
}

export enum StackDriftMode {
  Disabled = "Disabled",
  DetectOnly = "DetectOnly",
  AutoFix = "AutoFix",
}

export enum StackApplyEventType {
  Unknown = "Unknown",
  StdOut = "StdOut",
  StdErr = "StdErr",
  SystemMessage = "SystemMessage",
  CommandCompleted = "CommandCompleted",
}

export enum SpecificPermission {
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
  ManageNodeAgents = "ManageNodeAgents",
  Use = "Use",
  ManageCredentials = "ManageCredentials",
}

export enum SecretProviderType {
  InternalEncrypted = "InternalEncrypted",
  VaultCompatibleKvV2 = "VaultCompatibleKvV2",
}

export enum SecretDeliveryMode {
  EnvironmentVariable = "EnvironmentVariable",
  MountedFile = "MountedFile",
  NativePlatformSecret = "NativePlatformSecret",
}

export enum SearchStatusTone {
  Positive = "Positive",
  Negative = "Negative",
  Warning = "Warning",
  Info = "Info",
  Neutral = "Neutral",
}

export enum SchedulingMode {
  Replicated = "Replicated",
  Global = "Global",
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

export enum RestartCondition {
  None = "None",
  OnFailure = "OnFailure",
  Any = "Any",
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
  SwarmService = "SwarmService",
  ServiceAccount = "ServiceAccount",
}

export enum ResourceControlState {
  Idle = "Idle",
  Queued = "Queued",
  Processing = "Processing",
}

export enum ResourceBindingScope {
  Global = "Global",
  Stack = "Stack",
  Deployment = "Deployment",
  SwarmService = "SwarmService",
}

export enum ResourceBindingKind {
  Variable = "Variable",
  Secret = "Secret",
}

export enum RegistryTagStatus {
  Active = "Active",
  Inactive = "Inactive",
}

export enum RegistryStatus {
  Active = "Active",
  Disabled = "Disabled",
  Deprecated = "Deprecated",
}

export enum RegistryKind {
  Custom = "Custom",
  DockerHub = "DockerHub",
  Azure = "Azure",
  AWS = "AWS",
  Gitlab = "Gitlab",
  GitHub = "GitHub",
}

export enum QueueBuildTrigger {
  Manual = "Manual",
  Webhook = "Webhook",
  Dependency = "Dependency",
}

export enum PruneResource {
  All = "All",
  Volume = "Volume",
  Network = "Network",
  Image = "Image",
  Build = "Build",
}

export enum PortPublishMode {
  Ingress = "Ingress",
  Host = "Host",
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

export enum MountKind {
  Volume = "Volume",
  Bind = "Bind",
  Tmpfs = "Tmpfs",
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
  Build = "Build",
  BuildAgentPool = "BuildAgentPool",
  SwarmService = "SwarmService",
  RunAsActor = "RunAsActor",
  ServiceAccount = "ServiceAccount",
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

export enum LicenseCapability {
  CustomAccessControl = "CustomAccessControl",
  AutomatedOperations = "AutomatedOperations",
  AdvancedAlerting = "AdvancedAlerting",
  OperationalGuardrails = "OperationalGuardrails",
  ElasticBuildExecution = "ElasticBuildExecution",
}

export enum GlobalSearchResourceType {
  Platform = "Platform",
  Stack = "Stack",
  Deployment = "Deployment",
  GitRepository = "GitRepository",
  Registry = "Registry",
  AutomationAction = "AutomationAction",
  BackupPolicy = "BackupPolicy",
  BackupRepository = "BackupRepository",
  Build = "Build",
  BuildAgentPool = "BuildAgentPool",
  SwarmService = "SwarmService",
}

export enum GlobalSearchCategory {
  Platforms = "Platforms",
  Stacks = "Stacks",
  Deployments = "Deployments",
  Repositories = "Repositories",
  Registries = "Registries",
  Automations = "Automations",
  Backups = "Backups",
  Builds = "Builds",
  SwarmServices = "SwarmServices",
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

export enum GitRepositoryStatus {
  Unknown = "Unknown",
  Pending = "Pending",
  Created = "Created",
  Healthy = "Healthy",
  Degraded = "Degraded",
}

export enum GitRepositoryRefStatus {
  Pending = "Pending",
  Syncing = "Syncing",
  Healthy = "Healthy",
  Degraded = "Degraded",
}

export enum GitEntryTypeView {
  Directory = "Directory",
  File = "File",
  Symlink = "Symlink",
  Submodule = "Submodule",
}

export enum GitChangedPathStatus {
  Added = "Added",
  Modified = "Modified",
  Deleted = "Deleted",
  Renamed = "Renamed",
  Copied = "Copied",
  TypeChanged = "TypeChanged",
}

export enum GitAuthType {
  Basic = "Basic",
  Token = "Token",
  SshKey = "SshKey",
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

export enum CurrentProfileAuthenticationType {
  Local = "Local",
  Oidc = "Oidc",
}

export enum CpuArchitecture {
  Amd64 = "Amd64",
  Arm64 = "Arm64",
}

export enum ContainerSystemRole {
  Core = "Core",
  Database = "Database",
  Agent = "Agent",
  EdgeAgent = "EdgeAgent",
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
  Dependency = "Dependency",
}

export enum BuildRunStatus {
  Queued = "Queued",
  Preparing = "Preparing",
  Running = "Running",
  Succeeded = "Succeeded",
  Failed = "Failed",
  TimedOut = "TimedOut",
  Cancelled = "Cancelled",
  Rejected = "Rejected",
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

export enum BuildAgentPoolConnectionMode {
  InboundAgent = "InboundAgent",
  EdgeAgent = "EdgeAgent",
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
  Processing = "Processing",
}

export enum BackupRunItemStatus {
  Queued = "Queued",
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
  Processing = "Processing",
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
  Unavailable = "Unavailable",
  Unknown = "Unknown",
  Uninitialized = "Uninitialized",
  Ready = "Ready",
}

export enum BackupQueueTrigger {
  Manual = "Manual",
  Schedule = "Schedule",
  Webhook = "Webhook",
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

export enum AutomationRunTrigger {
  Manual = "Manual",
  Test = "Test",
  Schedule = "Schedule",
  Webhook = "Webhook",
}

export enum AutomationRunStatus {
  Queued = "Queued",
  Running = "Running",
  Succeeded = "Succeeded",
  Failed = "Failed",
  TimedOut = "TimedOut",
  Cancelled = "Cancelled",
  Rejected = "Rejected",
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
  PlatformDiskHigh = "PlatformDiskHigh",
  PlatformUnreachable = "PlatformUnreachable",
  PlatformVersionMismatch = "PlatformVersionMismatch",
  UnmanagedContainerCreated = "UnmanagedContainerCreated",
  DeploymentImageUpdateAvailable = "DeploymentImageUpdateAvailable",
  DeploymentAutoDeployFailed = "DeploymentAutoDeployFailed",
  DeploymentAutoUpdated = "DeploymentAutoUpdated",
  SwarmServiceOperationFailed = "SwarmServiceOperationFailed",
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
  BuildRunFailed = "BuildRunFailed",
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
  Build = "Build",
  License = "License",
  SwarmService = "SwarmService",
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

export enum AdoptionIssueSeverity {
  Warning = "Warning",
  Blocker = "Blocker",
}

export enum ActorType {
  User = "User",
  System = "System",
  Agent = "Agent",
  ServiceAccount = "ServiceAccount",
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
  Team = "Team",
  Role = "Role",
  License = "License",
  Build = "Build",
  BuildAgentPool = "BuildAgentPool",
  Volume = "Volume",
  BackupPolicy = "BackupPolicy",
  SwarmService = "SwarmService",
  ServiceAccount = "ServiceAccount",
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
  DeploymentAdopted = "DeploymentAdopted",
  PlatformCreated = "PlatformCreated",
  PlatformDeleted = "PlatformDeleted",
  PlatformConnected = "PlatformConnected",
  PlatformDisconnected = "PlatformDisconnected",
  PlatformRenamed = "PlatformRenamed",
  PlatformNodeAgentLifecycle = "PlatformNodeAgentLifecycle",
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
  StackImported = "StackImported",
  StackWebhookReceived = "StackWebhookReceived",
  InitialAdministratorCreated = "InitialAdministratorCreated",
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
  UserCreated = "UserCreated",
  UserUpdated = "UserUpdated",
  UserRenamed = "UserRenamed",
  UserDeleted = "UserDeleted",
  TeamCreated = "TeamCreated",
  TeamUpdated = "TeamUpdated",
  TeamRenamed = "TeamRenamed",
  TeamDeleted = "TeamDeleted",
  RoleCreated = "RoleCreated",
  RoleUpdated = "RoleUpdated",
  RoleRenamed = "RoleRenamed",
  RoleDeleted = "RoleDeleted",
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
  BackupPolicyCreated = "BackupPolicyCreated",
  BackupPolicyUpdated = "BackupPolicyUpdated",
  BackupPolicyRenamed = "BackupPolicyRenamed",
  BackupPolicyArchived = "BackupPolicyArchived",
  BackupRunQueued = "BackupRunQueued",
  BackupRunStarted = "BackupRunStarted",
  BackupRunCompleted = "BackupRunCompleted",
  SwarmServiceCreated = "SwarmServiceCreated",
  SwarmServiceAdopted = "SwarmServiceAdopted",
  SwarmServiceUpdated = "SwarmServiceUpdated",
  SwarmServiceRenamed = "SwarmServiceRenamed",
  SwarmServiceDeleted = "SwarmServiceDeleted",
  SwarmServiceApplied = "SwarmServiceApplied",
  SwarmServiceScaled = "SwarmServiceScaled",
  SwarmServiceForceUpdated = "SwarmServiceForceUpdated",
  SwarmServiceOperationFailed = "SwarmServiceOperationFailed",
  SwarmServiceDuplicated = "SwarmServiceDuplicated",
  SwarmServiceWebhookReceived = "SwarmServiceWebhookReceived",
  ServiceAccountCreated = "ServiceAccountCreated",
  ServiceAccountUpdated = "ServiceAccountUpdated",
  ServiceAccountRenamed = "ServiceAccountRenamed",
  ServiceAccountEnabled = "ServiceAccountEnabled",
  ServiceAccountDisabled = "ServiceAccountDisabled",
  ServiceAccountArchived = "ServiceAccountArchived",
  ServiceAccountTokenCreated = "ServiceAccountTokenCreated",
  ServiceAccountTokenRevoked = "ServiceAccountTokenRevoked",
}

export interface AccessTokenResponse {
  accessToken: string;
}

export interface ActionList {
  actions: AuthorizedAction[];
  capabilities: ResourceCapabilitiesView;
}

export interface ActivitiesView {
  pagedResult: PagedActivityView;
}

export interface ActivityChangedField {
  name: string;
  newValue?: string | null;
  oldValue?: string | null;
}

export interface ActivitySourceResource {
  /** @format uuid */
  resourceId: string;
  resourceName: string;
  resourceType: ActivityResourceType;
}

export interface ActivityView {
  /** @format uuid */
  actorId: string;
  actorName: string;
  actorType: ActorType;
  /** @format date-time */
  createdAt: string;
  eventType: ActivityEventType;
  /** @format uuid */
  id: string;
  info: PublicActivityEventInfo;
  /** @format uuid */
  platformId: string | null;
  platformName: string;
  platformStatus: PlatformStatus;
  /** @format uuid */
  resourceId: string | null;
  resourceName: string;
  resourceType: ActivityResourceType;
  status: ActivityStatus;
}

/** @format uuid */
export type ActorId = string;

export interface ActorView {
  /** @format uuid */
  id: string;
  isEnabled: boolean;
  name: string;
  type: ActorType;
}

export interface AddServiceAccountResourceAccessRequest {
  permissionLevel: PermissionLevel;
  /** @format uuid */
  resourceId: string;
  resourceType: ResourceType;
  specificPermissions?: SpecificPermission[];
}

export interface AddServiceAccountRoleRequest {
  /** @format uuid */
  roleId: string;
}

export interface AddTeamMemberRequest {
  /** @format uuid */
  memberActorId: string;
}

export interface AddTeamRoleRequest {
  /** @format uuid */
  roleId: string;
}

export interface AddUserRoleRequest {
  /** @format uuid */
  roleId: string;
}

export interface AdoptContainerInput {
  description?: string | null;
  importSensitiveEnvironmentAsSecrets?: boolean;
  name: string;
  previewFingerprint: string;
  spec: DeploymentSpec;
  tagIds?: string[];
}

export interface AdoptSwarmServiceInput {
  description?: string | null;
  name: string;
  previewFingerprint: string;
  spec: SwarmServiceSpec;
  tagIds?: string[] | null;
}

export interface AdoptionDeploymentDraft {
  description: string | null;
  name: string;
  /** @format uuid */
  platformId: string;
  spec: DeploymentSpec;
  tagIds: string[];
}

export interface AdoptionIssue {
  code: string;
  fieldPath: string | null;
  message: string;
  severity: AdoptionIssueSeverity;
}

export interface AdoptionSource {
  dockerContainerId: string;
  /** @format uuid */
  id: string;
  name: string;
  /** @format uuid */
  platformId: string;
  platformName: string;
  state: string;
}

/** Public installation instructions. The signing private key never enters this model. */
export interface AgentSetupView {
  agentImage: string;
  dockerRunCommand: string;
  environment: Record<string, string>;
  hubPublicKey: string;
  requiresTls: boolean;
}

export interface AlertChannelInput {
  alertDestination: AlertDestination;
  isActive: boolean;
  name?: string | null;
  url: string;
}

export interface AlertChannelView {
  alertDestination: AlertDestination;
  capabilities: null | ResourceCapabilitiesView;
  /** @format date-time */
  createdAt: string;
  /** @format uuid */
  createdByActorId: string;
  /** @format uuid */
  id: string;
  isActive: boolean;
  name: string;
  url: string;
}

export interface AlertEventPage {
  items: AlertEventView[];
  /** @format int32 */
  page: number;
  /** @format int32 */
  pageSize: number;
  /** @format int64 */
  totalCount: number;
}

export interface AlertEventView {
  /** @format date-time */
  acknowledgedAt: string | null;
  /** @format uuid */
  acknowledgedByActorId: string | null;
  /** @format uuid */
  actorId: string | null;
  actorName: string | null;
  actorType: null | ActorType;
  /** @format uuid */
  alertRuleId: string;
  /** @format date-time */
  createdAt: string;
  /** @format uuid */
  id: string;
  /** Event-specific details supplied by the alert emitter. */
  info: any;
  message: string;
  resolutionNote: string | null;
  /** @format date-time */
  resolvedAt: string | null;
  /** @format uuid */
  resolvedByActorId: string | null;
  /** @format uuid */
  resourceId: string | null;
  resourceName: string;
  resourcePath: string | null;
  resourceType: AlertResourceType;
  severity: AlertSeverity;
  status: AlertEventStatus;
  type: AlertType;
  /** @format date-time */
  updatedAt: string;
}

export type AlertQuietHour =
  | {
      $type: "Daily";
      description?: string | null;
      endTime: string;
      name: string;
      startTime: string;
      timezone: string;
    }
  | {
      $type: "Weekly";
      dayOfWeek: DayOfWeek;
      description?: string | null;
      endTime: string;
      name: string;
      startTime: string;
      timezone: string;
    };

export interface AlertResourceScope {
  /** @format uuid */
  resourceId: string;
  resourceType: AlertResourceType;
}

export interface AlertRuleActivitySnapshot {
  channelIds: string[];
  /** @format int32 */
  cooldownSeconds?: number | null;
  description?: string | null;
  /** @format uuid */
  id: string;
  limitedTo: any[];
  name: string;
  quietHours: any[];
  /** @format int32 */
  requiredMatches?: number | null;
  severity: string;
  status: string;
  /** @format double */
  threshold?: number | null;
  type: string;
}

export type AlertRuleConfig = AlertRuleInput & {
  /** @format uuid */
  id: string;
  isSystem: boolean;
};

export interface AlertRuleInput {
  channelIds?: string[];
  /** @format int32 */
  cooldownSeconds?: number | null;
  description?: string | null;
  limitedTo?: AlertResourceScope[];
  name?: string | null;
  quietHours?: AlertQuietHour[];
  /** @format int32 */
  requiredMatches?: number | null;
  severity: AlertSeverity;
  status?: AlertRuleStatus;
  /** @format double */
  threshold?: number | null;
  type: AlertType;
}

export type AlertRuleListItem = AlertRuleView & {
  channels: AlertChannelView[];
};

export interface AlertRuleView {
  capabilities: null | ResourceCapabilitiesView;
  channelIds: string[];
  /** @format int32 */
  cooldownSeconds: number | null;
  /** @format date-time */
  createdAt: string;
  /** @format uuid */
  createdByActorId: string;
  description: string | null;
  /** @format uuid */
  id: string;
  limitedTo: AlertResourceScope[];
  name: string;
  quietHours: AlertQuietHour[];
  /** @format int32 */
  requiredMatches: number | null;
  severity: AlertSeverity;
  status: AlertRuleStatus;
  /** @format double */
  threshold: number | null;
  type: AlertType;
}

export interface ApplicationInfoView {
  informationalVersion: string;
  name: string;
  realtimeTransport: string;
  version: string;
}

export interface ApplyDeploymentInput {
  /** @format uuid */
  id: string;
  recreate?: boolean | null;
}

export interface ApplyStackInput {
  /** @format uuid */
  id: string;
  recreate?: boolean | null;
}

export interface ArchiveServiceAccountsRequest {
  ids: string[];
}

export type AuthorizedAction = AutomationActionView & {
  capabilities: ResourceCapabilitiesView;
};

export type AuthorizedGitAccountView = GitAccountView & {
  capabilities: ResourceCapabilitiesView;
};

export type AuthorizedGitRepositoryView = GitRepositoryView & {
  capabilities: ResourceCapabilitiesView;
};

export type AuthorizedPool = BuildAgentPoolView & {
  capabilities: ResourceCapabilitiesView;
};

export type AuthorizedProject = BuildProjectView & {
  capabilities: ResourceCapabilitiesView;
};

export type AuthorizedRegistryView = RegistryView & {
  capabilities: ResourceCapabilitiesView;
  isDefault: boolean;
};

export type AuthorizedTagView = TagView & {
  capabilities: ResourceCapabilitiesView;
};

export interface AutoUpdateState {
  currentDigest?: string | null;
  /** @format date-time */
  lastCheckedAt: string;
  lastError?: string | null;
  remoteDigest?: string | null;
  status: AutoUpdateStatus;
}

export interface AutomationActionActivitySnapshot {
  alertOnFailure: boolean;
  code: string;
  defaultArgsJson: string;
  description?: string | null;
  enabled: boolean;
  /** @format uuid */
  id: string;
  name: string;
  /** @format uuid */
  runAsActorId: string;
  scheduleCron?: string | null;
  scheduleEnabled: boolean;
  scheduleTimeZone: string;
  /** @format int32 */
  timeoutSeconds: number;
  webhook?: null | WebhookConfig;
}

export interface AutomationActionInput {
  alertOnFailure: boolean;
  code: string;
  defaultArgsJson?: string | null;
  description?: string | null;
  enabled: boolean;
  name: string;
  /** @format uuid */
  runAsActorId?: string | null;
  scheduleCron?: string | null;
  scheduleEnabled: boolean;
  scheduleTimeZone?: string | null;
  tagIds?: string[];
  /** @format int32 */
  timeoutSeconds?: number | null;
  webhook?: null | WebhookConfig;
}

export interface AutomationActionRunLogsView {
  logs: string;
  /** @format uuid */
  runId: string;
}

export interface AutomationActionView {
  alertOnFailure: boolean;
  code: string;
  controlState: ResourceControlState;
  /** @format date-time */
  createdAt: string;
  /** @format uuid */
  createdByActorId: string;
  /** @format uuid */
  currentRunId: string | null;
  defaultArgsJson: string;
  description: string | null;
  enabled: boolean;
  /** @format uuid */
  id: string;
  /** @format date-time */
  lastScheduledRunAt: string | null;
  latestRun: null | AutomationRunView;
  name: string;
  /** @format int64 */
  rowVersion: number;
  /** @format uuid */
  runAsActorId: string;
  scheduleCron: string | null;
  scheduleEnabled: boolean;
  scheduleTimeZone: string;
  tags: ResourcesTagsTagSummary[];
  /** @format int32 */
  timeoutSeconds: number;
  /** @format date-time */
  updatedAt: string;
  webhook: null | WebhookConfig;
}

export interface AutomationProgress {
  error: null | AutomationProgressError;
  errorMessage: string | null;
  progressMessage: string | null;
  /** @format uuid */
  runId: string | null;
  status: null | AutomationRunStatus;
  stream: string | null;
}

export interface AutomationProgressError {
  /** @format int32 */
  code: number;
  message: string;
}

export interface AutomationRunView {
  /** @format uuid */
  actionId: string;
  actionName: string;
  argsJson: string;
  codeHash: string;
  codeSnapshot: string | null;
  /** @format int64 */
  durationMs: number | null;
  errorMessage: string | null;
  /** @format int32 */
  exitCode: number | null;
  /** @format date-time */
  finishedAt: string | null;
  /** @format uuid */
  id: string;
  logs: string | null;
  /** @format date-time */
  queuedAt: string;
  /** @format uuid */
  runAsActorId: string;
  /** @format date-time */
  startedAt: string | null;
  status: AutomationRunStatus;
  /** @format int32 */
  timeoutSeconds: number;
  trigger: AutomationRunTrigger;
  /** @format uuid */
  triggeredByActorId: string | null;
}

export interface AwsEc2BuildAgentPoolProviderSpec {
  /** @default "" */
  amiId?: string;
  /** @default "Amd64" */
  architecture?: CpuArchitecture;
  /** @default false */
  assignPublicIp?: boolean;
  /** @default null */
  assumeRoleArn?: string | null;
  /**
   * @format uuid
   * @default null
   */
  awsCredentialSecretId?: string | null;
  /** @default null */
  instanceProfileName?: string | null;
  /** @default "" */
  instanceType?: string;
  /** @default null */
  keyPairName?: string | null;
  /** @default "" */
  region?: string;
  /**
   * @format int32
   * @default 0
   */
  rootVolumeSizeGb?: number;
  /** @default [] */
  securityGroupIds?: string[];
  /** @default "" */
  subnetId?: string;
  /** @default null */
  tags?: Record<string, string> | null;
}

export interface BackupCoverageView {
  /** @format date-time */
  lastRunAt?: string | null;
  /** @format uuid */
  lastRunId?: string | null;
  lastRunStatus?: null | BackupRunStatus;
  /** @format date-time */
  lastSuccessfulRunAt?: string | null;
  /** @format date-time */
  nextRunAt?: string | null;
  /** @format int32 */
  policyCount: number;
  status: BackupCoverageStatus;
}

export interface BackupMetadataPatch {
  description?: string | null;
}

export interface BackupPolicyActivitySnapshot {
  alertOnFailure: boolean;
  /** @format uuid */
  backupRepositoryId: string;
  cron?: string | null;
  description?: string | null;
  enabled: boolean;
  /** @format uuid */
  id: string;
  /** @format int32 */
  keepLastSuccessful: number;
  name: string;
  /** @format uuid */
  runAsActorId: string;
  sourceKey: string;
  sourceType: string;
  timeZone?: string | null;
  /** @format int32 */
  timeoutSeconds: number;
  webhookEnabled: boolean;
}

export interface BackupPolicyInput {
  alertOnFailure: boolean;
  /** @format uuid */
  backupRepositoryId: string;
  cron?: string | null;
  description?: string | null;
  enabled: boolean;
  /** @format int32 */
  keepLastSuccessful?: number | null;
  name: string;
  /** @format uuid */
  runAsActorId?: string | null;
  source: BackupSourceSpec;
  tagIds?: string[];
  timeZone?: string | null;
  /** @format int32 */
  timeoutSeconds?: number | null;
  webhook?: null | WebhookConfig;
}

export interface BackupPolicyView {
  alertOnFailure: boolean;
  /** @format date-time */
  archivedAt: string | null;
  /** @format uuid */
  backupRepositoryId: string;
  capabilities: null | ResourceCapabilitiesView;
  controlState: ResourceControlState;
  /** @format date-time */
  createdAt: string;
  /** @format uuid */
  createdByActorId: string;
  cron: string | null;
  /** @format uuid */
  currentRunId: string | null;
  description: string | null;
  enabled: boolean;
  /** @format date-time */
  firstSuccessfulRunAt: string | null;
  /** @format uuid */
  id: string;
  /** @format int32 */
  keepLastSuccessful: number;
  /** @format date-time */
  lastScheduledRunAt: string | null;
  latestRun: null | BackupRunView;
  name: string;
  normalizedName: string;
  /** @format int64 */
  rowVersion: number;
  /** @format uuid */
  runAsActorId: string;
  source: BackupSourceSpec;
  tags: ResourcesTagsTagSummary[];
  timeZone: string | null;
  /** @format int32 */
  timeoutSeconds: number;
  /** @format date-time */
  updatedAt: string;
  webhook: null | WebhookConfig;
}

export type BackupPreviewResource =
  | {
      /** @format uuid */
      deploymentId: string;
      deploymentName: string;
    }
  | {
      /** @format uuid */
      stackId: string;
      stackName: string;
    }
  | {
      /** @format uuid */
      swarmServiceId: string;
      swarmServiceName: string;
    };

export interface BackupRepositoryInput {
  description?: string | null;
  name: string;
  /** @format uuid */
  passwordSecretId: string;
  spec: BackupRepositorySpec;
}

export type BackupRepositorySpec =
  | {
      $type: "FileSystem";
      location: BackupExecutionLocation;
      path: string;
      /** @format uuid */
      platformId?: string | null;
    }
  | {
      $type: "S3Compatible";
      /** @format uuid */
      accessKeySecretId: string;
      allowInsecureHttp?: boolean;
      bucket: string;
      bucketLookup?: S3BucketLookup;
      endpoint: string;
      prefix?: string | null;
      region?: string | null;
      /** @format uuid */
      secretKeySecretId: string;
      /** @format uuid */
      sessionTokenSecretId?: string | null;
    };

export interface BackupRepositoryValidationView {
  /** @format uuid */
  backupRepositoryId: string;
  /** @format uuid */
  id: string;
  lastErrorCode: string | null;
  lastErrorMessage: string | null;
  /** @format date-time */
  lastValidatedAt: string;
  location: BackupExecutionLocation;
  /** @format uuid */
  platformId: string | null;
  status: BackupRepositoryValidationStatus;
}

export interface BackupRepositoryView {
  /** @format date-time */
  archivedAt: string | null;
  capabilities: null | ResourceCapabilitiesView;
  /** @format int64 */
  controlStartedAt: number | null;
  controlState: ResourceControlState;
  /** @format date-time */
  createdAt: string;
  /** @format uuid */
  createdByActorId: string;
  /** @format uuid */
  currentRunId: string | null;
  description: string | null;
  /** @format uuid */
  id: string;
  /** @format date-time */
  lastCheckedAt: string | null;
  /** @format date-time */
  lastPrunedAt: string | null;
  name: string;
  normalizedName: string;
  /** @format uuid */
  passwordSecretId: string;
  /** @format int64 */
  rowVersion: number;
  spec: BackupRepositorySpec;
  status: BackupRepositoryStatus;
  type: BackupRepositoryType;
  /** @format date-time */
  updatedAt: string;
}

export interface BackupRestoreRunStreamItem {
  /** @format int32 */
  exitCode?: number | null;
  message?: string | null;
  /** @format uuid */
  restoreRunId: string;
  status?: null | BackupRestoreStatus;
  stream?: string | null;
}

export interface BackupRestoreRunView {
  /** @format uuid */
  backupRepositoryId: string;
  /** @format uuid */
  backupRunId: string;
  /** @format date-time */
  completedAt: string | null;
  errorCode: string | null;
  errorMessage: string | null;
  /** @format int32 */
  exitCode: number | null;
  /** @format uuid */
  id: string;
  overwriteExisting: boolean;
  /** @format date-time */
  queuedAt: string;
  /** @format uuid */
  sourceBackupRunItemId: string | null;
  /** @format date-time */
  startedAt: string | null;
  status: BackupRestoreStatus;
  targetDockerNodeId: string | null;
  /** @format uuid */
  targetPlatformId: string;
  targetVolumeName: string;
  /** @format uuid */
  triggeredByActorId: string;
}

export interface BackupRunItemView {
  /** @format uuid */
  backupRunId: string;
  /** @format int64 */
  bytesAdded: number | null;
  /** @format int64 */
  bytesProcessed: number | null;
  /** @format date-time */
  completedAt: string | null;
  dockerNodeId: string | null;
  errorCode: string | null;
  errorMessage: string | null;
  /** @format int32 */
  exitCode: number | null;
  /** @format int64 */
  filesProcessed: number | null;
  /** @format uuid */
  id: string;
  nodeHostname: string | null;
  parentSnapshotId: string | null;
  /** @format uuid */
  platformId: string;
  resticSnapshotId: string | null;
  /** @format date-time */
  startedAt: string | null;
  status: BackupRunItemStatus;
  volumeName: string;
}

export interface BackupRunStreamItem {
  /** @format int32 */
  exitCode?: number | null;
  message?: string | null;
  /** @format uuid */
  runId: string;
  status?: null | BackupRunStatus;
  stream?: string | null;
}

export interface BackupRunView {
  /** @format uuid */
  backupPolicyId: string;
  /** @format uuid */
  backupRepositoryId: string;
  /** @format int64 */
  bytesAdded: number | null;
  /** @format int64 */
  bytesProcessed: number | null;
  /** @format date-time */
  completedAt: string | null;
  errorCode: string | null;
  errorMessage: string | null;
  /** @format int32 */
  exitCode: number | null;
  /** @format int64 */
  filesProcessed: number | null;
  /** @format uuid */
  id: string;
  items: BackupRunItemView[];
  parentSnapshotId: string | null;
  policyNameSnapshot: string;
  /** @format date-time */
  queuedAt: string;
  repositoryTypeSnapshot: BackupRepositoryType;
  resticSnapshotId: string | null;
  snapshotAvailability: BackupSnapshotAvailability;
  sourceSnapshot: BackupSourceSpec;
  /** @format date-time */
  startedAt: string | null;
  status: BackupRunStatus;
  trigger: BackupRunTrigger;
  /** @format uuid */
  triggeredByActorId: string;
  warnings: string[];
}

export type BackupSourcePreview = BackupPreviewResource & {
  /** @format uuid */
  platformId: string;
  platformName: string;
  platformStatus: PlatformStatus;
  volumes: BackupVolumePreview[];
  warnings: string[];
};

export type BackupSourceSpec =
  | {
      $type: "DockerVolume";
      consistency?: VolumeBackupConsistency;
      dockerNodeId?: string | null;
      /** @format uuid */
      platformId: string;
      volumeName: string;
    }
  | {
      $type: "CitadelSystem";
    }
  | {
      $type: "Stack";
      /** @format uuid */
      stackId: string;
    }
  | {
      $type: "Deployment";
      /** @format uuid */
      deploymentId: string;
    }
  | {
      $type: "SwarmService";
      /** @format uuid */
      swarmServiceId: string;
    };

export interface BackupVolumePreview {
  dockerNodeId?: string | null;
  hasBackupCoverage: boolean;
  isExternal: boolean;
  isShared: boolean;
  kind: string;
  name: string;
  nodeHostname?: string | null;
}

export interface BuildAgentPoolActivitySnapshot {
  architecture: string;
  /** @format int32 */
  cleanupTimeoutSeconds: number;
  description?: string | null;
  enabled: boolean;
  /** @format int32 */
  failureRetentionMinutes: number;
  /** @format int32 */
  heartbeatTimeoutSeconds: number;
  /** @format uuid */
  id: string;
  instanceType: string;
  /** @format date-time */
  lastValidatedAt?: string | null;
  lastValidationMessage?: string | null;
  lastValidationStatus: string;
  /** @format int32 */
  maxActiveBuilders: number;
  /** @format int32 */
  maximumInstanceLifetimeSeconds: number;
  name: string;
  provider: string;
  providerSpec: any;
  /** @format int32 */
  provisioningTimeoutSeconds: number;
  /** @format int32 */
  queueTimeoutSeconds: number;
  region: string;
  /** @format int32 */
  registrationTimeoutSeconds: number;
}

export interface BuildAgentPoolInput {
  /** @format int32 */
  cleanupTimeoutSeconds?: number | null;
  description?: string | null;
  enabled: boolean;
  /** @format int32 */
  failureRetentionMinutes?: number | null;
  /** @format int32 */
  heartbeatTimeoutSeconds?: number | null;
  /** @format int32 */
  maxActiveBuilders?: number | null;
  /** @format int32 */
  maximumInstanceLifetimeSeconds?: number | null;
  name: string;
  providerSpec: BuildAgentPoolProviderSpec;
  /** @format int32 */
  provisioningTimeoutSeconds?: number | null;
  /** @format int32 */
  queueTimeoutSeconds?: number | null;
  /** @format int32 */
  registrationTimeoutSeconds?: number | null;
  tagIds?: string[];
}

export type BuildAgentPoolProviderSpec =
  | (AwsEc2BuildAgentPoolProviderSpec & {
      $type: "AwsEc2";
    })
  | (SelfManagedVmBuildAgentPoolProviderSpec & {
      $type: "SelfManagedVm";
    });

export interface BuildAgentPoolView {
  /** @format date-time */
  archivedAt: string | null;
  /** @format int32 */
  cleanupTimeoutSeconds: number;
  /** @format int64 */
  controlStartedAt: number | null;
  controlState: ResourceControlState;
  /** @format uuid */
  controlTriggeredBy: string | null;
  /** @format date-time */
  createdAt: string;
  /** @format uuid */
  createdByActorId: string;
  description: string | null;
  enabled: boolean;
  /** @format int32 */
  failureRetentionMinutes: number;
  /** @format int32 */
  heartbeatTimeoutSeconds: number;
  /** @format uuid */
  id: string;
  /** @format date-time */
  lastValidatedAt: string | null;
  lastValidationMessage: string | null;
  lastValidationStatus: BuildAgentPoolValidationStatus;
  /** @format int32 */
  maxActiveBuilders: number;
  /** @format int32 */
  maximumInstanceLifetimeSeconds: number;
  name: string;
  normalizedName: string;
  provider: BuildAgentPoolProvider;
  providerSpec: BuildAgentPoolProviderSpec;
  /** @format int32 */
  provisioningTimeoutSeconds: number;
  /** @format int32 */
  queueTimeoutSeconds: number;
  /** @format int32 */
  registrationTimeoutSeconds: number;
  /** @format int64 */
  rowVersion: number;
  tags: ResourcesTagsTagSummary[];
  /** @format date-time */
  updatedAt: string;
}

export interface BuildArgSpec {
  name: string;
  value?: string | null;
}

export interface BuildLogEntry {
  /** @format uuid */
  buildRunId: string;
  /** @format date-time */
  createdAt: string;
  /** @format uuid */
  id: string;
  message: string;
  stream: string;
}

export interface BuildMetadataPatch {
  description?: string | null;
}

export interface BuildPlatformSnapshot {
  address: string | null;
  /** @format uuid */
  buildAgentPoolId: string | null;
  builderKind: null | BuildProjectBuilderKind;
  /** @format uuid */
  id: string | null;
  name: string | null;
}

export interface BuildProjectActivitySnapshot {
  branch: string;
  /** @format uuid */
  buildAgentPoolId?: string | null;
  buildSecrets: BuildSecretActivitySnapshot[];
  builderKind: string;
  contextPath: string;
  description?: string | null;
  dockerfilePath: string;
  enabled: boolean;
  /** @format uuid */
  gitRepositoryId: string;
  /** @format uuid */
  id: string;
  imageRepository: string;
  name: string;
  /** @format uuid */
  platformId?: string | null;
  /** @format uuid */
  registryId: string;
  /** @format int32 */
  retentionRunCount: number;
  tagTemplates: string[];
  target?: string | null;
  /** @format int32 */
  timeoutSeconds: number;
  webhook?: null | WebhookConfig;
}

export interface BuildProjectInput {
  branch?: string | null;
  /** @format uuid */
  buildAgentPoolId?: string | null;
  buildArgs?: BuildArgSpec[] | null;
  buildSecrets?: BuildSecretSpec[] | null;
  builderKind?: BuildProjectBuilderKind;
  contextPath?: string | null;
  description?: string | null;
  dockerfilePath?: string | null;
  enabled: boolean;
  /** @format uuid */
  gitRepositoryId: string;
  imageRepository: string;
  name: string;
  /** @format uuid */
  platformId?: string | null;
  /** @format uuid */
  registryId: string;
  /** @format int32 */
  retentionRunCount?: number | null;
  tagIds?: string[];
  tagTemplates?: string[] | null;
  target?: string | null;
  /** @format int32 */
  timeoutSeconds?: number | null;
  webhook?: null | WebhookConfig;
}

export interface BuildProjectView {
  /** @format date-time */
  archivedAt: string | null;
  branch: string;
  /** @format uuid */
  buildAgentPoolId: string | null;
  buildArgs: BuildArgSpec[];
  buildSecrets: BuildSecretSpec[];
  builderKind: BuildProjectBuilderKind;
  contextPath: string;
  /** @format int64 */
  controlStartedAt: number | null;
  controlState: ResourceControlState;
  /** @format date-time */
  createdAt: string;
  /** @format uuid */
  createdByActorId: string;
  /** @format uuid */
  currentRunId: string | null;
  description: string | null;
  dockerfilePath: string;
  enabled: boolean;
  /** @format uuid */
  gitRepositoryId: string;
  /** @format uuid */
  id: string;
  imageRepository: string;
  latestRun: null | BuildRunView;
  name: string;
  normalizedName: string;
  /** @format uuid */
  platformId: string | null;
  /** @format uuid */
  registryId: string;
  /** @format int32 */
  retentionRunCount: number;
  /** @format int64 */
  rowVersion: number;
  tagTemplates: string[];
  tags: ResourcesTagsTagSummary[];
  target: string | null;
  /** @format int32 */
  timeoutSeconds: number;
  /** @format date-time */
  updatedAt: string;
  webhook: null | WebhookConfig;
}

export interface BuildRunView {
  branch: string;
  /** @format uuid */
  buildProjectId: string;
  /** @format date-time */
  completedAt: string | null;
  contextPath: string;
  dockerfilePath: string;
  errorCode: string | null;
  errorMessage: string | null;
  /** @format int32 */
  exitCode: number | null;
  /** @format uuid */
  gitRepositoryId: string;
  gitRepositoryNameSnapshot: string;
  /** @format uuid */
  id: string;
  imageDigest: string | null;
  imageReferences: string[];
  imageRepository: string;
  platformSnapshot: BuildPlatformSnapshot;
  projectNameSnapshot: string;
  /** @format date-time */
  queuedAt: string;
  registryHost: string;
  /** @format uuid */
  registryId: string;
  resolvedCommitSha: string | null;
  /** @format date-time */
  startedAt: string | null;
  status: BuildRunStatus;
  target: string | null;
  /** @format int32 */
  timeoutSeconds: number;
  trigger: BuildRunTrigger;
  /** @format uuid */
  triggeredByActorId: string;
}

export interface BuildSecretActivitySnapshot {
  id: string;
  /** @format uuid */
  secretId: string;
}

export interface BuildSecretSpec {
  id: string;
  /** @format uuid */
  secretId: string;
}

export interface ChangeCurrentPasswordRequest {
  currentPassword: string;
  newPassword: string;
}

export interface Channels {
  capabilities: ResourceCapabilitiesView;
  channels: AlertChannelView[];
}

export interface ComposeProjectImportDraftView {
  draft: ComposeProjectStackDraftView;
  importKind: StackImportKind;
  issues: StackAdoptionIssue[];
  runtimeFingerprint: string;
  source: ComposeProjectImportSourceView;
}

export interface ComposeProjectImportSourceView {
  containerIds: string[];
  containerNames: string[];
  /** @format uuid */
  platformId: string;
  platformName: string;
  projectName: string;
  services: ComposeProjectRuntimeService[];
}

export interface ComposeProjectImportValidation {
  canImportSensitiveEnvironmentValues: boolean;
  importableSensitiveEnvironmentNames: string[];
  issues: StackAdoptionIssue[];
  previewFingerprint: string;
  services: ComposeProjectServiceComparison[];
}

export interface ComposeProjectRuntimeService {
  /** @min 0 */
  containerCount: number;
  image: string | null;
  name: string;
  states: string[];
}

export interface ComposeProjectServiceComparison {
  definedInSource: boolean;
  name: string;
  /** @min 0 */
  runtimeContainerCount: number;
  runtimeImage: string | null;
  sourceImage: string | null;
}

export interface ComposeProjectStackDraftView {
  description: string | null;
  driftPolicy: StackDriftPolicy;
  name: string;
  /** @format uuid */
  platformId: string;
  tagIds: string[];
}

export interface ConfigContentView {
  content: string;
}

export interface ConfirmMandatoryMfaSetupInput {
  code: string;
}

export interface ConfirmProfileMfaSetupInput {
  code: string;
}

export interface ContainerAdoptionDraft {
  canImportSensitiveEnvironmentValues: boolean;
  draft: AdoptionDeploymentDraft;
  issues: AdoptionIssue[];
  previewFingerprint: string;
  source: AdoptionSource;
}

export interface ContainerDeploymentUpdateState {
  /** @format date-time */
  lastCheckedAt: string;
  status: AutoUpdateStatus;
}

/**
 * The container contract embeds a summary, never Deployment configuration or
 * resource bindings. These are read-model fields, not a cross-feature entity.
 */
export interface ContainerDeploymentView {
  autoUpdateState: ContainerDeploymentUpdateState;
  controlState: ResourceControlState;
  /** @format date-time */
  createdAt: string;
  /** @format uuid */
  createdByActorId: string;
  /** @format uuid */
  id: string;
  name: string;
  /** @format uuid */
  platformId: string;
  platformStatus: PlatformStatus;
  status: DeploymentStatus;
}

export interface ContainerHistory {
  containerId: string;
  containerName: string;
  stats: ContainerStatView[];
}

export interface ContainerInspectionView {
  appArmorProfile?: string | null;
  args?: string[];
  config?: Record<string, any> | null;
  created: string;
  driver?: string | null;
  execIDs?: string[];
  graphDriver?: Record<string, any> | null;
  hostConfig?: Record<string, any> | null;
  hostnamePath?: string | null;
  hostsPath?: string | null;
  id: string;
  image?: string | null;
  logPath?: string | null;
  mountLabel?: string | null;
  mounts?: Record<string, any>[];
  name?: string | null;
  networkSettings?: Record<string, any> | null;
  path?: string | null;
  platform?: string | null;
  processLabel?: string | null;
  resolvConfPath?: string | null;
  /** @format int64 */
  restartCount?: number | null;
  /** @format int64 */
  sizeRootFs?: number | null;
  /** @format int64 */
  sizeRw?: number | null;
  state?: Record<string, any> | null;
  [key: string]: any;
}

export interface ContainerRuntimeListView {
  containers: ContainerRuntimeView[];
}

export interface ContainerRuntimeView {
  capabilities: null | PlatformCapabilitiesView;
  containerStat: null | ContainerStatView;
  controlState: ResourceControlState;
  /** @format int64 */
  created: number;
  /** @format uuid */
  deploymentId: string | null;
  dockerNodeId: string | null;
  hasCitadelOwnershipLabels: boolean;
  /** Docker container ID; the statistics containerId is Citadel's database UUID. */
  id: string;
  image: string;
  imageId: string;
  isSwarmTask: boolean;
  isSystem: boolean;
  name: string;
  /** @format uuid */
  platformId: string;
  ports: Record<string, HostPortBinding[] | null> | null;
  stack: string | null;
  /** @format uuid */
  stackId: string | null;
  state: ContainerStateStatus;
  systemRole: null | ContainerSystemRole;
}

export interface ContainerStatSnapshot {
  /** @format uuid */
  containerId: string;
  /** @format double */
  cpuUsage: number;
  /** @format int64 */
  created: number;
  /** @format double */
  memoryActive: number;
  /** @format double */
  memoryCache: number;
  /** @format double */
  memoryLimit: number;
  /** @format double */
  rxBytes: number;
  /** @format double */
  txBytes: number;
}

export interface ContainerStatView {
  /** @format uuid */
  containerId: string;
  /** @format double */
  cpuUsage: number;
  /** @format int64 */
  created: number;
  /** @format double */
  memoryActive: number;
  /** @format double */
  memoryCache: number;
  /** @format double */
  memoryLimit: number;
  /** @format double */
  rxBytes: number;
  /** @format double */
  txBytes: number;
}

export interface ContainerSummaryView {
  capabilities: null | PlatformCapabilitiesView;
  containerId: string;
  deploymentView: null | ContainerDeploymentView;
  finishedAt: string;
  imageView: null | ImageView;
  name: string;
  networks: Record<string, string>;
  /** @format uuid */
  platformId: string;
  platformName: string;
  ports: Record<string, HostPortBinding[] | null>;
  startedAt: string;
  state: ContainerStateStatus;
  volumes: string[];
}

export interface ContainerView {
  capabilities: null | PlatformCapabilitiesView;
  containerId: string;
  controlState: ResourceControlState;
  /** @format int64 */
  created: number;
  /** @format uuid */
  deploymentId: string | null;
  deploymentView?: null | ContainerDeploymentView;
  dockerImageId: string;
  dockerNodeId: string | null;
  hasCitadelOwnershipLabels: boolean;
  /** @format uuid */
  id: string;
  imageView?: null | ImageView;
  isSwarmTask: boolean;
  isSystem: boolean;
  lastStats: null | ContainerStatView;
  name: string;
  nodeHostname: string | null;
  /** @format uuid */
  platformId: string;
  ports: Record<string, HostPortBinding[]>;
  /** @format int64 */
  projectionObservedAt: number | null;
  projectionStaleReason: string | null;
  /** @format int64 */
  projectionStaleSince: number | null;
  stack: string | null;
  /** @format uuid */
  stackId: string | null;
  state: ContainerStateStatus;
  systemRole: null | ContainerSystemRole;
  /** @format int64 */
  updated: number;
}

export interface ContainersResponse {
  capabilities: PlatformCapabilitiesView;
  containers: ContainerView[];
}

export interface Count {
  /** @format int64 */
  count: number;
}

export interface CreateDeploymentInput {
  description?: string | null;
  duplicateSource?: null | DuplicateSourceInput;
  name: string;
  /** @format uuid */
  platformId: string;
  spec: DeploymentSpec;
  tagIds?: string[] | null;
}

export interface CreateDeploymentInputView {
  description?: string | null;
  duplicateSource: DuplicateSourceInput;
  name: string;
  /** @format uuid */
  platformId: string;
  spec: DeploymentSpec;
  tagIds: string[];
}

export type CreateNetworkInput = CreateRuntimeNetwork & {
  /** @format uuid */
  platformId: string;
};

export interface CreateOidcProviderRequest {
  allowEmailAutoLink: boolean;
  allowedEmailDomains?: string | null;
  autoProvisionUsers: boolean;
  clientId: string;
  clientSecret?: string | null;
  /** @format uuid */
  defaultRoleId?: string | null;
  description?: string | null;
  displayName: string;
  enabled: boolean;
  issuer: string;
  name: string;
  requireEmailVerified: boolean;
  requiredClaimName?: string | null;
  requiredClaimValues?: string | null;
  scopes?: string | null;
}

export interface CreatePlatformInput {
  address?: string | null;
  connectorType?: PlatformConnectorType;
  description?: string | null;
  name: string;
  pruneHistoricalSwarmTaskContainers?: boolean;
  tagIds?: string[];
  type?: PlatformType;
}

export interface CreateRoleRequest {
  name: string;
  permissions?: RolePermissionInput[] | null;
}

export interface CreateRuntimeNetwork {
  attachable?: boolean | null;
  configFrom?: null | RuntimeConfigFrom;
  configOnly?: boolean | null;
  driver: string;
  enableIPv4?: boolean | null;
  enableIPv6?: boolean | null;
  ingress?: boolean | null;
  internal?: boolean | null;
  ipam?: null | RuntimeIpam;
  labels?: Record<string, string>;
  name: string;
  options?: Record<string, string>;
  scope: string;
}

export interface CreateRuntimeVolume {
  driver: string;
  labels?: Record<string, string>;
  name: string;
  options?: Record<string, string>;
}

export interface CreateServiceAccountRequest {
  description?: string | null;
  isEnabled?: boolean;
  name: string;
  resourceAccesses?: ServiceAccountResourceAccess[];
  roleIds?: string[];
  teamIds?: string[];
}

export interface CreateServiceAccountTokenRequest {
  /** @format date-time */
  expiresAtUtc?: string | null;
  name: string;
  neverExpires?: boolean;
}

export interface CreateStackInput {
  description?: string | null;
  driftPolicy?: null | StackDriftPolicy;
  duplicateSource?: null | DuplicateSourceInput;
  name: string;
  /** @format uuid */
  platformId: string;
  spec: StackSpec;
  stackSource: StackSource;
  tagIds?: string[];
}

export interface CreateSwarmMaterialInput {
  data?: string;
  labels?: Record<string, string>;
  name: string;
}

export interface CreateSwarmServiceInput {
  description?: string | null;
  duplicateSource?: null | DuplicateSourceInput;
  name: string;
  /** @format uuid */
  platformId: string;
  spec: SwarmServiceSpec;
  tagIds?: string[] | null;
}

export interface CreateTeamRequest {
  name: string;
  resourceAccesses?: ResourceAccessInput[];
  roleIds?: string[];
  userIds?: string[];
}

export interface CreateUserRequest {
  email: string;
  isEnabled?: boolean;
  name: string;
  password: string;
  resourceAccesses?: ResourceAccessInput[];
  roleIds?: string[];
  teamIds?: string[];
}

export type CreateVolumeInput = CreateRuntimeVolume & {
  /** @format uuid */
  platformId: string;
};

export interface CreatedNetworkView {
  id: string;
}

export type CreatedServiceAccountTokenView = ServiceAccountTokenView & {
  token: string;
};

export interface CurrentProfileAuthenticationView {
  canChangePassword: boolean;
  canUseLocalPasswordMfa: boolean;
  label: string;
  /** @format uuid */
  oidcProviderId: string | null;
  oidcProviderName: string | null;
  type: CurrentProfileAuthenticationType;
}

export interface CurrentProfileAuthorizationView {
  alertRules: ResourceCapabilitiesView;
  bindings: ResourceCapabilitiesView;
  isAdministrator: boolean;
  tags: ResourceCapabilitiesView;
}

export interface CurrentProfileView {
  authentication: CurrentProfileAuthenticationView;
  authorization: CurrentProfileAuthorizationView;
  /** @format date-time */
  createdAt: string;
  directRoles: ProfileResourceInfo[];
  displayName: string;
  email: string;
  /** @format uuid */
  id: string;
  teams: ProfileResourceInfo[];
}

export interface DeleteContainerOptions {
  force?: boolean;
  link?: boolean;
  v?: boolean;
}

export interface DeleteGitAccountsInput {
  ids: string[];
}

export interface DeleteImageView {
  result: Record<string, string>;
}

export interface DeleteImagesInput {
  force?: boolean;
  ids: string[];
  noPrune?: boolean;
  /** @format uuid */
  platformId: string;
}

export interface DeleteImagesView {
  items: DeleteImageView[];
}

export type DeleteInput = DeleteContainerOptions & {
  containerIds: string[];
};

export interface DeleteNetworksInput {
  ids: string[];
  /** @format uuid */
  platformId: string;
}

export interface DeletePlatformsInput {
  ids: string[];
}

export interface DeleteResourcesInput {
  ids: string[];
}

export interface DeleteRolesRequest {
  ids: string[];
}

export interface DeleteSwarmResourcesInput {
  ids?: string[];
}

export interface DeleteTeamsRequest {
  ids: string[];
}

export interface DeleteUsersRequest {
  ids: string[];
}

export interface DeleteVolumesInput {
  force?: boolean | null;
  names: string[];
  /** @format uuid */
  platformId: string;
}

export interface DeploymentActivitySnapshot {
  description?: string | null;
  /** @format uuid */
  id: string;
  name: string;
  /** @format uuid */
  platformId: string;
  spec: any;
}

export interface DeploymentApplyError {
  /** @format int64 */
  code: number;
  message: string;
}

export interface DeploymentCapabilities {
  canApply: boolean;
  canExecute: boolean;
  canInspect: boolean;
  canOpenTerminal: boolean;
  canPull: boolean;
  canRead: boolean;
  canViewLogs: boolean;
  canViewResourceBindings: boolean;
  canWrite: boolean;
}

export interface DeploymentConfigView {
  description?: string | null;
  /** @format uuid */
  id: string;
  name: string;
  /** @format uuid */
  platformId: string;
  spec: DeploymentSpec;
}

export interface DeploymentDuplicateDraftView {
  draft: CreateDeploymentInputView;
  warnings: DuplicateWarning[];
}

export type DeploymentImageInfo =
  | {
      $type: "Local";
      imageId: string;
    }
  | {
      $type: "External";
      imageTag: string;
      /** @format uuid */
      registryId: string;
      resolvedDigest?: string | null;
    }
  | {
      $type: "Build";
      /** @format date-time */
      appliedAt?: string | null;
      /** @format uuid */
      appliedBuildRunId?: string | null;
      appliedDigest?: string | null;
      appliedImageReference?: string | null;
      /** @format uuid */
      buildProjectId: string;
      redeployOnBuild?: boolean;
      /** @format uuid */
      resolvedBuildRunId?: string | null;
      resolvedDigest?: string | null;
      resolvedImageReference?: string | null;
    };

export interface DeploymentResultActivitySnapshot {
  containerIds?: string[] | null;
  message?: string | null;
  resourceBindings?: any;
}

export interface DeploymentSpec {
  command?: string[] | null;
  environmentVariables?: string[] | null;
  image: DeploymentImageInfo;
  labels?: Record<string, string> | null;
  lifeCycleSpec?: null | LifeCycleSpec;
  networks?: string[] | null;
  ports?: string[] | null;
  resourceSpec?: null | ResourceSpec;
  /** @default "Disabled" */
  updateBehavior?: UpdateBehavior;
  volumes?: string[] | null;
}

export interface DeploymentStreamItem {
  error?: null | DeploymentApplyError;
  errorMessage?: string | null;
  id?: string | null;
  progress?: null | DeploymentsModelImagePullProgress;
  progressMessage?: string | null;
  status?: string | null;
  stream?: string | null;
}

export interface DeploymentView {
  autoUpdateState?: null | AutoUpdateState;
  capabilities?: null | DeploymentCapabilities;
  /** @format uuid */
  containerId?: string | null;
  controlState: ResourceControlState;
  /** @format date-time */
  createdAt: string;
  /** @format uuid */
  createdByActorId: string;
  description?: string | null;
  dockerContainerId?: string | null;
  dockerImageId?: string | null;
  /** @format uuid */
  id: string;
  /** @format uuid */
  imageId?: string | null;
  imageName?: string | null;
  latestActivityView?: null | LatestActivityView;
  name: string;
  /** @format uuid */
  platformId: string;
  platformName?: string | null;
  platformStatus: PlatformStatus;
  spec: DeploymentSpec;
  status: DeploymentStatus;
  tags: ResourcesTagsTagSummary[];
}

export interface DeploymentsView {
  capabilities: ResourceCapabilitiesView;
  deployments: DeploymentView[];
}

export interface DisableProfileMfaInput {
  code?: string | null;
  password: string;
  recoveryCode?: string | null;
}

export interface DockerHubRepositoryInfo {
  isAutomated?: boolean;
  isPrivate?: boolean;
  isTrusted?: boolean;
  lastUpdated?: string | null;
  name?: string | null;
  namespace?: string | null;
  /** @format int64 */
  pullCount?: number;
}

export interface DockerHubTag {
  /** @format int64 */
  fullSize?: number | null;
  /** @format int64 */
  id?: number | null;
  image?: null | DockerHubTagImage;
  lastPulled?: string | null;
  lastUpdated?: string | null;
  name?: string | null;
  status: RegistryTagStatus;
}

export interface DockerHubTagImage {
  architecture?: string | null;
  digest?: string | null;
  lastPulled?: string | null;
  os?: string | null;
  /** @format int64 */
  size?: number | null;
  status: RegistryTagStatus;
}

export interface DockerPlatformDescriptor {
  apiVersion?: string | null;
  architecture?: string | null;
  /** @format int64 */
  containerCount?: number | null;
  /** @format int64 */
  containersPaused?: number | null;
  /** @format int64 */
  containersRunning?: number | null;
  /** @format int64 */
  containersStopped?: number | null;
  daemonId?: string;
  driver?: string | null;
  /** @format int64 */
  imageUsedBytes?: number | null;
  minimumApiVersion?: string | null;
  operatingSystem?: string | null;
  osType?: string | null;
  osVersion?: string | null;
  /** @format int64 */
  volumeUsedBytes?: number | null;
}

export type DockerSwarmPlatformDescriptor = DockerPlatformDescriptor & {
  clusterCreatedAt?: string | null;
  clusterId?: string | null;
  controlAvailable?: boolean | null;
  error?: string | null;
  localNodeState?: string | null;
  /** @format int64 */
  managers?: number | null;
  nodeAddr?: string | null;
  nodeID?: string | null;
  /** @format int64 */
  nodes?: number | null;
  remoteManagers?: RuntimeSwarmPeer[] | null;
  /** @format int64 */
  runningTaskCount?: number | null;
  /** @format int64 */
  serviceCount?: number | null;
};

export interface DuplicateDraftWarning {
  code: string;
  fieldPath: string | null;
  message: string;
}

export interface DuplicateSourceInput {
  /** @format uuid */
  resourceId: string;
  resourceName: string;
  resourceType: ActivityResourceType;
}

export interface DuplicateWarning {
  code: string;
  fieldPath?: string | null;
  message: string;
}

export interface EdgeEnrollmentView {
  /** @format uuid */
  enrollmentId: string;
  /** @format date-time */
  expiresAtUtc: string;
  instructions: EdgeInstructionsView;
  /** @format uuid */
  platformId: string;
  token: string;
}

export interface EdgeInstructionsView {
  agentImage: string;
  coreUrl: string;
  dockerRunCommand: string;
  environment: Record<string, string>;
}

export interface EdgeStatusView {
  agentFingerprint?: string | null;
  connectionStatus: string;
  /** @format date-time */
  enrollmentExpiresAtUtc?: string | null;
  /** @format date-time */
  lastConnectedAtUtc?: string | null;
  /** @format date-time */
  lastDisconnectedAtUtc?: string | null;
  /** @format date-time */
  lastHeartbeatAtUtc?: string | null;
  lastSeenHostname?: string | null;
  lastSeenVersion?: string | null;
  /** @format int32 */
  protocolVersion?: number | null;
  /** @format date-time */
  revokedAtUtc?: string | null;
}

export interface ExposedPortsView {
  ports: string[];
}

/** Repository summaries returned by the registry browser, never credentials. */
export type ExternalRepository =
  | {
      $type: "DockerHub";
      isPrivate: boolean;
      lastUpdated?: string | null;
      name?: string | null;
      namespace?: string | null;
      /** @format int64 */
      pullCount: number;
    }
  | {
      $type: "GitHub";
      createdAt?: string | null;
      htmlUrl?: string | null;
      id: string;
      name?: string | null;
      updatedAt?: string | null;
      url?: string | null;
    };

export interface ExternalSecretInput {
  externalKey: string;
  externalPath: string;
  /** @format int32 */
  externalVersion?: number | null;
  name: string;
  /** @format uuid */
  providerId: string;
}

export interface ExternalSecretPatch {
  externalKey?: string | null;
  externalPath?: string | null;
  /** @format int32 */
  externalVersion?: number | null;
  name?: string | null;
  /** @format uuid */
  providerId?: string | null;
}

export interface GitAccountConfigView {
  authType: GitAuthType;
  configuration: GitAuthConfiguration;
  domain: string;
  /** @format uuid */
  id: string;
  name: string;
  transport: GitTransport;
}

export interface GitAccountInput {
  authType: GitAuthType;
  configuration: GitAuthConfiguration;
  domain: string;
  name: string;
  transport: GitTransport;
}

export interface GitAccountPatch {
  authType?: null | GitAuthType;
  configuration?: null | GitAuthConfiguration;
  domain?: string | null;
  name?: string | null;
  transport?: null | GitTransport;
}

export interface GitAccountView {
  authType: GitAuthType;
  /** @format date-time */
  createdAt: string;
  /** @format uuid */
  createdByActorId: string;
  domain: string;
  /** @format uuid */
  id: string;
  name: string;
  transport: GitTransport;
}

export interface GitAccountsResponse {
  capabilities: ResourceCapabilitiesView;
  gitAccounts: AuthorizedGitAccountView[];
}

export type GitAuthConfiguration =
  | {
      $type: "Basic";
      password: string;
      username: string;
    }
  | {
      $type: "Token";
      token: string;
    }
  | {
      $type: "SshKey";
      passphrase?: string | null;
      privateKey: string;
      username: string;
    };

export interface GitChangedPath {
  path: string;
  previousPath?: string | null;
  status: GitChangedPathStatus;
}

export interface GitCommitComparison {
  baseCommitSha: string;
  files: GitChangedPath[];
  headCommitSha: string;
  isTruncated: boolean;
  /** @format uuid */
  repositoryId: string;
}

export interface GitComposeDiscovery {
  branch: string;
  projects: GitComposeProjectCandidate[];
  /** @format uuid */
  repositoryId: string;
  resolvedCommitSha: string;
}

export interface GitComposeProjectCandidate {
  composePaths: string[];
  envFilePaths: string[];
  suggestedWatchPaths: string[];
  workingDirectory: string;
}

export interface GitDirectoryEntry {
  mode: string;
  name: string;
  path: string;
  /**
   * @format int64
   * @min 0
   */
  size?: number | null;
  targetCommitSha?: string | null;
  type: GitEntryTypeView;
}

export interface GitDirectoryListing {
  commitSha: string;
  entries: GitDirectoryEntry[];
  isTruncated: boolean;
  path: string;
  providerRepositoryUrl?: string | null;
  /** @format uuid */
  repositoryId: string;
}

export interface GitFileContent {
  commitSha: string;
  content?: string | null;
  isBinary: boolean;
  isTruncated: boolean;
  path: string;
  previewUnavailableReason?: string | null;
  providerUrl?: string | null;
  /** @format uuid */
  repositoryId: string;
  /**
   * @format int64
   * @min 0
   */
  size: number;
  type: GitEntryTypeView;
}

export interface GitRepositoriesResponse {
  capabilities: ResourceCapabilitiesView;
  gitRepositories: AuthorizedGitRepositoryView[];
}

export interface GitRepositoryActivitySnapshot {
  defaultBranch: string;
  description?: string | null;
  /** @format uuid */
  gitAccountId?: string | null;
  /** @format uuid */
  id: string;
  name: string;
  onClone?: any;
  onPull?: any;
  resolvedCommitSha?: string | null;
  /** @format int32 */
  syncIntervalMinutes?: number | null;
  syncMode: string;
  url: string;
  webhook?: null | WebhookConfig;
}

export interface GitRepositoryBranchResponse {
  branch: string;
  commitSha: string;
}

export interface GitRepositoryBranchesResponse {
  branches: GitRepositoryBranchResponse[];
}

export interface GitRepositoryConfigResponse {
  defaultBranch: string;
  description: string | null;
  /** @format uuid */
  gitAccountId: string | null;
  /** @format uuid */
  id: string;
  name: string;
  onClone: null | RepoCommand;
  onPull: null | RepoCommand;
  /** @format int32 */
  syncIntervalMinutes: number | null;
  syncMode: GitRepositorySyncMode;
  tags: ResourcesTagsTagSummary[];
  url: string;
  webhook: null | WebhookConfig;
}

export interface GitRepositoryPatch {
  defaultBranch?: string | null;
  description?: string | null;
  /** @format uuid */
  gitAccountId?: string | null;
  name?: string | null;
  onClone?: null | RepoCommand;
  onPull?: null | RepoCommand;
  /** @format int32 */
  syncIntervalMinutes?: number | null;
  syncMode?: null | GitRepositorySyncMode;
  tagIds?: string[] | null;
  url?: string | null;
  webhook?: null | WebhookPatch;
}

export interface GitRepositoryRefView {
  branch: string;
  /** @format uuid */
  gitRepositoryId: string;
  /** @format uuid */
  id: string;
  lastError: string | null;
  /** @format date-time */
  lastSyncedAt: string;
  resolvedCommitSha: string | null;
  status: GitRepositoryRefStatus;
}

export interface GitRepositoryRefsResponse {
  refs: GitRepositoryRefView[];
}

export interface GitRepositorySyncActivitySnapshot {
  commitSha?: string | null;
  message?: string | null;
}

export interface GitRepositoryView {
  controlState: ResourceControlState;
  /** @format date-time */
  createdAt: string;
  /** @format uuid */
  createdByActorId: string;
  defaultBranch: string;
  description: string | null;
  /** @format uuid */
  gitAccountId: string | null;
  /** @format uuid */
  id: string;
  latestActivityView: null | LatestActivityView;
  name: string;
  onClone: null | RepoCommand;
  onPull: null | RepoCommand;
  status: GitRepositoryStatus;
  /** @format int32 */
  syncIntervalMinutes: number | null;
  syncMode: GitRepositorySyncMode;
  tags: ResourcesTagsTagSummary[];
  url: string;
  webhook: null | WebhookConfig;
}

export interface GithubContainerMetadata {
  tags?: string[] | null;
}

export interface GithubPackageMetadata {
  container?: null | GithubContainerMetadata;
}

export interface GithubPackageVersion {
  createdAt?: string | null;
  htmlUrl?: string | null;
  /** @format int64 */
  id: number;
  metadata?: null | GithubPackageMetadata;
  name: string;
  packageHtmlUrl?: string | null;
  updatedAt?: string | null;
  url?: string | null;
}

export type GlobalBindingsResponse = ResourceBindingsView & {
  capabilities: ResourceCapabilitiesView;
};

export interface GlobalSearchGroup {
  category: GlobalSearchCategory;
  items: GlobalSearchItem[];
}

export interface GlobalSearchItem {
  /** @format uuid */
  id: string;
  name: string;
  parent?: null | GlobalSearchParent;
  resourceType: GlobalSearchResourceType;
  secondaryText?: string | null;
  status?: null | GlobalSearchStatus;
}

export interface GlobalSearchParent {
  /** @format uuid */
  id: string;
  name: string;
  resourceType: GlobalSearchResourceType;
}

export interface GlobalSearchResponse {
  groups: GlobalSearchGroup[];
  query: string;
}

export interface GlobalSearchStatus {
  label: string;
  tone: SearchStatusTone;
}

export interface HealthResponse {
  status: string;
}

export interface HistoryContainerStatSnapshot {
  stats: {
    /** @format uuid */
    containerId: string;
    /** @format double */
    cpuUsage: number;
    /** @format int64 */
    created: number;
    /** @format double */
    memoryActive: number;
    /** @format double */
    memoryCache: number;
    /** @format double */
    memoryLimit: number;
    /** @format double */
    rxBytes: number;
    /** @format double */
    txBytes: number;
  }[];
}

export interface HistoryPlatformStatSnapshot {
  stats: {
    /** @format double */
    cpuUsage: number;
    /** @format int64 */
    created: number;
    /** @format int64 */
    diskTotalBytes: number | null;
    /** @format double */
    diskUsage: number | null;
    /** @format int64 */
    diskUsedBytes: number | null;
    /** @format double */
    memoryUsage: number;
    /** @format double */
    rxBytes: number;
    /** @format double */
    txBytes: number;
  }[];
}

export interface HostPortBinding {
  hostIP?: string | null;
  hostPort?: string | null;
}

export interface IdentityResourceAccessSnapshot {
  permissionLevel: PermissionLevel;
  /** @format uuid */
  resourceId: string;
  resourceType: ResourceType;
  /** @format int32 */
  specificPermissions: number;
}

export interface Ids {
  ids: string[];
}

export interface ImageCapabilitiesView {
  canExecute: boolean;
  canInspect: boolean;
  canPull: boolean;
  canRead: boolean;
  canWrite: boolean;
}

export interface ImageContainer {
  id: string;
  name: string;
  networks: Record<string, string>;
  ports: any;
  state: string;
  volumes: string[];
}

export interface ImageInspection {
  architecture: string;
  cmd: string[];
  containers: ImageContainer[];
  created: string;
  dockerNodeId?: string | null;
  entryPoint: string[];
  env: string[];
  exposedPorts: string[];
  id: string;
  labels: Record<string, string>;
  layers: ImageLayer[];
  name: string;
  os: string;
  registry?: any;
  repoTags: string[];
  /** @format int64 */
  size: number;
  stopSignal?: string | null;
  tag: string;
  user?: string | null;
  volumes: string[];
  workingDir?: string | null;
}

export type ImageInspectionView = ImageInspection & {
  capabilities?: null | ImageCapabilitiesView;
};

export interface ImageLayer {
  comment: string;
  /** @format int64 */
  created: number;
  createdBy: string;
  id: string;
  /** @format int64 */
  size: number;
}

export interface ImagePullError {
  /** @format int64 */
  code?: number | null;
  message?: string | null;
}

export interface ImageRegistryView {
  /** @format uuid */
  id: string;
  name: string;
  registryHost: string;
  type: RegistryKind;
}

export interface ImageUpdateState {
  currentDigest: string;
  imageName: string;
  /** @format date-time */
  lastCheckedAt: string;
  remoteDigest?: string | null;
  serviceName: string;
  updateAvailable: boolean;
}

export interface ImageView {
  capabilities: null | ImageCapabilitiesView;
  contentIdentity: string | null;
  controlState: ResourceControlState;
  /** @format date-time */
  createdAt: string;
  dockerImageId: string;
  dockerNodeId: string | null;
  /** @format uuid */
  id: string;
  isInUse: boolean;
  isStale: boolean;
  name: string;
  nodeHostname: string | null;
  /** @format uuid */
  platformId: string;
  registry?: null | ImageRegistryView;
  /** @format uuid */
  registryId: string | null;
  repoDigests: string[] | null;
  /** @format double */
  size: number;
  staleReason: string | null;
  tags: string[];
  /** @format date-time */
  updatedAt: string | null;
}

export interface ImagesResponse {
  capabilities: ImageCapabilitiesView;
  images: ImageView[];
}

export interface ImportRequest {
  description?: string | null;
  importKind?: null | StackImportKind;
  importSensitiveEnvironmentAsSecrets?: boolean;
  name: string;
  previewFingerprint: string;
  spec: StackSpec;
  stackSource: StackSource;
  tagIds?: string[];
}

export interface InitializeCitadelRequest {
  email: string;
  name: string;
  password: string;
}

export interface InstallLicenseRequest {
  license: string;
}

export interface InternalSecretInput {
  name: string;
  value: string;
}

export interface KubernetesPlatformDescriptor {
  apiServerUrl?: string | null;
  clusterName?: string | null;
  clusterVersion?: string | null;
  namespace?: string | null;
}

/** Activity summary embedded in resource HTTP and realtime responses. */
export interface LatestActivityView {
  /** @format date-time */
  createdAt: string;
  eventType: ActivityEventType;
  /** @format uuid */
  id: string;
  info: PublicActivityEventInfo;
  resourceType: ActivityResourceType;
  status: ActivityStatus;
}

export interface LicenseActivitySnapshot {
  customerId?: string | null;
  customerName?: string | null;
  effectiveCapabilities: LicenseCapability[];
  effectiveEdition: string;
  /** @format date-time */
  expiresAt?: string | null;
  fingerprint?: string | null;
  /** @format date-time */
  graceUntil?: string | null;
  licenseId?: string | null;
  licensedEdition?: string | null;
  replacedLicenseId?: string | null;
  /**
   * @format int32
   * @min 0
   */
  schema?: number | null;
  status: LicenseStatus;
}

export interface LicenseCapabilityView {
  capability: LicenseCapability;
  enabled: boolean;
}

export interface LicenseEntitlementsView {
  capabilities: LicenseCapabilityView[];
  effectiveEdition: string;
  status: LicenseStatus;
}

export interface LicenseRequestView {
  coreVersion: string;
  /** @format date-time */
  generatedAt: string;
  /** @format uuid */
  instanceId: string;
  product: string;
}

export interface LicenseView {
  capabilities: LicenseCapabilityView[];
  customerId: string | null;
  customerName: string | null;
  effectiveEdition: string;
  /** @format date-time */
  expiresAt: string | null;
  fingerprint: string | null;
  /** @format date-time */
  graceUntil: string | null;
  /** @format uuid */
  instanceId: string;
  /** @format date-time */
  issuedAt: string | null;
  licenseId: string | null;
  /**
   * @format int32
   * @min 0
   */
  licenseSchema: number | null;
  licensedEdition: string | null;
  /** @format date-time */
  notBefore: string | null;
  replacedLicenseId: string | null;
  status: LicenseStatus;
  warnings: string[];
}

export interface LifeCycleSpec {
  restartPolicy?: ContainerRestartPolicy;
  stopSignal?: null | StopSignal;
  /** @format int32 */
  stopTimeout?: number | null;
}

export interface LogSnapshot {
  lines: string[];
  truncated: boolean;
}

export interface LoginRequest {
  emailOrName: string;
  password: string;
}

export interface LoginResponse {
  accessToken: string | null;
  nextStep: LoginNextStep;
}

export interface LookupResourceInfo {
  group?: string | null;
  /** @format uuid */
  id: string;
  name: string;
}

export interface ManagedSwarmServiceView {
  appliedImageDigest: string | null;
  autoUpdateState: AutoUpdateState;
  capabilities: null | SwarmServiceCapabilities;
  controlState: ResourceControlState;
  /** @format date-time */
  createdAt: string;
  currentOperation: null | SwarmServiceOperationView;
  description: string | null;
  /** @format int32 */
  desiredTaskCount: number | null;
  dockerName: string;
  dockerServiceId: string | null;
  hasPendingDesiredChanges: boolean;
  hasRuntimeDrift: boolean;
  health: SwarmServiceHealth;
  /** @format uuid */
  id: string;
  name: string;
  /** @format uuid */
  platformId: string;
  platformName: string | null;
  platformStatus: PlatformStatus;
  /** @format int64 */
  rowVersion: number;
  /** @format int32 */
  runningTaskCount: number | null;
  spec: SwarmServiceSpec;
  synchronizationState: SwarmServiceSynchronizationState;
  tags: ResourcesTagsTagSummary[];
  tasks: any[] | null;
  updateMessage: string | null;
  updateState: string | null;
  /** @format date-time */
  updatedAt: string;
}

export interface ManagedSwarmServicesView {
  capabilities: ResourceCapabilitiesView;
  swarmServices: ManagedSwarmServiceView[];
}

export interface MandatoryMfaSetupCompleteView {
  accessToken: string;
  recoveryCodes: string[];
}

export interface MandatoryMfaSetupView {
  /** @format date-time */
  expiresAt: string;
  otpAuthUri: string;
  secret: string;
}

export interface MfaVerificationInput {
  code?: string | null;
  recoveryCode?: string | null;
}

export interface MfaVerificationView {
  accessToken: string;
}

export interface NetworkAttachmentView {
  endpointId: string | null;
  ipV4Address: string | null;
  ipv6Address: string | null;
  macAddress: string | null;
  name: string | null;
}

export interface NetworkCapabilitiesView {
  canExecute: boolean;
  canInspect: boolean;
  canRead: boolean;
  canWrite: boolean;
}

export interface NetworkIpamView {
  config: RuntimeIpamConfig[];
  driver: string | null;
  options: Record<string, string>;
}

export interface NetworkPeerView {
  ip: string | null;
  name: string | null;
}

export interface NetworkView {
  attachable: boolean;
  capabilities: null | NetworkCapabilitiesView;
  configFrom: string | null;
  configOnly: boolean;
  containers: Record<string, NetworkAttachmentView>;
  created: string;
  dockerNodeId: string | null;
  driver: string;
  enableIPv4: boolean;
  enableIPv6: boolean;
  id: string;
  inUse: boolean;
  ingress: boolean;
  internal: boolean;
  ipam: null | NetworkIpamView;
  isStale: boolean;
  isSystem: boolean;
  labels: Record<string, string>;
  name: string;
  nodeHostname: string | null;
  options: Record<string, string>;
  peers: NetworkPeerView[];
  scope: string;
  staleReason: string | null;
}

export interface NetworksResponse {
  capabilities: ResourceCapabilitiesView;
  networks: NetworkView[];
}

export interface NewGitRepository {
  defaultBranch: string;
  description?: string | null;
  /** @format uuid */
  gitAccountId?: string | null;
  name: string;
  onClone?: null | RepoCommand;
  onPull?: null | RepoCommand;
  /** @format int32 */
  syncIntervalMinutes?: number | null;
  syncMode?: GitRepositorySyncMode;
  tagIds?: string[];
  url: string;
  webhook?: null | WebhookConfig;
}

export interface NewRegistry {
  configuration: RegistrySpec;
  description?: string | null;
  name: string;
  /** Required for custom registry addresses; inferred for Docker Hub and GitHub. */
  registryHost?: string;
  status: RegistryStatus;
  tagIds?: string[];
}

export interface NewResourceBinding {
  kind: ResourceBindingKind;
  name: string;
  secretDeliveryMode?: null | SecretDeliveryMode;
  /** @format uuid */
  secretId?: string | null;
  targetPath?: string | null;
  value?: string | null;
}

export interface NewTag {
  color: string;
  name: string;
}

export interface NodeAgentCoverage {
  agentConnectionState: string;
  architecture: string;
  availability: string;
  compatible: boolean;
  dataSource: string;
  dockerNodeId: string;
  dockerReachable: boolean;
  eligible: boolean;
  hostname: string;
  /** @format date-time */
  lastHeartbeatAtUtc?: string | null;
  /** @format date-time */
  lastSuccessfulReconciliationAt?: string | null;
  nodeStatus: string;
  projectionStale: boolean;
  reasons: string[];
  role: string;
  schedulable: boolean;
  serviceTaskState?: string | null;
  staleReason?: string | null;
  /** @format date-time */
  staleSince?: string | null;
  supported: boolean;
}

export interface NodeAgentOperation {
  error?: string | null;
  kind: string;
  /** @format uuid */
  operationId: string;
  /** @format date-time */
  startedAtUtc: string;
  state: string;
}

export interface NodeAgentProgress {
  errorMessage?: string | null;
  isCompleted: boolean;
  isWarning: boolean;
  message: string;
  /** @format uuid */
  operationId: string;
  /** @format uuid */
  platformId: string;
  stage: string;
}

export interface NodeInspectionView {
  address: string;
  architecture: string;
  availability: string;
  /** @format date-time */
  createdAt?: string | null;
  /** @format int32 */
  desiredTaskCount: number;
  engineVersion: string;
  hostname: string;
  id: string;
  isLeader: boolean;
  labels: Record<string, string>;
  operatingSystem: string;
  reachability: string;
  role: string;
  /** @format int32 */
  runningTaskCount: number;
  status: string;
  statusMessage?: string | null;
  /** @format date-time */
  updatedAt?: string | null;
  /** @format int64 */
  versionIndex: number;
}

export interface OidcDiscoveryView {
  authorizationEndpoint: string;
  issuer: string;
  jwksUri: string;
  tokenEndpoint: string;
}

export interface OidcLoginProviderView {
  displayName: string;
  /** @format uuid */
  id: string;
}

export interface OidcLoginProvidersView {
  providers: OidcLoginProviderView[];
}

export interface OidcProviderActivitySnapshot {
  allowEmailAutoLink: boolean;
  allowedEmailDomains?: string | null;
  autoProvisionUsers: boolean;
  clientId: string;
  /** @format uuid */
  defaultRoleId?: string | null;
  description?: string | null;
  displayName: string;
  enabled: boolean;
  /** @format uuid */
  id: string;
  issuer: string;
  name: string;
  requireEmailVerified: boolean;
  requiredClaimName?: string | null;
  requiredClaimValues?: string | null;
  scopes: string;
}

export interface OidcProviderView {
  allowEmailAutoLink: boolean;
  allowedEmailDomains: string | null;
  autoProvisionUsers: boolean;
  clientId: string;
  /** @format date-time */
  createdAt: string;
  /** @format uuid */
  createdByActorId: string;
  /** @format uuid */
  defaultRoleId: string | null;
  description: string | null;
  displayName: string;
  enabled: boolean;
  hasClientSecret: boolean;
  /** @format uuid */
  id: string;
  issuer: string;
  name: string;
  requireEmailVerified: boolean;
  requiredClaimName: string | null;
  requiredClaimValues: string | null;
  scopes: string;
  /** @format date-time */
  updatedAt: string;
}

export interface OidcProvidersView {
  providers: OidcProviderView[];
}

export interface PagedActivityView {
  items: ActivityView[];
  /** @format int32 */
  page: number;
  /** @format int32 */
  pageSize: number;
  /** @format int64 */
  totalCount: number;
}

export interface PagedResultServiceAccountTokenView {
  items: {
    /** @format date-time */
    createdAtUtc: string;
    createdByActorId: ActorId;
    createdByName: string;
    /** @format date-time */
    expiresAtUtc: string | null;
    hint: string;
    /** @format uuid */
    id: string;
    /** @format date-time */
    lastUsedAtUtc: string | null;
    name: string;
    /** @format date-time */
    revokedAtUtc: string | null;
    revokedByActorId: null | ActorId;
  }[];
  /** @format int64 */
  page: number;
  /** @format int64 */
  pageSize: number;
  /** @format int64 */
  totalCount: number;
}

export interface PagedResultServiceAccountView {
  items: {
    /** @format int64 */
    activeTokenCount: number;
    actorId: ActorId;
    /** @format date-time */
    archivedAtUtc: string | null;
    /** @format date-time */
    createdAt: string;
    createdByActorId: ActorId;
    description: string | null;
    /** @format uuid */
    id: string;
    isEnabled: boolean;
    /** @format date-time */
    lastUsedAtUtc: string | null;
    name: string;
    resourceAccesses: ServiceAccountResourceAccess[];
    roles: ResourceInfo[];
    teams: ResourceInfo[];
    /** @format date-time */
    updatedAt: string;
  }[];
  /** @format int64 */
  page: number;
  /** @format int64 */
  pageSize: number;
  /** @format int64 */
  totalCount: number;
}

export interface PagedResultTeamView {
  items: {
    actorId: ActorId;
    /** @format uuid */
    id: string;
    isEnabled: boolean;
    members: TeamMemberView[] | null;
    name: string;
    resourceAccesses: ResourceAccessView[] | null;
    roles: ResourceInfo[] | null;
    /** @format int32 */
    totalMembers: number;
    users: ResourceInfo[] | null;
  }[];
  /** @format int64 */
  page: number;
  /** @format int64 */
  pageSize: number;
  /** @format int64 */
  totalCount: number;
}

export interface PagedResultUserView {
  items: {
    actorId: ActorId;
    email: string;
    /** @format uuid */
    id: string;
    isEnabled: boolean;
    name: string;
    resourceAccesses: ResourceAccessView[] | null;
    roles: ResourceInfo[] | null;
    teams: ResourceInfo[] | null;
  }[];
  /** @format int64 */
  page: number;
  /** @format int64 */
  pageSize: number;
  /** @format int64 */
  totalCount: number;
}

export interface PatchActorEnabledInput {
  isEnabled: boolean;
}

export interface PatchAlertChannelInput {
  alertDestination?: AlertDestination;
  isActive?: boolean;
  name?: string | null;
  url?: string;
}

export interface PatchAlertRuleInput {
  channelIds?: string[] | null;
  /** @format int32 */
  cooldownSeconds?: number | null;
  limitedTo?: AlertResourceScope[] | null;
  quietHours?: AlertQuietHour[] | null;
  /** @format int32 */
  requiredMatches?: number | null;
  severity?: AlertSeverity;
  status?: AlertRuleStatus;
  /** @format double */
  threshold?: number | null;
  type?: AlertType;
}

export interface PatchAlertRuleMetadata {
  description?: string | null;
}

export interface PatchDeploymentInput {
  description?: string | null;
  /** @format uuid */
  id?: string | null;
  name?: string | null;
  /** @format uuid */
  platformId?: string | null;
  spec?: any;
}

export interface PatchDeploymentMetadataInput {
  description?: string | null;
  tags?: string[] | null;
}

export interface PatchOidcProviderMetadataRequest {
  description?: string | null;
  tags?: string[] | null;
}

export interface PatchOidcProviderRequest {
  allowEmailAutoLink?: boolean | null;
  allowedEmailDomains?: string | null;
  autoProvisionUsers?: boolean | null;
  clientId?: string | null;
  clientSecret?: string | null;
  /** @format uuid */
  defaultRoleId?: string | null;
  description?: string | null;
  displayName?: string | null;
  enabled?: boolean | null;
  issuer?: string | null;
  name?: string | null;
  requireEmailVerified?: boolean | null;
  requiredClaimName?: string | null;
  requiredClaimValues?: string | null;
  scopes?: string | null;
}

export interface PatchPlatformMetadataInput {
  description?: string | null;
  tags?: string[] | null;
}

export interface PatchResourceMetadataInput {
  description?: string | null;
  tags?: string[] | null;
}

export interface PatchRolePermissionsRequest {
  permissions?: RolePermissionInput[] | null;
}

export interface PatchStackInput {
  description?: string | null;
  driftPolicy?: null | StackDriftPolicy;
  name?: string | null;
  /** @format uuid */
  platformId?: string | null;
  /** @format int64 */
  rowVersion?: number | null;
  spec?: any;
  stackSource?: null | StackSource;
}

export interface PatchStackMetadataInput {
  description?: string | null;
}

export interface PatchTeamRequest {
  isEnabled?: boolean | null;
  resourceAccesses?: ResourceAccessInput[] | null;
  roleIds?: string[] | null;
  userIds?: string[] | null;
}

export interface PatchUserPreferencesRequest {
  contentLayout?: null | UserContentLayout;
  dateTimeFormat?: null | UserDateTimeFormat;
  density?: null | UserUiDensity;
  font?: null | UserUiFont;
  radius?: null | UserUiRadius;
  theme?: null | UserTheme;
  themeColor?: null | UserThemeColor;
  timeZone?: string | null;
}

export interface PatchUserRequest {
  email?: string | null;
  isEnabled?: boolean | null;
  password?: string | null;
  resourceAccesses?: ResourceAccessInput[] | null;
  roleIds?: string[] | null;
  teamIds?: string[] | null;
}

export interface PermissionMatrixViewItem {
  label: string;
  maximumLevel: PermissionLevel;
  specificPermissionLabels: Record<string, string>;
  specificPermissions: Record<string, PermissionLevel>;
}

export interface PlatformActivitySnapshot {
  address: string;
  agentVersion?: string | null;
  connectorType: string;
  /** @format int64 */
  cpuCount: number;
  description?: string | null;
  /** @format uuid */
  id: string;
  /** @format int64 */
  imageCount: number;
  /** @format int64 */
  memTotal: number;
  name: string;
  /** @format int32 */
  networkCount: number;
  platformDescriptor: any;
  serverVersion?: string | null;
  status: string;
  /** @format int32 */
  volumeCount: number;
}

export interface PlatformBackupSummaries {
  platforms: PlatformBackupSummary[];
}

export interface PlatformBackupSummary {
  /** @format int32 */
  attentionPolicyCount: number;
  /** @format int32 */
  deploymentPolicyCount: number;
  /** @format int32 */
  dockerVolumePolicyCount: number;
  /** @format int32 */
  enabledPolicyCount: number;
  /** @format date-time */
  lastRunAt?: string | null;
  lastRunStatus?: null | BackupRunStatus;
  /** @format uuid */
  platformId: string;
  /** @format int32 */
  policyCount: number;
  /** @format int32 */
  stackPolicyCount: number;
  /** @format int32 */
  swarmServicePolicyCount: number;
}

export interface PlatformCapabilitiesView {
  canExecute: boolean;
  canInspect: boolean;
  canManageNodeAgents: boolean;
  canOpenTerminal: boolean;
  canPull: boolean;
  canRead: boolean;
  canViewLogs: boolean;
  canWrite: boolean;
}

export type PlatformDescriptor =
  | (DockerPlatformDescriptor & {
      $type: "Docker";
    })
  | (DockerSwarmPlatformDescriptor & {
      $type: "DockerSwarm";
    })
  | (KubernetesPlatformDescriptor & {
      $type: "Kubernetes";
    });

/** Omitted fields preserve their stored value. Only address and description allow null. */
export interface PlatformPatch {
  address?: string | null;
  connectorType?: PlatformConnectorType;
  description?: string | null;
  name?: string;
  pruneHistoricalSwarmTaskContainers?: boolean;
  type?: PlatformType;
}

export interface PlatformStatView {
  /** @format double */
  cpuUsage: number;
  /** @format int64 */
  created: number;
  /** @format int64 */
  diskTotalBytes: number | null;
  /** @format double */
  diskUsage: number | null;
  /** @format int64 */
  diskUsedBytes: number | null;
  /** @format double */
  memoryUsage: number;
  /** @format double */
  rxBytes: number;
  /** @format double */
  txBytes: number;
}

export interface PlatformView {
  address: string;
  agentVersion: string | null;
  capabilities: null | PlatformCapabilitiesView;
  clusterId: string | null;
  connectorType: PlatformConnectorType;
  /** @format int64 */
  cpuCount: number;
  /** @format int64 */
  deploymentCount: number;
  deploymentStatusCounts: WorkloadStatusCounts;
  description: string | null;
  /** @format uuid */
  id: string;
  /** @format int64 */
  imageCount: number;
  /** @format int64 */
  memTotal: number;
  name: string;
  /** @format int32 */
  networkCount: number;
  platformDescriptor: null | PlatformDescriptor;
  pruneHistoricalSwarmTaskContainers: boolean;
  serverVersion: string | null;
  /** @format int64 */
  stackCount: number;
  stackStatusCounts: WorkloadStatusCounts;
  stats: PlatformStatView[] | null;
  status: PlatformStatus;
  swarmServiceStatusCounts: WorkloadStatusCounts;
  tags: ResourcesTagsTagSummary[];
  type: PlatformType;
  /** @format int32 */
  volumeCount: number;
}

export interface PlatformsResponse {
  capabilities: ResourceCapabilitiesView;
  platforms: PlatformView[];
}

export interface Policies {
  capabilities: ResourceCapabilitiesView;
  policies: BackupPolicyView[];
}

export interface Pools {
  capabilities: ResourceCapabilitiesView;
  pools: AuthorizedPool[];
}

export interface ProblemDetails {
  capability?: string | null;
  detail?: string | null;
  effectiveEdition?: string | null;
  errors?: Record<string, string[]> | null;
  licenseStatus?: string | null;
  requestId?: string | null;
  /**
   * @format int32
   * @min 0
   */
  status: number;
  title: string;
  traceId?: string | null;
  type: string;
}

export interface ProfileMfaRecoveryCodesView {
  enabled: boolean;
  recoveryCodes: string[];
}

export interface ProfileMfaSetupView {
  /** @format date-time */
  expiresAt: string;
  otpAuthUri: string;
  secret: string;
}

export interface ProfileMfaStatusView {
  canDisable: boolean;
  enabled: boolean;
  policy: MfaPolicy;
  /** @format int32 */
  remainingRecoveryCodes: number;
}

export interface ProfileResourceInfo {
  /** @format uuid */
  id: string;
  name: string;
}

export interface Projects {
  capabilities: ResourceCapabilitiesView;
  projects: AuthorizedProject[];
}

export interface PrunePlatformInput {
  resource: PruneResource;
}

export interface PrunePlatformView {
  buildCacheDeleted: string[];
  imagesDeleted: string[];
  networksDeleted: string[];
  resource: PruneResource;
  /** @format int64 */
  spaceReclaimed: number;
  volumesDeleted: string[];
}

export type PublicActivityEventInfo =
  | {
      $type: "InitialAdministratorCreated";
      mode: string;
      /** @format uuid */
      userId: string;
      userName: string;
    }
  | {
      $type: "AlertRuleCreated";
      alertRule: AlertRuleActivitySnapshot;
    }
  | {
      $type: "AlertRuleUpdated";
      newRule: AlertRuleActivitySnapshot;
      oldRule: AlertRuleActivitySnapshot;
    }
  | {
      $type: "BackupPolicyCreated";
      policy: BackupPolicyActivitySnapshot;
    }
  | {
      $type: "BackupRunQueued";
      /** @format uuid */
      runId: string;
      trigger: string;
    }
  | {
      $type: "BackupRunStarted";
      /** @format uuid */
      runId: string;
      trigger: string;
    }
  | {
      $type: "BackupRunCompleted";
      /** @format int64 */
      durationMs?: number | null;
      errorMessage?: string | null;
      /** @format uuid */
      runId: string;
      status: string;
      trigger: string;
    }
  | {
      $type: "BackupPolicyRenamed";
      newName: string;
      oldName: string;
    }
  | {
      $type: "BackupPolicyUpdated";
      newPolicy: BackupPolicyActivitySnapshot;
      oldPolicy: BackupPolicyActivitySnapshot;
    }
  | {
      $type: "AlertRuleRenamed";
      newName: string;
      oldName: string;
    }
  | (VolumeContentDownloaded & {
      $type: "VolumeContentDownloaded";
    })
  | (WebhookActivityDetails & {
      $type: "GitRepoWebhookReceived";
    })
  | (WebhookActivityDetails & {
      $type: "StackWebhookReceived";
    })
  | (WebhookActivityDetails & {
      $type: "BuildWebhookReceived";
    })
  | (WebhookActivityDetails & {
      $type: "SwarmServiceWebhookReceived";
    })
  | {
      $type: "UserProfileUpdated";
      changes: ActivityChangedField[];
    }
  | {
      $type: "UserPreferencesUpdated";
      changes: ActivityChangedField[];
    }
  | {
      $type: "UserPasswordChanged";
    }
  | {
      $type: "UserSessionRevoked";
      /** @format uuid */
      sessionId: string;
    }
  | {
      $type: "UserOtherSessionsRevoked";
      /** @format int32 */
      count: number;
    }
  | {
      $type: "UserMfaEnabled";
    }
  | {
      $type: "UserMfaDisabled";
    }
  | {
      $type: "UserMfaVerificationFailed";
    }
  | {
      $type: "UserMfaRecoveryCodeUsed";
    }
  | {
      $type: "UserMfaRecoveryCodesRegenerated";
    }
  | {
      $type: "UserMfaResetByAdministrator";
      /** @format uuid */
      targetUserId: string;
    }
  | {
      $type: "UserCreated";
      user: UserActivitySnapshot;
    }
  | {
      $type: "UserUpdated";
      newUser: UserActivitySnapshot;
      oldUser: UserActivitySnapshot;
      passwordChanged: boolean;
    }
  | {
      $type: "UserRenamed";
      newName: string;
      oldName: string;
    }
  | {
      $type: "UserDeleted";
      user: UserActivitySnapshot;
    }
  | {
      $type: "TeamCreated";
      team: TeamActivitySnapshot;
    }
  | {
      $type: "TeamUpdated";
      newTeam: TeamActivitySnapshot;
      oldTeam: TeamActivitySnapshot;
    }
  | {
      $type: "TeamRenamed";
      newName: string;
      oldName: string;
    }
  | {
      $type: "TeamDeleted";
      team: TeamActivitySnapshot;
    }
  | {
      $type: "RoleCreated";
      role: RoleActivitySnapshot;
    }
  | {
      $type: "RoleUpdated";
      newRole: RoleActivitySnapshot;
      oldRole: RoleActivitySnapshot;
    }
  | {
      $type: "RoleRenamed";
      newName: string;
      oldName: string;
    }
  | {
      $type: "RoleDeleted";
      role: RoleActivitySnapshot;
    }
  | {
      $type: "ServiceAccountCreated";
      account: ServiceAccountActivitySnapshot;
    }
  | {
      $type: "ServiceAccountUpdated";
      newAccount: ServiceAccountActivitySnapshot;
      oldAccount: ServiceAccountActivitySnapshot;
    }
  | {
      $type: "ServiceAccountRenamed";
      newName: string;
      oldName: string;
    }
  | {
      $type: "ServiceAccountEnabled";
      /** @format uuid */
      accountId: string;
    }
  | {
      $type: "ServiceAccountDisabled";
      /** @format uuid */
      accountId: string;
    }
  | {
      $type: "ServiceAccountArchived";
      /** @format uuid */
      accountId: string;
    }
  | {
      $type: "ServiceAccountTokenCreated";
      /** @format uuid */
      accountId: string;
      /** @format date-time */
      expiresAtUtc?: string | null;
      publicHint: string;
      /** @format uuid */
      tokenId: string;
      tokenName: string;
    }
  | {
      $type: "ServiceAccountTokenRevoked";
      /** @format uuid */
      accountId: string;
      publicHint: string;
      /** @format uuid */
      tokenId: string;
    }
  | {
      $type: "LicenseInstalled";
      license: LicenseActivitySnapshot;
    }
  | {
      $type: "LicenseReplaced";
      newLicense: LicenseActivitySnapshot;
      oldLicense: LicenseActivitySnapshot;
    }
  | {
      $type: "LicenseRemoved";
      license: LicenseActivitySnapshot;
    }
  | {
      $type: "LicenseEnteredGracePeriod";
      license: LicenseActivitySnapshot;
    }
  | {
      $type: "LicenseExpired";
      license: LicenseActivitySnapshot;
    }
  | {
      $type: "LicenseValidationFailed";
      errorCode?: string | null;
      fingerprint?: string | null;
      status: LicenseStatus;
    }
  | {
      $type: "OidcProviderCreated";
      provider: OidcProviderActivitySnapshot;
    }
  | {
      $type: "OidcProviderUpdated";
      newProvider: OidcProviderActivitySnapshot;
      oldProvider: OidcProviderActivitySnapshot;
    }
  | {
      $type: "OidcProviderRenamed";
      newName: string;
      oldName: string;
    }
  | {
      $type: "OidcProviderDeleted";
      provider: OidcProviderActivitySnapshot;
    }
  | {
      $type: "RegistryCreated";
      registry: RegistryActivitySnapshot;
    }
  | {
      $type: "RegistryUpdated";
      newRegistry: RegistryActivitySnapshot;
      oldRegistry: RegistryActivitySnapshot;
    }
  | {
      $type: "RegistryRenamed";
      newName: string;
      oldName: string;
    }
  | {
      $type: "RegistryDeleted";
      registry: RegistryActivitySnapshot;
    }
  | {
      $type: "DeploymentCreated";
      deployment: DeploymentActivitySnapshot;
    }
  | {
      $type: "DeploymentAdopted";
      containerId: string;
      containerName: string;
      deployment: DeploymentActivitySnapshot;
    }
  | {
      $type: "DeploymentDuplicated";
      deployment: DeploymentActivitySnapshot;
      source: ActivitySourceResource;
    }
  | {
      $type: "DeploymentUpdated";
      newDeployment: DeploymentActivitySnapshot;
      oldDeployment: DeploymentActivitySnapshot;
    }
  | {
      $type: "DeploymentRenamed";
      newName: string;
      oldName: string;
    }
  | {
      $type: "DeploymentDeleted";
      deployment: DeploymentActivitySnapshot;
    }
  | {
      $type: "DeploymentStarted";
      containerIds: string[];
    }
  | {
      $type: "DeploymentStopped";
      containerIds: string[];
    }
  | {
      $type: "DeploymentPaused";
      containerIds: string[];
    }
  | {
      $type: "DeploymentDegraded";
      reason: string;
    }
  | {
      $type: "StackDegraded";
      reason: string;
    }
  | {
      $type: "DeploymentApplied";
      deployment?: null | DeploymentActivitySnapshot;
      result: DeploymentResultActivitySnapshot;
    }
  | {
      $type: "StackCreated";
      stack: StackActivitySnapshot;
    }
  | {
      $type: "StackDuplicated";
      source: ActivitySourceResource;
      stack: StackActivitySnapshot;
    }
  | {
      $type: "StackUpdated";
      newStack: StackActivitySnapshot;
      oldStack: StackActivitySnapshot;
    }
  | {
      $type: "StackRenamed";
      newName: string;
      oldName: string;
    }
  | {
      $type: "StackDeleted";
      stack: StackActivitySnapshot;
    }
  | {
      $type: "StackStarted";
      containerIds: string[];
    }
  | {
      $type: "StackStopped";
      containerIds: string[];
    }
  | {
      $type: "StackPaused";
      containerIds: string[];
    }
  | {
      $type: "StackApplied";
      result: StackResultActivitySnapshot;
      stack?: null | StackActivitySnapshot;
    }
  | {
      $type: "StackRollback";
      newStack?: null | StackActivitySnapshot;
      oldStack?: null | StackActivitySnapshot;
      result: StackResultActivitySnapshot;
    }
  | {
      $type: "StackDriftDetected";
      fingerprint: string;
      reason: string;
    }
  | {
      $type: "StackDriftResolved";
      previousFingerprint: string;
    }
  | {
      $type: "StackImported";
      projectName: string;
      serviceNames?: string[];
      stack: StackActivitySnapshot;
    }
  | {
      $type: "SwarmServiceCreated";
      service: SwarmServiceActivitySnapshot;
    }
  | {
      $type: "SwarmServiceDuplicated";
      service: SwarmServiceActivitySnapshot;
      source: ActivitySourceResource;
    }
  | {
      $type: "SwarmServiceAdopted";
      dockerServiceId: string;
      service: SwarmServiceActivitySnapshot;
    }
  | {
      $type: "SwarmServiceUpdated";
      newService: SwarmServiceActivitySnapshot;
      oldService: SwarmServiceActivitySnapshot;
    }
  | {
      $type: "SwarmServiceRenamed";
      newName: string;
      oldName: string;
    }
  | {
      $type: "SwarmServiceDeleted";
      service: SwarmServiceActivitySnapshot;
    }
  | {
      $type: "SwarmServiceApplied";
      /** @format uuid */
      operationId: string;
      warnings: string[];
    }
  | {
      $type: "SwarmServiceScaled";
      /** @format uuid */
      operationId: string;
      /** @format int32 */
      replicas: number;
      warnings: string[];
    }
  | {
      $type: "SwarmServiceForceUpdated";
      /** @format uuid */
      operationId: string;
      warnings: string[];
    }
  | {
      $type: "SwarmServiceOperationFailed";
      kind: string;
      /** @format uuid */
      operationId: string;
      reason: string;
    }
  | {
      $type: "PlatformConnected";
      platform: PlatformActivitySnapshot;
      previousStatus: string;
    }
  | {
      $type: "PlatformDisconnected";
      platform: PlatformActivitySnapshot;
      previousStatus: string;
    }
  | {
      $type: "PlatformCreated";
      platform: PlatformActivitySnapshot;
    }
  | {
      $type: "PlatformRenamed";
      newName: string;
      oldName: string;
    }
  | {
      $type: "PlatformDeleted";
      platform: PlatformActivitySnapshot;
    }
  | {
      $type: "PlatformNodeAgentLifecycle";
      kind: string;
      message: string;
      /** @format uuid */
      operationId: string;
      state: string;
    }
  | {
      $type: "GitRepoCreated";
      gitRepo: GitRepositoryActivitySnapshot;
    }
  | {
      $type: "GitRepoUpdated";
      newGitRepo: GitRepositoryActivitySnapshot;
      oldGitRepo: GitRepositoryActivitySnapshot;
    }
  | {
      $type: "GitRepoRenamed";
      newName: string;
      oldName: string;
    }
  | {
      $type: "GitRepoDeleted";
      gitRepo: GitRepositoryActivitySnapshot;
    }
  | {
      $type: "GitRepoPulled";
      gitRepo: GitRepositoryActivitySnapshot;
      result: GitRepositorySyncActivitySnapshot;
    }
  | {
      $type: "GitRepoCloned";
      gitRepo: GitRepositoryActivitySnapshot;
      result: GitRepositorySyncActivitySnapshot;
    }
  | {
      $type: "ActionCreated";
      action: AutomationActionActivitySnapshot;
    }
  | {
      $type: "ActionUpdated";
      newAction: AutomationActionActivitySnapshot;
      oldAction: AutomationActionActivitySnapshot;
    }
  | {
      $type: "ActionRenamed";
      newName: string;
      oldName: string;
    }
  | {
      $type: "ActionDeleted";
      action: AutomationActionActivitySnapshot;
    }
  | {
      $type: "ActionRunQueued";
      /** @format uuid */
      runId: string;
      trigger: string;
    }
  | {
      $type: "ActionRunStarted";
      /** @format uuid */
      runId: string;
      trigger: string;
    }
  | {
      $type: "ActionRunSucceeded";
      /** @format int64 */
      durationMs?: number | null;
      /** @format int32 */
      exitCode?: number | null;
      /** @format uuid */
      runId: string;
      trigger: string;
    }
  | {
      $type: "ActionRunFailed";
      /** @format int64 */
      durationMs?: number | null;
      errorMessage?: string | null;
      /** @format int32 */
      exitCode?: number | null;
      /** @format uuid */
      runId: string;
      trigger: string;
    }
  | {
      $type: "ActionRunTimedOut";
      /** @format int64 */
      durationMs?: number | null;
      errorMessage?: string | null;
      /** @format uuid */
      runId: string;
      trigger: string;
    }
  | {
      $type: "ActionRunCancelled";
      /** @format uuid */
      runId: string;
      trigger: string;
    }
  | {
      $type: "ActionRunRejected";
      reason: string;
      /** @format uuid */
      runId: string;
      trigger: string;
    }
  | {
      $type: "BuildRunQueued";
      /** @format uuid */
      runId: string;
      trigger: string;
    }
  | {
      $type: "BuildRunStarted";
      /** @format uuid */
      runId: string;
      trigger: string;
    }
  | {
      $type: "BuildRunSucceeded";
      /** @format int64 */
      durationMs?: number | null;
      /** @format int32 */
      exitCode?: number | null;
      imageDigest?: string | null;
      /** @format uuid */
      runId: string;
      trigger: string;
    }
  | {
      $type: "BuildRunFailed";
      /** @format int64 */
      durationMs?: number | null;
      errorMessage?: string | null;
      /** @format int32 */
      exitCode?: number | null;
      /** @format uuid */
      runId: string;
      status: string;
      trigger: string;
    }
  | {
      $type: "BuildRunTimedOut";
      /** @format int64 */
      durationMs?: number | null;
      errorMessage?: string | null;
      /** @format uuid */
      runId: string;
      trigger: string;
    }
  | {
      $type: "BuildRunCancelled";
      /** @format uuid */
      runId: string;
      trigger: string;
    }
  | {
      $type: "BuildCreated";
      build: BuildProjectActivitySnapshot;
    }
  | {
      $type: "BuildUpdated";
      newBuild: BuildProjectActivitySnapshot;
      oldBuild: BuildProjectActivitySnapshot;
    }
  | {
      $type: "BuildRenamed";
      newName: string;
      oldName: string;
    }
  | {
      $type: "BuildDeleted";
      build: BuildProjectActivitySnapshot;
    }
  | {
      $type: "BuildAgentPoolCreated";
      pool: BuildAgentPoolActivitySnapshot;
    }
  | {
      $type: "BuildAgentPoolUpdated";
      newPool: BuildAgentPoolActivitySnapshot;
      oldPool: BuildAgentPoolActivitySnapshot;
    }
  | {
      $type: "BuildAgentPoolRenamed";
      newName: string;
      oldName: string;
    }
  | {
      $type: "BuildAgentPoolDeleted";
      pool: BuildAgentPoolActivitySnapshot;
    }
  | {
      $type: "BuildAgentPoolTested";
      message: string;
      pool: BuildAgentPoolActivitySnapshot;
      status: string;
    };

export interface PullImageInput {
  imageTag: string;
  /** @format uuid */
  platformId: string;
  /** @format uuid */
  registryId: string;
}

export interface PullImageStreamItem {
  digest?: string | null;
  dockerImageId?: string | null;
  error?: null | ImagePullError;
  errorMessage?: string | null;
  from?: string | null;
  id?: string | null;
  progress?: null | PlatformsImagePullImagePullProgress;
  progressMessage?: string | null;
  status?: string | null;
  stream?: string | null;
}

export interface ReadinessResponse {
  database: boolean;
  docker: boolean;
  setup: boolean;
  status: string;
}

export interface RecreateStackOnNewCommitState {
  currentCommitSha: string;
  /** @format date-time */
  lastCheckedAt: string;
  remoteCommitSha?: string | null;
}

export interface RecreateStackOnNewImageState {
  autoUpdateStates?: ImageUpdateState[];
}

export interface RegenerateProfileMfaRecoveryCodesInput {
  code: string;
  password: string;
}

export interface RegistriesResponse {
  capabilities: ResourceCapabilitiesView;
  registries: AuthorizedRegistryView[];
}

export interface RegistryActivitySnapshot {
  configuration: any;
  description: string;
  /** @format uuid */
  id: string;
  name: string;
  registryHost: string;
  status: string;
}

export interface RegistryConfigResponse {
  configuration: RegistrySpec;
  description: string;
  /** @format uuid */
  id: string;
  name: string;
  registryHost: string;
  status: RegistryStatus;
  tags: ResourcesTagsTagSummary[];
}

/** Partial provider settings; omission preserves a value and null removes it. */
export interface RegistryConfigurationPatch {
  $type?: null | RegistryKind;
  accessKey?: string | null;
  authEnabled?: boolean | null;
  authenticationRequired?: boolean | null;
  ghcrAuthEnabled?: boolean | null;
  instanceUrl?: string | null;
  nameSpace?: string | null;
  password?: string | null;
  pat?: string | null;
  region?: string | null;
  secretAccessKey?: string | null;
  userName?: string | null;
}

export interface RegistryPatch {
  configuration?: null | RegistryConfigurationPatch;
  description?: string | null;
  name?: string | null;
  registryHost?: string | null;
  status?: null | RegistryStatus;
  tagIds?: string[] | null;
}

export type RegistrySpec =
  | {
      $type: "Custom";
      authEnabled?: boolean | null;
      password?: string | null;
      userName?: string | null;
    }
  | {
      $type: "DockerHub";
      pat?: string | null;
      userName?: string | null;
    }
  | {
      $type: "Azure";
      password: string;
      userName: string;
    }
  | {
      $type: "AWS";
      accessKey: string;
      authenticationRequired?: boolean | null;
      region: string;
      secretAccessKey: string;
    }
  | {
      $type: "Gitlab";
      instanceUrl: string;
      pat: string;
      userName: string;
    }
  | {
      $type: "GitHub";
      ghcrAuthEnabled?: boolean | null;
      nameSpace?: string | null;
      pat?: string | null;
    };

export interface RegistryView {
  /** @format date-time */
  createdAt: string;
  /** @format uuid */
  createdByActorId: string;
  description: string | null;
  /** @format uuid */
  id: string;
  name: string;
  registryHost: string;
  status: RegistryStatus;
  tags: ResourcesTagsTagSummary[];
  type: RegistryKind;
}

export interface RenameAlertRuleInput {
  /** @format uuid */
  id: string;
  name: string;
}

export interface RenameBackupPolicyInput {
  /** @format uuid */
  id: string;
  name: string;
}

export interface RenameDeploymentInput {
  /** @format uuid */
  id: string;
  name: string;
}

export interface RenameInput {
  /** @format uuid */
  id: string;
  name: string;
}

export interface RenameOidcProviderRequest {
  /** @format uuid */
  id: string;
  name: string;
}

export interface RenamePlatformInput {
  /** @format uuid */
  id: string;
  name: string;
}

export interface RenamePool {
  /** @format uuid */
  id: string;
  name: string;
}

export interface RenameResourceInput {
  /** @format uuid */
  id: string;
  name: string;
}

export interface RenameRoleRequest {
  /** @format uuid */
  id: string;
  name: string;
}

export interface RenameServiceAccountRequest {
  /** @format uuid */
  id: string;
  name: string;
}

export interface RenameStackInput {
  /** @format uuid */
  id: string;
  name: string;
}

export interface RenameSwarmServiceInput {
  /** @format uuid */
  id: string;
  name: string;
}

export interface RenameTeamRequest {
  /** @format uuid */
  id: string;
  name: string;
}

export interface RenameUserRequest {
  /** @format uuid */
  id: string;
  name: string;
}

export interface ReplaceResourceTagsInput {
  tagIds?: string[];
}

export interface RepoCommand {
  commands?: string[];
  path?: string;
}

export interface Repositories {
  capabilities: ResourceCapabilitiesView;
  repositories: BackupRepositoryView[];
}

export interface RepositoryLocationInput {
  location: BackupExecutionLocation;
  /** @format uuid */
  platformId?: string | null;
}

export interface ResolveInput {
  ids: string[];
  resolutionNote?: string | null;
}

export interface ResourceAccessInput {
  permissionLevel: PermissionLevel;
  /** @format uuid */
  resourceId: string;
  resourceType: ResourceType;
  specificPermissions?: SpecificPermission[];
}

export interface ResourceAccessView {
  /** @format uuid */
  id: string | null;
  permissionLevel: PermissionLevel;
  /** @format uuid */
  resourceId: string;
  resourceName: string | null;
  resourceType: ResourceType;
  specificPermissions: SpecificPermission[] | null;
}

export interface ResourceBindingInput {
  /** @format uuid */
  id: string;
  kind: ResourceBindingKind;
  name: string;
  secretDeliveryMode?: null | SecretDeliveryMode;
  /** @format uuid */
  secretId?: string | null;
  targetPath?: string | null;
  value?: string | null;
}

export interface ResourceBindingSnapshot {
  kind: ResourceBindingKind;
  name: string;
  scope: ResourceBindingScope;
  secretDeliveryMode?: null | SecretDeliveryMode;
  /** @format uuid */
  secretId?: string | null;
  targetPath?: string | null;
  value: string;
}

export interface ResourceBindingView {
  /** @format uuid */
  id: string;
  isInherited: boolean;
  kind: ResourceBindingKind;
  name: string;
  /** @format uuid */
  resourceId: string | null;
  scope: ResourceBindingScope;
  secretDeliveryMode: null | SecretDeliveryMode;
  /** @format uuid */
  secretId: string | null;
  targetPath: string | null;
  value: string | null;
}

export interface ResourceBindingsView {
  effectiveEntries: ResourceBindingView[];
  entries: ResourceBindingView[];
}

export interface ResourceCapabilitiesView {
  canExecute: boolean;
  canRead: boolean;
  canWrite: boolean;
}

export interface ResourceInfo {
  group: string | null;
  /** @format uuid */
  id: string;
  name: string;
}

export interface ResourceSpec {
  /** @format float */
  memoryLimit?: number | null;
  /** @format float */
  nanoCpus?: number | null;
}

export interface ResourceTagsResponse {
  tags: ResourcesTagsTagSummary[];
}

export interface RestoreInput {
  overwriteExisting: boolean;
  /** @format uuid */
  sourceBackupRunItemId?: string | null;
  targetDockerNodeId?: string | null;
  /** @format uuid */
  targetPlatformId: string;
  targetVolumeName: string;
}

export interface Restores {
  runs: BackupRestoreRunView[];
}

export interface RevokeOtherProfileSessionsView {
  /** @format int64 */
  count: number;
}

export interface RoleActivitySnapshot {
  permissions: RolePermissionActivitySnapshot[];
  roleType: string;
}

export interface RolePermissionActivitySnapshot {
  permissionLevel: PermissionLevel;
  resourceType: ResourceType;
  /** @format int32 */
  specificPermissions: number;
}

export interface RolePermissionInput {
  permissionLevel: PermissionLevel;
  resourceType: ResourceType;
  specificPermissions?: SpecificPermission[] | null;
}

export interface RolePermissionView {
  permissionLevel: PermissionLevel;
  resourceType: ResourceType;
  specificPermissions?: SpecificPermission[];
}

export interface RoleView {
  /** @format uuid */
  id: string;
  name: string;
  permissions: RolePermissionView[];
  roleType: RoleType;
}

export interface RolesResponse {
  capabilities: ResourceCapabilitiesView;
  roles: RoleView[];
}

export interface RollbackStackInput {
  /** @format uuid */
  releaseId: string;
  /** @format uuid */
  stackId: string;
}

export interface Rules {
  alertRules: AlertRuleListItem[];
  capabilities: ResourceCapabilitiesView;
}

export interface RunAsActorUsageView {
  /** @format uuid */
  id: string;
  isActive: boolean;
  name: string;
  resourceType: ResourceType;
}

export interface RunInput {
  argsJson?: any;
  code?: string | null;
  /** @format int32 */
  timeoutSeconds?: number | null;
}

export interface RunList {
  runs: AutomationRunView[];
}

export interface RuntimeConfigFrom {
  network: string;
}

export interface RuntimeIpam {
  config?: RuntimeIpamConfig[];
  driver: string;
  options?: Record<string, string>;
}

export interface RuntimeIpamConfig {
  gateway?: string | null;
  ipRange?: string | null;
  subnet?: string | null;
}

export interface RuntimeSwarmPeer {
  address: string;
  nodeId: string;
}

export interface ScaleSwarmServiceInput {
  /** @format int32 */
  replicas: number;
}

export interface SecretDefinitionView {
  /** @format date-time */
  createdAt: string;
  externalKey: string | null;
  externalPath: string | null;
  /** @format int32 */
  externalVersion: number | null;
  /** @format uuid */
  id: string;
  name: string;
  /** @format uuid */
  providerId: string | null;
  providerType: SecretProviderType;
}

export interface SecretDefinitionsResponse {
  capabilities: ResourceCapabilitiesView;
  secrets: SecretDefinitionView[];
}

export interface SecretProviderInput {
  address: string;
  mountPath: string;
  name: string;
  token: string;
}

export interface SecretProviderPatch {
  address?: string | null;
  mountPath?: string | null;
  name?: string | null;
  token?: string | null;
}

export interface SecretProviderView {
  address: string;
  /** @format date-time */
  createdAt: string;
  /** @format uuid */
  id: string;
  mountPath: string;
  name: string;
  providerType: SecretProviderType;
}

export interface SecretProvidersResponse {
  providers: SecretProviderView[];
}

export interface SecretTestResult {
  message: string;
  success: boolean;
}

export interface SelfManagedVmBuildAgentPoolProviderSpec {
  architecture?: CpuArchitecture;
  connectionMode?: BuildAgentPoolConnectionMode;
  endpoint?: string | null;
  labels?: string[] | null;
  /** @format int32 */
  maxWorkers?: number;
  /** @format uuid */
  registrationSecretId?: string | null;
}

export interface ServiceAccountActivitySnapshot {
  description?: string | null;
  /** @format uuid */
  id: string;
  isEnabled: boolean;
  name: string;
  resourceAccesses: ServiceAccountResourceAccessSnapshot[];
  roleIds: string[];
  teamIds: string[];
}

export interface ServiceAccountCapabilities {
  canExecute: boolean;
  canManageCredentials: boolean;
  canRead: boolean;
  canUse: boolean;
  canWrite: boolean;
}

export type ServiceAccountDetailResponse = ServiceAccountView & {
  capabilities: ServiceAccountCapabilities;
};

export interface ServiceAccountLimitsView {
  /** @format int64 */
  defaultTokenLifetimeDays: number;
  /** @format int64 */
  maximumActiveTokensPerAccount: number;
  /** @format int64 */
  maximumTokenLifetimeDays: number;
}

export interface ServiceAccountResourceAccess {
  /** @format uuid */
  id?: string | null;
  permissionLevel: PermissionLevel;
  /** @format uuid */
  resourceId: string;
  resourceName?: string | null;
  resourceType: ResourceType;
  specificPermissions?: SpecificPermission[];
}

export interface ServiceAccountResourceAccessSnapshot {
  permissionLevel: PermissionLevel;
  /** @format uuid */
  resourceId: string;
  resourceType: ResourceType;
  /** @format int32 */
  specificPermissions: number;
}

export interface ServiceAccountTokenView {
  /** @format date-time */
  createdAtUtc: string;
  createdByActorId: ActorId;
  createdByName: string;
  /** @format date-time */
  expiresAtUtc: string | null;
  hint: string;
  /** @format uuid */
  id: string;
  /** @format date-time */
  lastUsedAtUtc: string | null;
  name: string;
  /** @format date-time */
  revokedAtUtc: string | null;
  revokedByActorId: null | ActorId;
}

export interface ServiceAccountTokensResponse {
  pagedResult: PagedResultServiceAccountTokenView;
}

export interface ServiceAccountView {
  /** @format int64 */
  activeTokenCount: number;
  actorId: ActorId;
  /** @format date-time */
  archivedAtUtc: string | null;
  /** @format date-time */
  createdAt: string;
  createdByActorId: ActorId;
  description: string | null;
  /** @format uuid */
  id: string;
  isEnabled: boolean;
  /** @format date-time */
  lastUsedAtUtc: string | null;
  name: string;
  resourceAccesses: ServiceAccountResourceAccess[];
  roles: ResourceInfo[];
  teams: ResourceInfo[];
  /** @format date-time */
  updatedAt: string;
}

export interface ServiceAccountsResponse {
  capabilities: ResourceCapabilitiesView;
  pagedResult: PagedResultServiceAccountView;
}

export interface ServiceInspectionView {
  configIds: string[];
  /** @format date-time */
  createdAt?: string | null;
  /** @format int32 */
  desiredTaskCount: number;
  id: string;
  image: string;
  labels: Record<string, string>;
  mode: string;
  name: string;
  networkIds: string[];
  ports: string[];
  /** @format int32 */
  runningTaskCount: number;
  secretIds: string[];
  updateMessage?: string | null;
  updateState: string;
  /** @format date-time */
  updatedAt?: string | null;
  /**
   * @format int64
   * @min 0
   */
  versionIndex: number;
}

export interface ServiceMetadataInput {
  description?: string | null;
}

export interface ServiceStatistics {
  complete: boolean;
  dockerServiceId: string;
  /** @min 0 */
  expectedTasks: number;
  missingDockerNodeIds: string[];
  /** @format date-time */
  newestSampleAt?: string | null;
  observedContainerProjectionIds: string[];
  /** @min 0 */
  observedTasks: number;
  /** @format date-time */
  oldestSampleAt?: string | null;
  stats: ContainerStatSnapshot[];
}

export interface SetupStatusView {
  /** @min 0 */
  passwordMaximumLength: number;
  /** @min 0 */
  passwordMinimumLength: number;
  requiresSetup: boolean;
}

export interface StackActivitySnapshot {
  description?: string | null;
  driftPolicy: any;
  /** @format uuid */
  id: string;
  name: string;
  stackRelease?: any;
  stackSource: string;
}

export interface StackAdoptionIssue {
  code: string;
  fieldPath: string | null;
  message: string;
  severity: string;
}

export interface StackBuildImageBinding {
  /** @format date-time */
  appliedAt?: string | null;
  /** @format uuid */
  appliedBuildRunId?: string | null;
  appliedDigest?: string | null;
  appliedImageReference?: string | null;
  /** @format uuid */
  buildProjectId: string;
  redeployOnBuild?: boolean;
  /** @format uuid */
  resolvedBuildRunId?: string | null;
  resolvedDigest?: string | null;
  resolvedImageReference?: string | null;
  serviceName: string;
}

export interface StackCapabilities {
  canApply: boolean;
  canDelete: boolean;
  canExecute: boolean;
  canInspect: boolean;
  canOpenTerminal: boolean;
  canPull: boolean;
  canRead: boolean;
  canViewLogs: boolean;
  canViewReleases: boolean;
  canViewResourceBindings: boolean;
  canWrite: boolean;
}

export interface StackCommand {
  commands?: string[];
  path?: string;
}

export interface StackConfigView {
  description: string | null;
  driftPolicy: StackDriftPolicy;
  /** @format uuid */
  id: string;
  name: string;
  /** @format uuid */
  platformId: string;
  platformType: PlatformType;
  /** @format int64 */
  rowVersion: number;
  spec: StackSpec;
  stackSource: StackSource;
  stackUpdateState: StackUpdateState;
}

export type StackDrift =
  | {
      $type: "MissingContainer";
      serviceName: string;
    }
  | {
      $type: "ExtraContainer";
      containerId: string;
      serviceName: string;
    }
  | {
      $type: "ContainerStopped";
      containerId: string;
      serviceName: string;
    }
  | {
      $type: "ContainerPaused";
      containerId: string;
      serviceName: string;
    }
  | {
      $type: "ContainerUnhealthy";
      containerId: string;
      healthStatus?: string | null;
      serviceName: string;
    }
  | {
      $type: "ImageMismatch";
      actualImage: string;
      expectedImage: string;
      serviceName: string;
    }
  | {
      $type: "ConfigHashMismatch";
      actualHash?: string | null;
      expectedHash?: string | null;
      serviceName: string;
    };

export interface StackDriftPolicy {
  alertOnDrift: boolean;
  autoResumePausedContainers: boolean;
  autoStartStoppedContainers: boolean;
  markDegraded: boolean;
  mode: StackDriftMode;
  removeExtraContainers: boolean;
}

export interface StackDriftPolicyInput {
  alertOnDrift?: boolean | null;
  autoResumePausedContainers?: boolean | null;
  autoStartStoppedContainers?: boolean | null;
  markDegraded?: boolean | null;
  mode?: null | StackDriftMode;
  removeExtraContainers?: boolean | null;
}

export interface StackDriftReport {
  drifts: StackDrift[];
  hasAutoFixableDrift: boolean;
  hasDrift: boolean;
  hasStructuralDrift: boolean;
  /** @format uuid */
  platformId: string;
  /** @format uuid */
  stackId: string;
}

export interface StackDuplicateDraftView {
  draft: CreateStackInput;
  warnings: DuplicateDraftWarning[];
}

export interface StackHistory {
  containers: ContainerHistory[];
}

export interface StackReconciliationAction {
  action: StackReconciliationActionType;
  containerId: string;
  errorMessage: string | null;
  serviceName: string;
  succeeded: boolean;
}

export interface StackReconciliationResult {
  actions: StackReconciliationAction[];
  afterReport: null | StackDriftReport;
  beforeReport: StackDriftReport;
  /** @format uuid */
  stackId: string;
  status: StackReconciliationStatus;
}

export interface StackReleaseSource {
  branch?: string | null;
  composeDigest?: string | null;
  composeEnvFilesFromRepo?: string[] | null;
  composePaths?: string[];
  envFilePaths?: string[];
  /** @format uuid */
  gitRepositoryId?: string | null;
  gitRepositoryName?: string | null;
  gitRepositoryUrl?: string | null;
  requestedCommitSha?: string | null;
  resolvedCommitSha: string;
  sourceType: StackSource;
  watchPaths?: string[] | null;
  workingDirectory?: string | null;
}

export interface StackReleaseView {
  actorName: string;
  actorType: ActorType;
  /** @format date-time */
  createdAt: string;
  /** @format uuid */
  createdByActorId: string;
  /** @format uuid */
  id: string;
  /** @format uuid */
  platformId: string;
  platformName?: string | null;
  platformStatus: PlatformStatus;
  resourceBindings?: ResourceBindingSnapshot[] | null;
  source?: null | StackReleaseSource;
  spec: StackSpec;
  /** @format uuid */
  stackId: string;
  status: StackReleaseStatus;
  version: string;
}

export interface StackReleasesView {
  releases: StackReleaseView[];
}

export interface StackResultActivitySnapshot {
  containerIds?: string[] | null;
  message?: string | null;
  resourceBindings?: any;
}

export type StackSpec =
  | (StackSpecCommon & {
      composeFile: string;
      updateBehavior?: StackUpdateBehavior;
    } & {
      $type: "WebEditor";
    })
  | (StackSpecCommon & {
      additionalEnvFileFromRepo?: string[];
      branch: string;
      commitSha?: string | null;
      composeEnvFilesFromRepo?: string[];
      composePaths?: string[];
      /** @format uuid */
      gitRepoId: string;
      updateBehavior?: StackUpdateBehavior;
      watchPaths?: string[];
      webhook?: null | StackWebhookConfig;
      workingDirectory?: string | null;
    } & {
      $type: "Git";
    });

export interface StackSpecCommon {
  buildImageBindings?: StackBuildImageBinding[];
  destroyBeforeDeploy?: boolean;
  envFilePath?: string | null;
  postDeploy?: null | StackCommand;
  preDeploy?: null | StackCommand;
  projectName?: string | null;
  /** @format uuid */
  registryId?: string | null;
}

export interface StackStreamItem {
  /** @format int32 */
  exitCode?: number | null;
  message?: string | null;
  progressMessage?: string | null;
  severity?: string | null;
  stackStatus?: null | StackReleaseStatus;
  type: StackApplyEventType;
}

export type StackUpdateState =
  | {
      $type: "WebEditor";
      recreateStackOnNewImageState: RecreateStackOnNewImageState;
    }
  | {
      $type: "Git";
      recreateStackOnNewCommitState: RecreateStackOnNewCommitState;
      recreateStackOnNewImageState: RecreateStackOnNewImageState;
    };

export interface StackView {
  capabilities: null | StackCapabilities;
  controlState: ResourceControlState;
  /** @format date-time */
  createdAt: string;
  /** @format uuid */
  createdByActorId: string;
  /** @format uuid */
  currentStackReleaseId: string;
  description: string | null;
  driftPolicy: StackDriftPolicy;
  /** @format uuid */
  id: string;
  latestActivityView: null | LatestActivityView;
  name: string;
  /** @format uuid */
  platformId: string | null;
  platformName: string | null;
  platformStatus: PlatformStatus;
  platformType: PlatformType;
  resourceBindings: ResourceBindingSnapshot[] | null;
  /** @format int64 */
  rowVersion: number;
  source: null | StackReleaseSource;
  spec: null | StackSpec;
  stackSource: StackSource;
  stackUpdateState: StackUpdateState;
  status: StackReleaseStatus;
  tags: ResourcesTagsTagSummary[];
  version: string | null;
}

export type StackWebhookConfig = WebhookConfig & {
  forceDeploy?: boolean;
};

export interface StacksView {
  capabilities: ResourceCapabilitiesView;
  stacks: StackView[];
}

export interface StartProfileMfaSetupInput {
  password: string;
}

export interface SwarmConfigView {
  capabilities: null | PlatformCapabilitiesView;
  /** @format date-time */
  createdAt: string | null;
  id: string;
  inUse: boolean;
  isStale: boolean;
  labels: Record<string, string>;
  name: string;
  /** @format date-time */
  observedAt: string;
  serviceNames: string[];
  templatingDriver: string | null;
  /** @format date-time */
  updatedAt: string | null;
  /** @format int64 */
  versionIndex: number;
}

export interface SwarmItemsResponseSwarmConfigView {
  capabilities: PlatformCapabilitiesView;
  items: {
    capabilities: null | PlatformCapabilitiesView;
    /** @format date-time */
    createdAt: string | null;
    id: string;
    inUse: boolean;
    isStale: boolean;
    labels: Record<string, string>;
    name: string;
    /** @format date-time */
    observedAt: string;
    serviceNames: string[];
    templatingDriver: string | null;
    /** @format date-time */
    updatedAt: string | null;
    /** @format int64 */
    versionIndex: number;
  }[];
}

export interface SwarmItemsResponseSwarmNetworkView {
  capabilities: PlatformCapabilitiesView;
  items: {
    capabilities: null | PlatformCapabilitiesView;
    /** @format date-time */
    createdAt: string | null;
    driver: string;
    enableIPv6: boolean;
    id: string;
    isAttachable: boolean;
    isEncrypted: boolean;
    isIngress: boolean;
    isInternal: boolean;
    isStale: boolean;
    labels: Record<string, string>;
    name: string;
    /** @format date-time */
    observedAt: string;
    scope: string;
    serviceNames: string[];
    subnets: string[];
  }[];
}

export interface SwarmItemsResponseSwarmNodeView {
  capabilities: PlatformCapabilitiesView;
  items: {
    address: string;
    architecture: string;
    availability: string;
    capabilities: null | PlatformCapabilitiesView;
    /** @format date-time */
    createdAt: string | null;
    /** @format int32 */
    desiredTaskCount: number;
    engineVersion: string;
    hostname: string;
    id: string;
    isLeader: boolean;
    isStale: boolean;
    labels: Record<string, string>;
    /** @format date-time */
    observedAt: string;
    operatingSystem: string;
    reachability: string;
    role: string;
    /** @format int32 */
    runningTaskCount: number;
    status: string;
    statusMessage: string | null;
    /** @format date-time */
    updatedAt: string | null;
    /** @format int64 */
    versionIndex: number;
  }[];
}

export interface SwarmItemsResponseSwarmSecretView {
  capabilities: PlatformCapabilitiesView;
  items: {
    capabilities: null | PlatformCapabilitiesView;
    /** @format date-time */
    createdAt: string | null;
    driver: string | null;
    id: string;
    inUse: boolean;
    isStale: boolean;
    labels: Record<string, string>;
    name: string;
    /** @format date-time */
    observedAt: string;
    serviceNames: string[];
    /** @format date-time */
    updatedAt: string | null;
    /** @format int64 */
    versionIndex: number;
  }[];
}

export interface SwarmItemsResponseSwarmServiceView {
  capabilities: PlatformCapabilitiesView;
  items: {
    capabilities: null | PlatformCapabilitiesView;
    configIds: string[];
    /** @format date-time */
    createdAt: string | null;
    /** @format int32 */
    desiredTaskCount: number;
    dockerStackNamespace: string | null;
    id: string;
    image: string;
    isStale: boolean;
    labels: Record<string, string>;
    mode: string;
    name: string;
    networkIds: string[];
    /** @format date-time */
    observedAt: string;
    ownership: SwarmServiceOwnership;
    ownershipDiagnostic: string | null;
    ports: string[];
    /** @format int32 */
    runningTaskCount: number;
    secretIds: string[];
    /** @format uuid */
    stackId: string | null;
    /** @format uuid */
    swarmServiceId: string | null;
    updateMessage: string | null;
    updateState: string;
    /** @format date-time */
    updatedAt: string | null;
    /** @format int64 */
    versionIndex: number;
  }[];
}

export interface SwarmItemsResponseSwarmTaskView {
  capabilities: PlatformCapabilitiesView;
  items: {
    capabilities: null | PlatformCapabilitiesView;
    /** @format date-time */
    createdAt: string | null;
    desiredState: string;
    error: string | null;
    id: string;
    image: string;
    isStale: boolean;
    name: string;
    nodeHostname: string;
    nodeId: string;
    /** @format date-time */
    observedAt: string;
    ports: string[];
    serviceId: string;
    serviceName: string;
    /** @format int32 */
    slot: number | null;
    state: string;
    statusMessage: string | null;
    /** @format date-time */
    statusTimestamp: string | null;
    /** @format date-time */
    updatedAt: string | null;
    /** @format int64 */
    versionIndex: number;
  }[];
}

export interface SwarmNetworkView {
  capabilities: null | PlatformCapabilitiesView;
  /** @format date-time */
  createdAt: string | null;
  driver: string;
  enableIPv6: boolean;
  id: string;
  isAttachable: boolean;
  isEncrypted: boolean;
  isIngress: boolean;
  isInternal: boolean;
  isStale: boolean;
  labels: Record<string, string>;
  name: string;
  /** @format date-time */
  observedAt: string;
  scope: string;
  serviceNames: string[];
  subnets: string[];
}

export interface SwarmNodeAgentCoverage {
  agentImageDigest?: string | null;
  agentImageReference?: string | null;
  canManageNodeAgents: boolean;
  /** @min 0 */
  connectedNodes: number;
  /** @min 0 */
  coveredNodes: number;
  /** @min 0 */
  eligibleNodes: number;
  /** @min 0 */
  enrollingNodes: number;
  /** @format date-time */
  enrollmentExpiresAtUtc?: string | null;
  /** @min 0 */
  incompatibleNodes: number;
  isInstalled: boolean;
  /** @format date-time */
  lastMembershipReconciliationAtUtc?: string | null;
  /** @min 0 */
  missingNodes: number;
  nodes: NodeAgentCoverage[];
  /** @min 0 */
  offlineNodes: number;
  operation?: null | NodeAgentOperation;
  reasons: string[];
  /** @min 0 */
  staleNodes: number;
  state: string;
  /** @min 0 */
  totalNodes: number;
  /** @min 0 */
  unschedulableNodes: number;
  /** @min 0 */
  unsupportedNodes: number;
}

export interface SwarmNodeAvailabilityTarget {
  nodeId: string;
  /** @format int64 */
  versionIndex: number;
}

export interface SwarmNodeView {
  address: string;
  architecture: string;
  availability: string;
  capabilities: null | PlatformCapabilitiesView;
  /** @format date-time */
  createdAt: string | null;
  /** @format int32 */
  desiredTaskCount: number;
  engineVersion: string;
  hostname: string;
  id: string;
  isLeader: boolean;
  isStale: boolean;
  labels: Record<string, string>;
  /** @format date-time */
  observedAt: string;
  operatingSystem: string;
  reachability: string;
  role: string;
  /** @format int32 */
  runningTaskCount: number;
  status: string;
  statusMessage: string | null;
  /** @format date-time */
  updatedAt: string | null;
  /** @format int64 */
  versionIndex: number;
}

export interface SwarmOverviewView {
  capabilities: PlatformCapabilitiesView;
  /** @format int64 */
  desiredTaskCount: number;
  health: string;
  /** @format int64 */
  imageCount: number;
  isStale: boolean;
  /** @format int64 */
  localNetworkCount: number;
  /** @format int64 */
  managerCount: number;
  message: string | null;
  /** @format int64 */
  networkCount: number;
  /** @format int64 */
  nodeCount: number;
  /** @format uuid */
  platformId: string;
  quorum: SwarmQuorumView;
  /** @format int64 */
  runningTaskCount: number;
  /** @format int64 */
  serviceCount: number;
  serviceStatusCounts: WorkloadStatusCounts;
  /** @format int64 */
  volumeCount: number;
}

export interface SwarmPreflightInput {
  driftPolicy?: null | StackDriftPolicy;
  name: string;
  /** @format uuid */
  platformId: string;
  spec: StackSpec;
  stackSource: StackSource;
}

export interface SwarmQuorumView {
  hasLeader: boolean;
  /** @format int64 */
  reachableManagers: number;
  /** @format int64 */
  requiredManagers: number;
  state: SwarmQuorumState;
}

export interface SwarmSecretView {
  capabilities: null | PlatformCapabilitiesView;
  /** @format date-time */
  createdAt: string | null;
  driver: string | null;
  id: string;
  inUse: boolean;
  isStale: boolean;
  labels: Record<string, string>;
  name: string;
  /** @format date-time */
  observedAt: string;
  serviceNames: string[];
  /** @format date-time */
  updatedAt: string | null;
  /** @format int64 */
  versionIndex: number;
}

export interface SwarmServiceActivitySnapshot {
  description?: string | null;
  dockerName: string;
  dockerServiceId?: string | null;
  /** @format uuid */
  id: string;
  name: string;
  /** @format uuid */
  platformId: string;
  spec: any;
}

export interface SwarmServiceAdoptionDraftView {
  draft: CreateSwarmServiceInput;
  issues: SwarmServiceAdoptionIssue[];
  previewFingerprint: string;
  source: SwarmServiceAdoptionSource;
}

export interface SwarmServiceAdoptionIssue {
  code: string;
  message: string;
}

export interface SwarmServiceAdoptionSource {
  dockerServiceId: string;
  name: string;
  /** @format uuid */
  platformId: string;
  platformName: string;
}

export interface SwarmServiceCapabilities {
  canApply: boolean;
  canExecute: boolean;
  canInspect: boolean;
  canRead: boolean;
  canViewLogs: boolean;
  canViewResourceBindings: boolean;
  canWrite: boolean;
}

export interface SwarmServiceConfigReference {
  configId: string;
  configName: string;
  targetName: string;
}

export interface SwarmServiceDuplicateDraftView {
  draft: CreateSwarmServiceInput;
  warnings: string[];
}

export interface SwarmServiceHealthCheck {
  /** @format int64 */
  intervalNanoseconds?: number | null;
  /** @format int32 */
  retries?: number | null;
  /** @format int64 */
  startPeriodNanoseconds?: number | null;
  test: string[];
  /** @format int64 */
  timeoutNanoseconds?: number | null;
}

export type SwarmServiceImageInfo =
  | {
      $type: "External";
      imageTag: string;
      /** @format uuid */
      registryId: string;
      resolvedDigest?: string | null;
    }
  | {
      $type: "Build";
      /** @format uuid */
      buildProjectId: string;
      /** @format uuid */
      resolvedBuildRunId?: string | null;
      resolvedDigest?: string | null;
      resolvedImageReference?: string | null;
    };

export interface SwarmServiceMount {
  kind: MountKind;
  readOnly?: boolean;
  source: string;
  target: string;
}

export interface SwarmServiceOperationView {
  /** @format date-time */
  attemptedAt: string | null;
  /** @format date-time */
  completedAt: string | null;
  /** @format uuid */
  id: string;
  kind: SwarmServiceOperationKind;
  /** @format date-time */
  preparedAt: string;
  resultCode: string | null;
  resultMessage: string | null;
  state: SwarmServiceOperationState;
  warnings: string[];
}

export interface SwarmServicePort {
  protocol?: string;
  publishMode?: PortPublishMode;
  /** @format int32 */
  publishedPort?: number | null;
  /** @format int32 */
  targetPort: number;
}

export interface SwarmServiceProgressItem {
  errorMessage: string | null;
  isCompleted?: boolean;
  isWarning?: boolean;
  message: string;
  /** @format uuid */
  operationId: string | null;
  /** @format uuid */
  serviceId: string;
  stage: string;
}

export interface SwarmServiceResources {
  /** @format int64 */
  limitMemoryBytes?: number | null;
  /** @format int64 */
  limitNanoCpus?: number | null;
  /** @format int64 */
  reservationMemoryBytes?: number | null;
  /** @format int64 */
  reservationNanoCpus?: number | null;
}

export interface SwarmServiceRestartPolicy {
  condition?: RestartCondition;
  /** @format int64 */
  delayNanoseconds?: number | null;
  /** @format int32 */
  maximumAttempts?: number | null;
  /** @format int64 */
  windowNanoseconds?: number | null;
}

export interface SwarmServiceSecretReference {
  secretId: string;
  secretName: string;
  targetName: string;
}

export interface SwarmServiceSpec {
  arguments?: string[];
  command?: string[];
  configs?: SwarmServiceConfigReference[];
  environment?: string[];
  healthCheck?: null | SwarmServiceHealthCheck;
  image: SwarmServiceImageInfo;
  labels?: Record<string, string>;
  mounts?: SwarmServiceMount[];
  networkIds?: string[];
  placementConstraints?: string[];
  ports?: SwarmServicePort[];
  /** @format int32 */
  replicas?: number | null;
  resources?: null | SwarmServiceResources;
  restartPolicy?: null | SwarmServiceRestartPolicy;
  schedulingMode?: SchedulingMode;
  secrets?: SwarmServiceSecretReference[];
  /** @format int64 */
  stopGracePeriodNanoseconds?: number | null;
  updateBehavior?: UpdateBehavior;
  updatePolicy?: null | SwarmServiceUpdatePolicy;
  user?: string | null;
  webhook?: null | WebhookConfig;
  workingDirectory?: string | null;
}

export interface SwarmServiceUpdatePolicy {
  /** @format int64 */
  delayNanoseconds?: number | null;
  failureAction?: UpdateFailureAction;
  order?: UpdateOrder;
  /** @format int32 */
  parallelism?: number;
}

export interface SwarmServiceView {
  capabilities: null | PlatformCapabilitiesView;
  configIds: string[];
  /** @format date-time */
  createdAt: string | null;
  /** @format int32 */
  desiredTaskCount: number;
  dockerStackNamespace: string | null;
  id: string;
  image: string;
  isStale: boolean;
  labels: Record<string, string>;
  mode: string;
  name: string;
  networkIds: string[];
  /** @format date-time */
  observedAt: string;
  ownership: SwarmServiceOwnership;
  ownershipDiagnostic: string | null;
  ports: string[];
  /** @format int32 */
  runningTaskCount: number;
  secretIds: string[];
  /** @format uuid */
  stackId: string | null;
  /** @format uuid */
  swarmServiceId: string | null;
  updateMessage: string | null;
  updateState: string;
  /** @format date-time */
  updatedAt: string | null;
  /** @format int64 */
  versionIndex: number;
}

export interface SwarmStackCompatibilityIssue {
  code: string;
  fieldPath?: string | null;
  message: string;
  severity: SwarmStackCompatibilitySeverity;
}

export interface SwarmStackCompatibilityReport {
  isCompatible: boolean;
  issues: SwarmStackCompatibilityIssue[];
}

export interface SwarmTaskView {
  capabilities: null | PlatformCapabilitiesView;
  /** @format date-time */
  createdAt: string | null;
  desiredState: string;
  error: string | null;
  id: string;
  image: string;
  isStale: boolean;
  name: string;
  nodeHostname: string;
  nodeId: string;
  /** @format date-time */
  observedAt: string;
  ports: string[];
  serviceId: string;
  serviceName: string;
  /** @format int32 */
  slot: number | null;
  state: string;
  statusMessage: string | null;
  /** @format date-time */
  statusTimestamp: string | null;
  /** @format date-time */
  updatedAt: string | null;
  /** @format int64 */
  versionIndex: number;
}

export interface TagPatch {
  color?: string | null;
  name?: string | null;
}

export interface TagView {
  color: string;
  /** @format date-time */
  createdAt: string;
  /** @format uuid */
  createdByActorId: string;
  /** @format uuid */
  id: string;
  name: string;
  normalizedName: string;
  /** @format date-time */
  updatedAt: string;
  /** @format int32 */
  usageCount: number;
}

export interface TagsResponse {
  capabilities: ResourceCapabilitiesView;
  tags: AuthorizedTagView[];
}

export interface TaskHistory {
  /** @format uuid */
  containerProjectionId: string;
  dockerContainerId: string;
  stats: ContainerStatView[];
}

export interface TaskTerminalView {
  dockerContainerId: string;
}

export interface TeamActivitySnapshot {
  isEnabled: boolean;
  memberActorIds: string[];
  resourceAccesses: IdentityResourceAccessSnapshot[];
  roleIds: string[];
}

export interface TeamMemberView {
  actorId: ActorId;
  name: string;
  principalType: ActorType;
  /** @format uuid */
  resourceId: string;
}

export interface TeamSearchItemView {
  /** @format uuid */
  id: string;
  name: string;
}

export interface TeamView {
  actorId: ActorId;
  /** @format uuid */
  id: string;
  isEnabled: boolean;
  members: TeamMemberView[] | null;
  name: string;
  resourceAccesses: ResourceAccessView[] | null;
  roles: ResourceInfo[] | null;
  /** @format int32 */
  totalMembers: number;
  users: ResourceInfo[] | null;
}

export interface TeamsResponse {
  capabilities: ResourceCapabilitiesView;
  pagedResult: PagedResultTeamView;
}

export interface TestExternalSecretInput {
  externalKey: string;
  externalPath: string;
  /** @format int32 */
  externalVersion?: number | null;
  /** @format uuid */
  providerId: string;
}

export interface TestOidcDiscoveryRequest {
  issuer?: string | null;
  /** @format uuid */
  providerId?: string | null;
}

export interface TestSecretProviderInput {
  address: string;
  mountPath: string;
  name?: string | null;
  /** @format uuid */
  providerId?: string | null;
  token?: string | null;
}

export interface UpdateAutomationActionInput {
  alertOnFailure?: boolean;
  code?: string;
  defaultArgsJson?: string | null;
  description?: string | null;
  enabled?: boolean;
  /** @format uuid */
  runAsActorId?: string | null;
  scheduleCron?: string | null;
  scheduleEnabled?: boolean;
  scheduleTimeZone?: string | null;
  /** @format int32 */
  timeoutSeconds?: number | null;
  webhook?: null | WebhookPatch;
}

export interface UpdateAutomationActionMetadata {
  description?: string | null;
}

export interface UpdateBackupPolicyInput {
  alertOnFailure?: boolean | null;
  /** @format uuid */
  backupRepositoryId?: string | null;
  cron?: string | null;
  description?: string | null;
  enabled?: boolean | null;
  /** @format int32 */
  keepLastSuccessful?: number | null;
  /** @format uuid */
  runAsActorId?: string | null;
  source?: null | BackupSourceSpec;
  timeZone?: string | null;
  /** @format int32 */
  timeoutSeconds?: number | null;
  webhook?: null | WebhookPatch;
}

export interface UpdateBackupRepositoryInput {
  description?: string | null;
  spec?: null | BackupRepositorySpec;
}

export interface UpdateBuildAgentPoolInput {
  /** @format int32 */
  cleanupTimeoutSeconds?: number | null;
  description?: string | null;
  enabled?: boolean | null;
  /** @format int32 */
  failureRetentionMinutes?: number | null;
  /** @format int32 */
  heartbeatTimeoutSeconds?: number | null;
  /** @format int32 */
  maxActiveBuilders?: number | null;
  /** @format int32 */
  maximumInstanceLifetimeSeconds?: number | null;
  providerSpec?: null | BuildAgentPoolProviderSpec;
  /** @format int32 */
  provisioningTimeoutSeconds?: number | null;
  /** @format int32 */
  queueTimeoutSeconds?: number | null;
  /** @format int32 */
  registrationTimeoutSeconds?: number | null;
}

export interface UpdateBuildProjectInput {
  branch?: string | null;
  /** @format uuid */
  buildAgentPoolId?: string | null;
  buildArgs?: BuildArgSpec[] | null;
  buildSecrets?: BuildSecretSpec[] | null;
  builderKind?: BuildProjectBuilderKind;
  contextPath?: string | null;
  description?: string | null;
  dockerfilePath?: string | null;
  enabled?: boolean;
  /** @format uuid */
  gitRepositoryId?: string;
  imageRepository?: string;
  /** @format uuid */
  platformId?: string | null;
  /** @format uuid */
  registryId?: string;
  /** @format int32 */
  retentionRunCount?: number | null;
  tagTemplates?: string[] | null;
  target?: string | null;
  /** @format int32 */
  timeoutSeconds?: number | null;
  webhook?: null | WebhookPatch;
}

export interface UpdateCurrentProfileRequest {
  displayName: string;
}

export interface UpdateServiceAccountRequest {
  description?: string | null;
  isEnabled?: boolean | null;
}

export interface UpdateSwarmNodeInput {
  availability: string;
  labels?: Record<string, string>;
  /** @format int64 */
  versionIndex: number;
}

export interface UpdateSwarmNodesAvailabilityInput {
  availability: string;
  nodes?: SwarmNodeAvailabilityTarget[];
}

export interface UpdateSwarmResourceLabelsInput {
  labels?: Record<string, string>;
  /** @format int64 */
  versionIndex: number;
}

export interface UpdateSwarmServiceInput {
  /** @format int64 */
  rowVersion: number;
  spec: SwarmServiceSpec;
}

export interface UserActivitySnapshot {
  email: string;
  isEnabled: boolean;
  resourceAccesses: IdentityResourceAccessSnapshot[];
  roleIds: string[];
  teamIds: string[];
}

export interface UserPreferencesView {
  contentLayout: UserContentLayout;
  dateTimeFormat: UserDateTimeFormat;
  density: UserUiDensity;
  font: UserUiFont;
  isPersisted: boolean;
  radius: UserUiRadius;
  theme: UserTheme;
  themeColor: UserThemeColor;
  timeZone: string | null;
}

export interface UserSearchItemView {
  email: string;
  /** @format uuid */
  id: string;
  name: string;
}

export interface UserSessionSummaryView {
  /** @format date-time */
  createdAt: string;
  displayName: string;
  /** @format date-time */
  expiresAt: string;
  /** @format uuid */
  id: string;
  ipAddress: string | null;
  isCurrent: boolean;
  /** @format date-time */
  lastSeenAt: string;
  userAgent: string | null;
}

export interface UserSessionsView {
  canRevokeOtherSessions: boolean;
  sessions: UserSessionSummaryView[];
}

export interface UserView {
  actorId: ActorId;
  email: string;
  /** @format uuid */
  id: string;
  isEnabled: boolean;
  name: string;
  resourceAccesses: ResourceAccessView[] | null;
  roles: ResourceInfo[] | null;
  teams: ResourceInfo[] | null;
}

export interface UsersResponse {
  capabilities: ResourceCapabilitiesView;
  pagedResult: PagedResultUserView;
}

export interface ValidateImportRequest {
  importKind?: null | StackImportKind;
  name: string;
  spec: StackSpec;
  stackSource: StackSource;
}

export interface VerifyInput {
  alertDestination: AlertDestination;
  name: string;
  url: string;
}

export interface VolumeCapabilitiesView {
  canBrowse: boolean;
  canDownload: boolean;
  canExecute: boolean;
  canInspect: boolean;
  canRead: boolean;
  canWrite: boolean;
}

export interface VolumeContainerView {
  id: string;
  image: string;
  imageId: string;
  name: string;
  networks: Record<string, string>;
  ports: Record<string, HostPortBinding[]>;
  state: ContainerStateStatus;
}

export interface VolumeContentDownloaded {
  fileName: string;
  isDirectory: boolean;
  path: string;
  volumeName: string;
}

export interface VolumeDirectory {
  entries: VolumeFileEntry[];
  isTruncated: boolean;
  path: string;
  /** @format uuid */
  platformId: string;
  volumeName: string;
}

export interface VolumeFileEntry {
  linkTarget?: string | null;
  /** @format date-time */
  modifiedAt?: string | null;
  name: string;
  path: string;
  /**
   * @format int64
   * @min 0
   */
  size?: number | null;
  type: VolumeEntryType;
}

export interface VolumeUsageDataView {
  /** @format int64 */
  refCount: number;
  /** @format int64 */
  size: number;
}

export interface VolumeView {
  backupCoverage?: null | BackupCoverageView;
  capabilities: null | VolumeCapabilitiesView;
  clusterVolume: any;
  containers: VolumeContainerView[];
  createdAt: string;
  dockerNodeId: string | null;
  driver: string;
  id: string;
  inUse: boolean;
  isStale: boolean;
  labels: Record<string, string>;
  mountpoint: string;
  name: string;
  nodeHostname: string | null;
  options: Record<string, string>;
  scope: string;
  staleReason: string | null;
  status: Record<string, string>;
  usageData: null | VolumeUsageDataView;
}

export interface VolumesResponse {
  capabilities: ResourceCapabilitiesView;
  volumes: VolumeView[];
}

/**
 * Safe webhook audit metadata. Authentication headers, credentials and request
 * bodies are deliberately not representable in the persisted event.
 */
export type WebhookActivityDetails = WebhookActivitySource & {
  authType: string;
  dispatchedBranch?: string | null;
  dispatchedCommitSha?: string | null;
  execution: string;
  reason?: string | null;
  /** @format uuid */
  requestId: string;
  status: string;
};

export interface WebhookActivitySource {
  branch?: string | null;
  commitSha?: string | null;
  deliveryId?: string | null;
  eventType?: string | null;
  repositoryFullName?: string | null;
}

/** Shared webhook wire configuration and defaults. */
export interface WebhookConfig {
  /** @default "GitHubHmacSha256" */
  authScheme?: WebhookAuthScheme;
  /** @default null */
  branchFilter?: string | null;
  /** @default false */
  enabled?: boolean;
  /** @default "GitHub" */
  provider?: WebhookProvider;
  /** @default null */
  secret?: string | null;
}

/** Partial configuration. Defaults belong to creation, never to omitted patch fields. */
export interface WebhookPatch {
  authScheme?: WebhookAuthScheme;
  branchFilter?: string | null;
  enabled?: boolean;
  provider?: WebhookProvider;
  secret?: string | null;
}

export interface WebhookResponse {
  accepted: boolean;
  reason: string | null;
  /** @format uuid */
  requestId: string;
  status: string;
}

export interface WorkloadStatusCounts {
  /** @format int64 */
  degraded: number;
  /** @format int64 */
  failed: number;
  /** @format int64 */
  healthy: number;
  /** @format int64 */
  inProgress: number;
  /** @format int64 */
  paused: number;
  /** @format int64 */
  stopped: number;
  /** @format int64 */
  total: number;
  /** @format int64 */
  unknown: number;
}

export interface DeploymentsModelImagePullProgress {
  /** @format int64 */
  current?: number | null;
  /** @format int64 */
  start?: number | null;
  /** @format int64 */
  total?: number | null;
  units?: string | null;
}

export interface PlatformsImagePullImagePullProgress {
  /** @format int64 */
  current?: number | null;
  /** @format int64 */
  start?: number | null;
  /** @format int64 */
  total?: number | null;
  units?: string | null;
}

export interface ResourcesTagsTagSummary {
  color: string;
  /** @format uuid */
  id: string;
  name: string;
}

export interface ServerAlertsHttpEvents {
  pagedResult: AlertEventPage;
}

export interface ServerBackupsHttpEvents {
  events: string[];
  /** @format uuid */
  runId: string;
}

export interface ServerBackupsHttpLogs {
  logs: string;
  /** @format uuid */
  runId: string;
}

export interface ServerBackupsHttpQueueInput {
  trigger?: null | BackupQueueTrigger;
}

export interface ServerBackupsHttpRuns {
  runs: BackupRunView[];
}

export interface ServerBuildsHttpLogs {
  logs: BuildLogEntry[];
  /** @format uuid */
  runId: string;
}

export interface ServerBuildsHttpQueueInput {
  trigger?: null | QueueBuildTrigger;
}

export interface ServerBuildsHttpRuns {
  runs: BuildRunView[];
}

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
 * @title Citadel API
 * @version 1.0.0
 */
export class Api<
  SecurityDataType extends unknown,
> extends HttpClient<SecurityDataType> {
  api = {
    /**
     * No description
     *
     * @tags Activities
     * @name ListActivities
     * @summary List authorized activities
     * @request GET:/api/v1/activities
     * @secure
     * @response `200` `ActivitiesView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listActivities: (
      query?: {
        /** @format uuid */
        ResourceId?: string;
        ResourceType?: ActivityResourceType;
        EventType?: ActivityEventType;
        /**
         * @format int32
         * @min 1
         * @default 1
         */
        Page?: number;
        /**
         * @format int32
         * @min 1
         * @max 500
         * @default 50
         */
        PageSize?: number;
      },
      params: RequestParams = {},
    ) =>
      this.request<ActivitiesView, ProblemDetails>({
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
     * @tags Activities
     * @name GetActivity
     * @summary Get an authorized activity
     * @request GET:/api/v1/activities/{id}
     * @secure
     * @response `200` `ActivityView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getActivity: (id: string, params: RequestParams = {}) =>
      this.request<ActivityView, ProblemDetails>({
        path: `/api/v1/activities/${id}`,
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
     * @summary Get an Actor
     * @request GET:/api/v1/actors/{id}
     * @secure
     * @response `200` `ActorView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getActor: (id: string, params: RequestParams = {}) =>
      this.request<ActorView, ProblemDetails>({
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
     * @summary Enable or disable an Actor
     * @request PATCH:/api/v1/actors/{id}/enabled
     * @secure
     * @response `200` `ActorView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    patchActorEnabled: (
      id: string,
      data: PatchActorEnabledInput,
      params: RequestParams = {},
    ) =>
      this.request<ActorView, ProblemDetails>({
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
     * @tags AlertEvents
     * @name ListAlertEvents
     * @summary List Alert Events
     * @request GET:/api/v1/alertEvents
     * @secure
     * @response `200` `ServerAlertsHttpEvents` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listAlertEvents: (
      query?: {
        /** @format uuid */
        ResourceId?: string;
        AlertType?: string;
        ResourceType?: string;
        UnresolvedOnly?: boolean;
        /**
         * @format int32
         * @min 1
         * @default 1
         */
        Page?: number;
        /**
         * @format int32
         * @min 1
         * @max 1000
         * @default 50
         */
        PageSize?: number;
      },
      params: RequestParams = {},
    ) =>
      this.request<ServerAlertsHttpEvents, ProblemDetails>({
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
     * @name AcknowledgeAlertEvents
     * @summary Acknowledge Alert Events
     * @request POST:/api/v1/alertEvents/acknowledge
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    acknowledgeAlertEvents: (data: Ids, params: RequestParams = {}) =>
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
     * @summary Resolve Alert Events
     * @request POST:/api/v1/alertEvents/resolve
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    resolveAlertEvents: (data: ResolveInput, params: RequestParams = {}) =>
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
     * @tags AlertEvents
     * @name GetUnresolvedAlertEventsCount
     * @summary Count unresolved Alert Events
     * @request GET:/api/v1/alertEvents/unresolved-count
     * @secure
     * @response `200` `Count` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getUnresolvedAlertEventsCount: (params: RequestParams = {}) =>
      this.request<Count, ProblemDetails>({
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
     * @name GetAlertEvent
     * @summary Get an Alert Event
     * @request GET:/api/v1/alertEvents/{id}
     * @secure
     * @response `200` `AlertEventView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getAlertEvent: (id: string, params: RequestParams = {}) =>
      this.request<AlertEventView, ProblemDetails>({
        path: `/api/v1/alertEvents/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AlertRules
     * @name DeleteAlertRules
     * @summary Delete Alert Rules
     * @request DELETE:/api/v1/alertRules
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    deleteAlertRules: (data: Ids, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @name ListAlertRules
     * @summary List Alert Rules
     * @request GET:/api/v1/alertRules
     * @secure
     * @response `200` `Rules` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listAlertRules: (params: RequestParams = {}) =>
      this.request<Rules, ProblemDetails>({
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
     * @summary Create an Alert Rule
     * @request POST:/api/v1/alertRules
     * @secure
     * @response `200` `AlertRuleView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createAlertRule: (data: AlertRuleInput, params: RequestParams = {}) =>
      this.request<AlertRuleView, ProblemDetails>({
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
     * @name DeleteAlertChannels
     * @summary Delete Alert Channels
     * @request DELETE:/api/v1/alertRules/channels
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    deleteAlertChannels: (data: Ids, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @name ListAlertChannels
     * @summary List Alert Channels
     * @request GET:/api/v1/alertRules/channels
     * @secure
     * @response `200` `Channels` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listAlertChannels: (params: RequestParams = {}) =>
      this.request<Channels, ProblemDetails>({
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
     * @summary Create an Alert Channel
     * @request POST:/api/v1/alertRules/channels
     * @secure
     * @response `200` `AlertChannelView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createAlertChannel: (data: AlertChannelInput, params: RequestParams = {}) =>
      this.request<AlertChannelView, ProblemDetails>({
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
     * @name VerifyAlertChannel
     * @summary Verify an Alert Channel
     * @request POST:/api/v1/alertRules/channels/verify
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    verifyAlertChannel: (data: VerifyInput, params: RequestParams = {}) =>
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
     * @tags AlertRules
     * @name GetAlertChannel
     * @summary Get an Alert Channel
     * @request GET:/api/v1/alertRules/channels/{id}
     * @secure
     * @response `200` `AlertChannelView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getAlertChannel: (id: string, params: RequestParams = {}) =>
      this.request<AlertChannelView, ProblemDetails>({
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
     * @summary Update an Alert Channel
     * @request PATCH:/api/v1/alertRules/channels/{id}
     * @secure
     * @response `200` `AlertChannelView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateAlertChannel: (
      id: string,
      data: PatchAlertChannelInput,
      params: RequestParams = {},
    ) =>
      this.request<AlertChannelView, ProblemDetails>({
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
     * @name RenameAlertRule
     * @summary Rename an Alert Rule
     * @request POST:/api/v1/alertRules/rename
     * @secure
     * @response `200` `AlertRuleView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    renameAlertRule: (data: RenameAlertRuleInput, params: RequestParams = {}) =>
      this.request<AlertRuleView, ProblemDetails>({
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
     * @name GetAlertRule
     * @summary Get an Alert Rule
     * @request GET:/api/v1/alertRules/{id}
     * @secure
     * @response `200` `AlertRuleView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getAlertRule: (id: string, params: RequestParams = {}) =>
      this.request<AlertRuleView, ProblemDetails>({
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
     * @summary Update an Alert Rule
     * @request PATCH:/api/v1/alertRules/{id}
     * @secure
     * @response `200` `AlertRuleView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateAlertRule: (
      id: string,
      data: PatchAlertRuleInput,
      params: RequestParams = {},
    ) =>
      this.request<AlertRuleView, ProblemDetails>({
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
     * @summary Get Alert Rule configuration
     * @request GET:/api/v1/alertRules/{id}/_cfg
     * @secure
     * @response `200` `AlertRuleConfig` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getAlertRuleConfig: (id: string, params: RequestParams = {}) =>
      this.request<AlertRuleConfig, ProblemDetails>({
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
     * @name UpdateAlertRuleMetadata
     * @summary Update Alert Rule metadata
     * @request PATCH:/api/v1/alertRules/{id}/_metadata
     * @secure
     * @response `200` `AlertRuleView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateAlertRuleMetadata: (
      id: string,
      data: PatchAlertRuleMetadata,
      params: RequestParams = {},
    ) =>
      this.request<AlertRuleView, ProblemDetails>({
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
     * @tags Application
     * @name GetApplicationInfo
     * @summary Get application information
     * @request GET:/api/v1/application/info
     * @secure
     * @response `200` `ApplicationInfoView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @tags Authentication
     * @name Login
     * @summary Sign in
     * @request POST:/api/v1/authentication/login
     * @response `200` `LoginResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    login: (data: LoginRequest, params: RequestParams = {}) =>
      this.request<LoginResponse, ProblemDetails>({
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
     * @name Logout
     * @summary End browser session
     * @request POST:/api/v1/authentication/logout
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    logout: (params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/authentication/logout`,
        method: "POST",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Authentication
     * @name GetAuthenticationMfaSetup
     * @summary Get mandatory MFA setup
     * @request GET:/api/v1/authentication/mfa/setup
     * @response `200` `MandatoryMfaSetupView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getAuthenticationMfaSetup: (params: RequestParams = {}) =>
      this.request<MandatoryMfaSetupView, ProblemDetails>({
        path: `/api/v1/authentication/mfa/setup`,
        method: "GET",
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Authentication
     * @name ConfirmAuthenticationMfaSetup
     * @summary Complete mandatory MFA setup
     * @request POST:/api/v1/authentication/mfa/setup/confirm
     * @response `200` `MandatoryMfaSetupCompleteView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    confirmAuthenticationMfaSetup: (
      data: ConfirmMandatoryMfaSetupInput,
      params: RequestParams = {},
    ) =>
      this.request<MandatoryMfaSetupCompleteView, ProblemDetails>({
        path: `/api/v1/authentication/mfa/setup/confirm`,
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
     * @summary Complete an MFA challenge
     * @request POST:/api/v1/authentication/mfa/verify
     * @response `200` `MfaVerificationView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    verifyAuthenticationMfa: (
      data: MfaVerificationInput,
      params: RequestParams = {},
    ) =>
      this.request<MfaVerificationView, ProblemDetails>({
        path: `/api/v1/authentication/mfa/verify`,
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
     * @name ListOidcLoginProviders
     * @summary List enabled OIDC login providers
     * @request GET:/api/v1/authentication/oidc/providers
     * @response `200` `OidcLoginProvidersView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @name CompleteOidcLogin
     * @summary Complete OIDC login
     * @request GET:/api/v1/authentication/oidc/{id}/callback
     * @response `302` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
      this.request<any, void | ProblemDetails>({
        path: `/api/v1/authentication/oidc/${id}/callback`,
        method: "GET",
        query: query,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Authentication
     * @name BeginOidcLogin
     * @summary Begin OIDC login
     * @request GET:/api/v1/authentication/oidc/{id}/login
     * @response `302` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @name RefreshToken
     * @summary Refresh browser access
     * @request GET:/api/v1/authentication/refresh
     * @response `200` `AccessTokenResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    refreshToken: (params: RequestParams = {}) =>
      this.request<AccessTokenResponse, ProblemDetails>({
        path: `/api/v1/authentication/refresh`,
        method: "GET",
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AutomationActions
     * @name ListAutomationActions
     * @summary List Automation Actions
     * @request GET:/api/v1/automation/actions
     * @secure
     * @response `200` `ActionList` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listAutomationActions: (
      query?: {
        tags?: string[];
      },
      params: RequestParams = {},
    ) =>
      this.request<ActionList, ProblemDetails>({
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
     * @summary Create an Automation Action
     * @request POST:/api/v1/automation/actions
     * @secure
     * @response `200` `AuthorizedAction` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createAutomationAction: (
      data: AutomationActionInput,
      params: RequestParams = {},
    ) =>
      this.request<AuthorizedAction, ProblemDetails>({
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
     * @name RenameAutomationAction
     * @summary Rename an Automation Action
     * @request POST:/api/v1/automation/actions/rename
     * @secure
     * @response `200` `AuthorizedAction` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    renameAutomationAction: (data: RenameInput, params: RequestParams = {}) =>
      this.request<AuthorizedAction, ProblemDetails>({
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
     * @name DeleteAutomationAction
     * @summary Delete an Automation Action
     * @request DELETE:/api/v1/automation/actions/{id}
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    deleteAutomationAction: (id: string, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/automation/actions/${id}`,
        method: "DELETE",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags AutomationActions
     * @name GetAutomationAction
     * @summary Get an Automation Action
     * @request GET:/api/v1/automation/actions/{id}
     * @secure
     * @response `200` `AuthorizedAction` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getAutomationAction: (id: string, params: RequestParams = {}) =>
      this.request<AuthorizedAction, ProblemDetails>({
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
     * @summary Update an Automation Action
     * @request PATCH:/api/v1/automation/actions/{id}
     * @secure
     * @response `200` `AuthorizedAction` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateAutomationAction: (
      id: string,
      data: UpdateAutomationActionInput,
      params: RequestParams = {},
    ) =>
      this.request<AuthorizedAction, ProblemDetails>({
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
     * @name UpdateAutomationActionMetadata
     * @summary Update Automation Action metadata
     * @request PATCH:/api/v1/automation/actions/{id}/_metadata
     * @secure
     * @response `200` `AuthorizedAction` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateAutomationActionMetadata: (
      id: string,
      data: UpdateAutomationActionMetadata,
      params: RequestParams = {},
    ) =>
      this.request<AuthorizedAction, ProblemDetails>({
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
     * @summary Queue an Automation Action run
     * @request POST:/api/v1/automation/actions/{id}/run
     * @secure
     * @response `200` `(AutomationProgress)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    runAutomationAction: (
      id: string,
      data: null | RunInput,
      params: RequestParams = {},
    ) =>
      this.request<AutomationProgress[], ProblemDetails>({
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
     * @name ListAutomationActionRuns
     * @summary List Automation Action runs
     * @request GET:/api/v1/automation/actions/{id}/runs
     * @secure
     * @response `200` `RunList` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listAutomationActionRuns: (
      id: string,
      query?: {
        /**
         * @format int32
         * @min 1
         * @max 100
         * @default 20
         */
        limit?: number;
      },
      params: RequestParams = {},
    ) =>
      this.request<RunList, ProblemDetails>({
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
     * @summary Get an Automation Action run
     * @request GET:/api/v1/automation/actions/{id}/runs/{runId}
     * @secure
     * @response `200` `AutomationRunView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getAutomationActionRun: (
      id: string,
      runId: string,
      params: RequestParams = {},
    ) =>
      this.request<AutomationRunView, ProblemDetails>({
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
     * @name CancelAutomationActionRun
     * @summary Cancel an Automation Action run
     * @request POST:/api/v1/automation/actions/{id}/runs/{runId}/cancel
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    cancelAutomationActionRun: (
      id: string,
      runId: string,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/automation/actions/${id}/runs/${runId}/cancel`,
        method: "POST",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags AutomationActions
     * @name GetAutomationActionRunLogs
     * @summary Get Automation Action run logs
     * @request GET:/api/v1/automation/actions/{id}/runs/{runId}/logs
     * @secure
     * @response `200` `AutomationActionRunLogsView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getAutomationActionRunLogs: (
      id: string,
      runId: string,
      params: RequestParams = {},
    ) =>
      this.request<AutomationActionRunLogsView, ProblemDetails>({
        path: `/api/v1/automation/actions/${id}/runs/${runId}/logs`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags AutomationActions
     * @name GetAutomationActionTags
     * @summary Get AutomationAction tags
     * @request GET:/api/v1/automation/actions/{id}/tags
     * @secure
     * @response `200` `ResourceTagsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getAutomationActionTags: (id: string, params: RequestParams = {}) =>
      this.request<ResourceTagsResponse, ProblemDetails>({
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
     * @summary Replace AutomationAction tags
     * @request PUT:/api/v1/automation/actions/{id}/tags
     * @secure
     * @response `200` `ResourceTagsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    replaceAutomationActionTags: (
      id: string,
      data: ReplaceResourceTagsInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceTagsResponse, ProblemDetails>({
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
     * @name TestAutomationAction
     * @summary Queue a test Automation Action run
     * @request POST:/api/v1/automation/actions/{id}/test
     * @secure
     * @response `200` `(AutomationProgress)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    testAutomationAction: (
      id: string,
      data: null | RunInput,
      params: RequestParams = {},
    ) =>
      this.request<AutomationProgress[], ProblemDetails>({
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
     * @tags BackupPolicies
     * @name ListBackupPolicies
     * @summary List Backup Policies
     * @request GET:/api/v1/backupPolicies
     * @secure
     * @response `200` `Policies` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listBackupPolicies: (
      query?: {
        tags?: string[];
      },
      params: RequestParams = {},
    ) =>
      this.request<Policies, ProblemDetails>({
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
     * @summary Create a Backup Policy
     * @request POST:/api/v1/backupPolicies
     * @secure
     * @response `200` `BackupPolicyView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createBackupPolicy: (data: BackupPolicyInput, params: RequestParams = {}) =>
      this.request<BackupPolicyView, ProblemDetails>({
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
     * @name GetPlatformBackupSummaries
     * @summary Get Platform Backup summaries
     * @request GET:/api/v1/backupPolicies/platform-summaries
     * @secure
     * @response `200` `PlatformBackupSummaries` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getPlatformBackupSummaries: (
      query: {
        platformIds: string[];
      },
      params: RequestParams = {},
    ) =>
      this.request<PlatformBackupSummaries, ProblemDetails>({
        path: `/api/v1/backupPolicies/platform-summaries`,
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
     * @name RenameBackupPolicy
     * @summary Rename a Backup Policy
     * @request POST:/api/v1/backupPolicies/rename
     * @secure
     * @response `200` `BackupPolicyView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    renameBackupPolicy: (
      data: RenameBackupPolicyInput,
      params: RequestParams = {},
    ) =>
      this.request<BackupPolicyView, ProblemDetails>({
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
     * @name ArchiveBackupPolicy
     * @summary Archive a Backup Policy
     * @request DELETE:/api/v1/backupPolicies/{id}
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    archiveBackupPolicy: (id: string, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/backupPolicies/${id}`,
        method: "DELETE",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupPolicies
     * @name GetBackupPolicy
     * @summary Get a Backup Policy
     * @request GET:/api/v1/backupPolicies/{id}
     * @secure
     * @response `200` `BackupPolicyView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @summary Update a Backup Policy
     * @request PATCH:/api/v1/backupPolicies/{id}
     * @secure
     * @response `200` `BackupPolicyView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateBackupPolicy: (
      id: string,
      data: UpdateBackupPolicyInput,
      params: RequestParams = {},
    ) =>
      this.request<BackupPolicyView, ProblemDetails>({
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
     * @name UpdateBackupPolicyMetadata
     * @summary Update Backup Policy metadata
     * @request PATCH:/api/v1/backupPolicies/{id}/_metadata
     * @secure
     * @response `200` `BackupPolicyView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateBackupPolicyMetadata: (
      id: string,
      data: BackupMetadataPatch,
      params: RequestParams = {},
    ) =>
      this.request<BackupPolicyView, ProblemDetails>({
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
     * @name RunBackupPolicy
     * @summary Run a Backup Policy with progress
     * @request POST:/api/v1/backupPolicies/{id}/run
     * @secure
     * @response `200` `(BackupRunStreamItem)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    runBackupPolicy: (
      id: string,
      data: ServerBackupsHttpQueueInput,
      params: RequestParams = {},
    ) =>
      this.request<BackupRunStreamItem[], ProblemDetails>({
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
     * @tags BackupPolicies
     * @name QueueBackupRun
     * @summary Queue a Backup Run
     * @request POST:/api/v1/backupPolicies/{id}/runs
     * @secure
     * @response `200` `BackupRunView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    queueBackupRun: (
      id: string,
      data: null | ServerBackupsHttpQueueInput,
      params: RequestParams = {},
    ) =>
      this.request<BackupRunView, ProblemDetails>({
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
     * @name GetBackupPolicyTags
     * @summary Get BackupPolicy tags
     * @request GET:/api/v1/backupPolicies/{id}/tags
     * @secure
     * @response `200` `ResourceTagsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getBackupPolicyTags: (id: string, params: RequestParams = {}) =>
      this.request<ResourceTagsResponse, ProblemDetails>({
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
     * @summary Replace BackupPolicy tags
     * @request PUT:/api/v1/backupPolicies/{id}/tags
     * @secure
     * @response `200` `ResourceTagsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    replaceBackupPolicyTags: (
      id: string,
      data: ReplaceResourceTagsInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceTagsResponse, ProblemDetails>({
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
     * @tags BackupRepositories
     * @name ListBackupRepositories
     * @summary List Backup Repositories
     * @request GET:/api/v1/backupRepositories
     * @secure
     * @response `200` `Repositories` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listBackupRepositories: (params: RequestParams = {}) =>
      this.request<Repositories, ProblemDetails>({
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
     * @summary Create a Backup Repository
     * @request POST:/api/v1/backupRepositories
     * @secure
     * @response `200` `BackupRepositoryView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createBackupRepository: (
      data: BackupRepositoryInput,
      params: RequestParams = {},
    ) =>
      this.request<BackupRepositoryView, ProblemDetails>({
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
     * @name ArchiveBackupRepository
     * @summary Archive a Backup Repository
     * @request DELETE:/api/v1/backupRepositories/{id}
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    archiveBackupRepository: (id: string, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/backupRepositories/${id}`,
        method: "DELETE",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRepositories
     * @name GetBackupRepository
     * @summary Get a Backup Repository
     * @request GET:/api/v1/backupRepositories/{id}
     * @secure
     * @response `200` `BackupRepositoryView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @summary Update a Backup Repository
     * @request PATCH:/api/v1/backupRepositories/{id}
     * @secure
     * @response `200` `BackupRepositoryView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateBackupRepository: (
      id: string,
      data: UpdateBackupRepositoryInput,
      params: RequestParams = {},
    ) =>
      this.request<BackupRepositoryView, ProblemDetails>({
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
     * @name CheckBackupRepository
     * @summary Check a Backup Repository
     * @request POST:/api/v1/backupRepositories/{id}/check
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    checkBackupRepository: (
      id: string,
      data: RepositoryLocationInput,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
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
     * @name InitializeBackupRepository
     * @summary Initialize a Backup Repository
     * @request POST:/api/v1/backupRepositories/{id}/initialize
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    initializeBackupRepository: (
      id: string,
      data: RepositoryLocationInput,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
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
     * @name PruneBackupRepository
     * @summary Prune a Backup Repository
     * @request POST:/api/v1/backupRepositories/{id}/prune
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    pruneBackupRepository: (
      id: string,
      data: RepositoryLocationInput,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
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
     * @tags BackupRepositories
     * @name ValidateBackupRepository
     * @summary Validate a Backup Repository
     * @request POST:/api/v1/backupRepositories/{id}/validate
     * @secure
     * @response `200` `BackupRepositoryValidationView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    validateBackupRepository: (
      id: string,
      data: RepositoryLocationInput,
      params: RequestParams = {},
    ) =>
      this.request<BackupRepositoryValidationView, ProblemDetails>({
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
     * @tags BackupRestoreRuns
     * @name ListBackupRestoreRuns
     * @summary List Backup Restore Runs
     * @request GET:/api/v1/backupRestoreRuns
     * @secure
     * @response `200` `Restores` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listBackupRestoreRuns: (
      query?: {
        /** @format uuid */
        backupRunId?: string;
        /** @format uuid */
        policyId?: string;
        /**
         * @format int32
         * @min 1
         * @max 100
         * @default 50
         */
        limit?: number;
      },
      params: RequestParams = {},
    ) =>
      this.request<Restores, ProblemDetails>({
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
     * @summary Get a Backup Restore Run
     * @request GET:/api/v1/backupRestoreRuns/{id}
     * @secure
     * @response `200` `BackupRestoreRunView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @name CancelBackupRestoreRun
     * @summary Cancel a Backup Restore Run
     * @request POST:/api/v1/backupRestoreRuns/{id}/cancel
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    cancelBackupRestoreRun: (id: string, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/backupRestoreRuns/${id}/cancel`,
        method: "POST",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRestoreRuns
     * @name GetBackupRestoreRunEvents
     * @summary Get Backup Restore Run events
     * @request GET:/api/v1/backupRestoreRuns/{id}/events
     * @secure
     * @response `200` `ServerBackupsHttpEvents` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getBackupRestoreRunEvents: (id: string, params: RequestParams = {}) =>
      this.request<ServerBackupsHttpEvents, ProblemDetails>({
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
     * @name GetBackupRestoreRunLogs
     * @summary Get Backup Restore Run logs
     * @request GET:/api/v1/backupRestoreRuns/{id}/logs
     * @secure
     * @response `200` `ServerBackupsHttpLogs` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getBackupRestoreRunLogs: (id: string, params: RequestParams = {}) =>
      this.request<ServerBackupsHttpLogs, ProblemDetails>({
        path: `/api/v1/backupRestoreRuns/${id}/logs`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRuns
     * @name ListBackupRuns
     * @summary List Backup Runs
     * @request GET:/api/v1/backupRuns
     * @secure
     * @response `200` `ServerBackupsHttpRuns` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listBackupRuns: (
      query?: {
        /** @format uuid */
        policyId?: string;
        /**
         * @format int32
         * @min 1
         * @max 100
         * @default 50
         */
        limit?: number;
      },
      params: RequestParams = {},
    ) =>
      this.request<ServerBackupsHttpRuns, ProblemDetails>({
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
     * @summary Get a Backup Run
     * @request GET:/api/v1/backupRuns/{id}
     * @secure
     * @response `200` `BackupRunView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @name CancelBackupRun
     * @summary Cancel a Backup Run
     * @request POST:/api/v1/backupRuns/{id}/cancel
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    cancelBackupRun: (id: string, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/backupRuns/${id}/cancel`,
        method: "POST",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags BackupRuns
     * @name GetBackupRunEvents
     * @summary Get Backup Run events
     * @request GET:/api/v1/backupRuns/{id}/events
     * @secure
     * @response `200` `ServerBackupsHttpEvents` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getBackupRunEvents: (id: string, params: RequestParams = {}) =>
      this.request<ServerBackupsHttpEvents, ProblemDetails>({
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
     * @name GetBackupRunLogs
     * @summary Get Backup Run logs
     * @request GET:/api/v1/backupRuns/{id}/logs
     * @secure
     * @response `200` `ServerBackupsHttpLogs` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getBackupRunLogs: (id: string, params: RequestParams = {}) =>
      this.request<ServerBackupsHttpLogs, ProblemDetails>({
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
     * @name RestoreBackupVolume
     * @summary Queue a Volume restore
     * @request POST:/api/v1/backupRuns/{id}/restoreVolume
     * @secure
     * @response `200` `BackupRestoreRunView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    restoreBackupVolume: (
      id: string,
      data: RestoreInput,
      params: RequestParams = {},
    ) =>
      this.request<BackupRestoreRunView, ProblemDetails>({
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
     * @summary Restore a Backup Volume with progress
     * @request POST:/api/v1/backupRuns/{id}/restoreVolume/run
     * @secure
     * @response `200` `(BackupRestoreRunStreamItem)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    runBackupRestoreVolume: (
      id: string,
      data: RestoreInput,
      params: RequestParams = {},
    ) =>
      this.request<BackupRestoreRunStreamItem[], ProblemDetails>({
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
     * @tags BuildAgentPools
     * @name ListBuildAgentPools
     * @summary List Build Agent Pools
     * @request GET:/api/v1/buildAgentPools
     * @secure
     * @response `200` `Pools` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listBuildAgentPools: (
      query?: {
        tags?: string[];
      },
      params: RequestParams = {},
    ) =>
      this.request<Pools, ProblemDetails>({
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
     * @summary Create a Build Agent Pool
     * @request POST:/api/v1/buildAgentPools
     * @secure
     * @response `200` `AuthorizedPool` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createBuildAgentPool: (
      data: BuildAgentPoolInput,
      params: RequestParams = {},
    ) =>
      this.request<AuthorizedPool, ProblemDetails>({
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
     * @name RenameBuildAgentPool
     * @summary Rename a Build Agent Pool
     * @request POST:/api/v1/buildAgentPools/rename
     * @secure
     * @response `200` `AuthorizedPool` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    renameBuildAgentPool: (data: RenamePool, params: RequestParams = {}) =>
      this.request<AuthorizedPool, ProblemDetails>({
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
     * @name ArchiveBuildAgentPool
     * @summary Archive a Build Agent Pool
     * @request DELETE:/api/v1/buildAgentPools/{id}
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    archiveBuildAgentPool: (id: string, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/buildAgentPools/${id}`,
        method: "DELETE",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildAgentPools
     * @name GetBuildAgentPool
     * @summary Get a Build Agent Pool
     * @request GET:/api/v1/buildAgentPools/{id}
     * @secure
     * @response `200` `AuthorizedPool` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getBuildAgentPool: (id: string, params: RequestParams = {}) =>
      this.request<AuthorizedPool, ProblemDetails>({
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
     * @summary Update a Build Agent Pool
     * @request PATCH:/api/v1/buildAgentPools/{id}
     * @secure
     * @response `200` `AuthorizedPool` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateBuildAgentPool: (
      id: string,
      data: UpdateBuildAgentPoolInput,
      params: RequestParams = {},
    ) =>
      this.request<AuthorizedPool, ProblemDetails>({
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
     * @name UpdateBuildAgentPoolMetadata
     * @summary Update Build Agent Pool metadata
     * @request PATCH:/api/v1/buildAgentPools/{id}/_metadata
     * @secure
     * @response `200` `AuthorizedPool` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateBuildAgentPoolMetadata: (
      id: string,
      data: BuildMetadataPatch,
      params: RequestParams = {},
    ) =>
      this.request<AuthorizedPool, ProblemDetails>({
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
     * @name CreateBuildAgentPoolEdgeEnrollment
     * @summary Create build pool Edge Agent enrollment
     * @request POST:/api/v1/buildAgentPools/{id}/edge/enrollments
     * @secure
     * @response `200` `EdgeEnrollmentView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createBuildAgentPoolEdgeEnrollment: (
      id: string,
      params: RequestParams = {},
    ) =>
      this.request<EdgeEnrollmentView, ProblemDetails>({
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
     * @name RevokeBuildAgentPoolEdgeAgent
     * @summary Revoke build pool Edge Agent
     * @request POST:/api/v1/buildAgentPools/{id}/edge/revoke
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @tags BuildAgentPools
     * @name GetBuildAgentPoolEdgeStatus
     * @summary Get build pool Edge Agent status
     * @request GET:/api/v1/buildAgentPools/{id}/edge/status
     * @secure
     * @response `200` `EdgeStatusView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getBuildAgentPoolEdgeStatus: (id: string, params: RequestParams = {}) =>
      this.request<EdgeStatusView, ProblemDetails>({
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
     * @name GetBuildAgentPoolTags
     * @summary Get BuildAgentPool tags
     * @request GET:/api/v1/buildAgentPools/{id}/tags
     * @secure
     * @response `200` `ResourceTagsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getBuildAgentPoolTags: (id: string, params: RequestParams = {}) =>
      this.request<ResourceTagsResponse, ProblemDetails>({
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
     * @summary Replace BuildAgentPool tags
     * @request PUT:/api/v1/buildAgentPools/{id}/tags
     * @secure
     * @response `200` `ResourceTagsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    replaceBuildAgentPoolTags: (
      id: string,
      data: ReplaceResourceTagsInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceTagsResponse, ProblemDetails>({
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
     * @name TestBuildAgentPool
     * @summary Test a Build Agent Pool
     * @request POST:/api/v1/buildAgentPools/{id}/test
     * @secure
     * @response `200` `AuthorizedPool` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    testBuildAgentPool: (id: string, params: RequestParams = {}) =>
      this.request<AuthorizedPool, ProblemDetails>({
        path: `/api/v1/buildAgentPools/${id}/test`,
        method: "POST",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildProjects
     * @name ListBuildProjects
     * @summary List Build Projects
     * @request GET:/api/v1/buildProjects
     * @secure
     * @response `200` `Projects` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listBuildProjects: (
      query?: {
        tags?: string[];
      },
      params: RequestParams = {},
    ) =>
      this.request<Projects, ProblemDetails>({
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
     * @summary Create a Build Project
     * @request POST:/api/v1/buildProjects
     * @secure
     * @response `200` `AuthorizedProject` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createBuildProject: (data: BuildProjectInput, params: RequestParams = {}) =>
      this.request<AuthorizedProject, ProblemDetails>({
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
     * @name RenameBuild
     * @summary Update Build Project
     * @request POST:/api/v1/buildProjects/rename
     * @secure
     * @response `200` `AuthorizedProject` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    renameBuild: (data: RenamePool, params: RequestParams = {}) =>
      this.request<AuthorizedProject, ProblemDetails>({
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
     * @name ArchiveBuildProject
     * @summary Archive a Build Project
     * @request DELETE:/api/v1/buildProjects/{id}
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    archiveBuildProject: (id: string, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/buildProjects/${id}`,
        method: "DELETE",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildProjects
     * @name GetBuildProject
     * @summary Get a Build Project
     * @request GET:/api/v1/buildProjects/{id}
     * @secure
     * @response `200` `AuthorizedProject` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getBuildProject: (id: string, params: RequestParams = {}) =>
      this.request<AuthorizedProject, ProblemDetails>({
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
     * @summary Update Build Project
     * @request PATCH:/api/v1/buildProjects/{id}
     * @secure
     * @response `200` `AuthorizedProject` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateBuildProject: (
      id: string,
      data: UpdateBuildProjectInput,
      params: RequestParams = {},
    ) =>
      this.request<AuthorizedProject, ProblemDetails>({
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
     * @name UpdateBuildMetadata
     * @summary Update Build Project
     * @request PATCH:/api/v1/buildProjects/{id}/_metadata
     * @secure
     * @response `200` `AuthorizedProject` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateBuildMetadata: (
      id: string,
      data: BuildMetadataPatch,
      params: RequestParams = {},
    ) =>
      this.request<AuthorizedProject, ProblemDetails>({
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
     * @summary Queue a Build Run
     * @request POST:/api/v1/buildProjects/{id}/runs
     * @secure
     * @response `200` `BuildRunView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    queueBuildRun: (
      id: string,
      data: null | ServerBuildsHttpQueueInput,
      params: RequestParams = {},
    ) =>
      this.request<BuildRunView, ProblemDetails>({
        path: `/api/v1/buildProjects/${id}/runs`,
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
     * @name GetBuildTags
     * @summary Get BuildProject tags
     * @request GET:/api/v1/buildProjects/{id}/tags
     * @secure
     * @response `200` `ResourceTagsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getBuildTags: (id: string, params: RequestParams = {}) =>
      this.request<ResourceTagsResponse, ProblemDetails>({
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
     * @summary Replace BuildProject tags
     * @request PUT:/api/v1/buildProjects/{id}/tags
     * @secure
     * @response `200` `ResourceTagsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    replaceBuildTags: (
      id: string,
      data: ReplaceResourceTagsInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceTagsResponse, ProblemDetails>({
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
     * @tags BuildRuns
     * @name ListBuildRuns
     * @summary List Build Runs
     * @request GET:/api/v1/buildRuns
     * @secure
     * @response `200` `ServerBuildsHttpRuns` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listBuildRuns: (
      query?: {
        /** @format uuid */
        projectId?: string;
        /**
         * @format int32
         * @min 1
         * @max 100
         * @default 50
         */
        limit?: number;
      },
      params: RequestParams = {},
    ) =>
      this.request<ServerBuildsHttpRuns, ProblemDetails>({
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
     * @summary Get a Build Run
     * @request GET:/api/v1/buildRuns/{id}
     * @secure
     * @response `200` `BuildRunView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @name CancelBuildRun
     * @summary Cancel a Build Run
     * @request POST:/api/v1/buildRuns/{id}/cancel
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    cancelBuildRun: (id: string, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/buildRuns/${id}/cancel`,
        method: "POST",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags BuildRuns
     * @name GetBuildRunLogs
     * @summary Get Build Run logs
     * @request GET:/api/v1/buildRuns/{id}/logs
     * @secure
     * @response `200` `ServerBuildsHttpLogs` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getBuildRunLogs: (id: string, params: RequestParams = {}) =>
      this.request<ServerBuildsHttpLogs, ProblemDetails>({
        path: `/api/v1/buildRuns/${id}/logs`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Containers
     * @name DeleteContainers
     * @summary Delete Containers
     * @request DELETE:/api/v1/containers
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    deleteContainers: (data: DeleteInput, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @tags Containers
     * @name PauseContainers
     * @summary Pause Containers
     * @request PATCH:/api/v1/containers/pause
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    pauseContainers: (data: string[], params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @summary Restart Containers
     * @request PATCH:/api/v1/containers/restart
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    restartContainers: (data: string[], params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @name StartContainers
     * @summary Start Containers
     * @request PATCH:/api/v1/containers/start
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    startContainers: (data: string[], params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @summary Stop Containers
     * @request PATCH:/api/v1/containers/stop
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    stopContainers: (data: string[], params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @name UnpauseContainers
     * @summary Unpause Containers
     * @request PATCH:/api/v1/containers/unpause
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    unpauseContainers: (data: string[], params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @name GetContainer
     * @summary Get a Container
     * @request GET:/api/v1/containers/{id}
     * @secure
     * @response `200` `ContainerView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getContainer: (id: string, params: RequestParams = {}) =>
      this.request<ContainerView, ProblemDetails>({
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
     * @name AdoptContainer
     * @summary Adopt a Container without changing Docker
     * @request POST:/api/v1/containers/{id}/adopt
     * @secure
     * @response `200` `DeploymentView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    adoptContainer: (
      id: string,
      data: AdoptContainerInput,
      params: RequestParams = {},
    ) =>
      this.request<DeploymentView, ProblemDetails>({
        path: `/api/v1/containers/${id}/adopt`,
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
     * @tags Containers
     * @name GetContainerAdoptionDraft
     * @summary Review adoption of an unmanaged Container
     * @request GET:/api/v1/containers/{id}/adoption-draft
     * @secure
     * @response `200` `ContainerAdoptionDraft` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    getContainerAdoptionDraft: (id: string, params: RequestParams = {}) =>
      this.request<ContainerAdoptionDraft, ProblemDetails>({
        path: `/api/v1/containers/${id}/adoption-draft`,
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
     * @summary getContainerData
     * @request GET:/api/v1/containers/{id}/data
     * @secure
     * @response `200` `ContainerRuntimeView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    getContainerData: (id: string, params: RequestParams = {}) =>
      this.request<ContainerRuntimeView, ProblemDetails>({
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
     * @name GetContainerInfo
     * @summary getContainerInfo
     * @request GET:/api/v1/containers/{id}/info
     * @secure
     * @response `200` `ContainerSummaryView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    getContainerInfo: (id: string, params: RequestParams = {}) =>
      this.request<ContainerSummaryView, ProblemDetails>({
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
     * @name InspectContainer
     * @summary Inspect a Container with sensitive environment values redacted
     * @request GET:/api/v1/containers/{id}/inspect
     * @secure
     * @response `200` `ContainerInspectionView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    inspectContainer: (id: string, params: RequestParams = {}) =>
      this.request<ContainerInspectionView, ProblemDetails>({
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
     * @name GetContainerStats
     * @summary Get Container statistics
     * @request GET:/api/v1/containers/{id}/stats
     * @secure
     * @response `200` `HistoryContainerStatSnapshot` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getContainerStats: (
      id: string,
      query?: {
        hours?: StatsHours;
      },
      params: RequestParams = {},
    ) =>
      this.request<HistoryContainerStatSnapshot, ProblemDetails>({
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
     * @tags Deployments
     * @name DeleteDeployments
     * @summary Delete Deployments
     * @request DELETE:/api/v1/deployments
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    deleteDeployments: (data: string[], params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @name ListDeployments
     * @summary List authorized Deployments
     * @request GET:/api/v1/deployments
     * @secure
     * @response `200` `DeploymentsView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listDeployments: (
      query?: {
        tags?: string[];
        /** @format uuid */
        platformId?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<DeploymentsView, ProblemDetails>({
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
     * @summary Create a Deployment
     * @request POST:/api/v1/deployments
     * @secure
     * @response `200` `DeploymentView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createDeployment: (
      data: CreateDeploymentInput,
      params: RequestParams = {},
    ) =>
      this.request<DeploymentView, ProblemDetails>({
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
     * @name ApplyDeployment
     * @summary Apply a Deployment
     * @request POST:/api/v1/deployments/apply
     * @secure
     * @response `200` `(DeploymentStreamItem)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    applyDeployment: (data: ApplyDeploymentInput, params: RequestParams = {}) =>
      this.request<DeploymentStreamItem[], ProblemDetails>({
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
     * @name PauseDeployments
     * @summary Pause Deployments
     * @request POST:/api/v1/deployments/pause
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    pauseDeployments: (data: string[], params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @name RenameDeployment
     * @summary Rename a Deployment
     * @request POST:/api/v1/deployments/rename
     * @secure
     * @response `200` `DeploymentView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    renameDeployment: (
      data: RenameDeploymentInput,
      params: RequestParams = {},
    ) =>
      this.request<DeploymentView, ProblemDetails>({
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
     * @name RestartDeployments
     * @summary Restart Deployments
     * @request POST:/api/v1/deployments/restart
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    restartDeployments: (data: string[], params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @name ResumeDeployments
     * @summary Resume Deployments
     * @request POST:/api/v1/deployments/resume
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    resumeDeployments: (data: string[], params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @name StartDeployments
     * @summary Start Deployments
     * @request POST:/api/v1/deployments/start
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    startDeployments: (data: string[], params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @name StopDeployments
     * @summary Stop Deployments
     * @request POST:/api/v1/deployments/stop
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    stopDeployments: (data: string[], params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @name GetDeployment
     * @summary Get a Deployment
     * @request GET:/api/v1/deployments/{deploymentId}
     * @secure
     * @response `200` `DeploymentView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getDeployment: (deploymentId: string, params: RequestParams = {}) =>
      this.request<DeploymentView, ProblemDetails>({
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
     * @name GetDeploymentConfig
     * @summary Get Deployment configuration
     * @request GET:/api/v1/deployments/{deploymentId}/_cfg
     * @secure
     * @response `200` `DeploymentConfigView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getDeploymentConfig: (deploymentId: string, params: RequestParams = {}) =>
      this.request<DeploymentConfigView, ProblemDetails>({
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
     * @name GetDeploymentBackupSourcePreview
     * @summary Preview Deployment Backup volumes
     * @request GET:/api/v1/deployments/{deploymentId}/backup-source-preview
     * @secure
     * @response `200` `BackupSourcePreview` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    getDeploymentBackupSourcePreview: (
      deploymentId: string,
      params: RequestParams = {},
    ) =>
      this.request<BackupSourcePreview, ProblemDetails>({
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
     * @name CheckDeploymentUpdates
     * @summary Check the applied Deployment image for updates
     * @request POST:/api/v1/deployments/{deploymentId}/check-updates
     * @secure
     * @response `200` `DeploymentView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    checkDeploymentUpdates: (
      deploymentId: string,
      params: RequestParams = {},
    ) =>
      this.request<DeploymentView, ProblemDetails>({
        path: `/api/v1/deployments/${deploymentId}/check-updates`,
        method: "POST",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name GetDeploymentDuplicateDraft
     * @summary Build a Deployment duplicate draft
     * @request GET:/api/v1/deployments/{deploymentId}/duplicate-draft
     * @secure
     * @response `200` `DeploymentDuplicateDraftView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getDeploymentDuplicateDraft: (
      deploymentId: string,
      params: RequestParams = {},
    ) =>
      this.request<DeploymentDuplicateDraftView, ProblemDetails>({
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
     * @name GetDeploymentTags
     * @summary Get Deployment tags
     * @request GET:/api/v1/deployments/{deploymentId}/tags
     * @secure
     * @response `200` `ResourceTagsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getDeploymentTags: (deploymentId: string, params: RequestParams = {}) =>
      this.request<ResourceTagsResponse, ProblemDetails>({
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
     * @summary Replace Deployment tags
     * @request PUT:/api/v1/deployments/{deploymentId}/tags
     * @secure
     * @response `200` `ResourceTagsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    replaceDeploymentTags: (
      deploymentId: string,
      data: ReplaceResourceTagsInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceTagsResponse, ProblemDetails>({
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
     * @name UpdateDeployment
     * @summary Update Deployment configuration
     * @request PATCH:/api/v1/deployments/{id}
     * @secure
     * @response `200` `DeploymentView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateDeployment: (
      id: string,
      data: PatchDeploymentInput,
      params: RequestParams = {},
    ) =>
      this.request<DeploymentView, ProblemDetails>({
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
     * @summary Update Deployment metadata
     * @request PATCH:/api/v1/deployments/{id}/_metadata
     * @secure
     * @response `200` `DeploymentView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateDeploymentMetadata: (
      id: string,
      data: PatchDeploymentMetadataInput,
      params: RequestParams = {},
    ) =>
      this.request<DeploymentView, ProblemDetails>({
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
     * @name GetDeploymentContainerInfo
     * @summary getDeploymentContainerInfo
     * @request GET:/api/v1/deployments/{id}/info
     * @secure
     * @response `200` `ContainerSummaryView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    getDeploymentContainerInfo: (id: string, params: RequestParams = {}) =>
      this.request<ContainerSummaryView, ProblemDetails>({
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
     * @name InspectDeployment
     * @summary inspectDeployment
     * @request GET:/api/v1/deployments/{id}/inspect
     * @secure
     * @response `200` `ContainerInspectionView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    inspectDeployment: (id: string, params: RequestParams = {}) =>
      this.request<ContainerInspectionView, ProblemDetails>({
        path: `/api/v1/deployments/${id}/inspect`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Deployments
     * @name GetDeploymentStats
     * @summary Get Deployment statistics
     * @request GET:/api/v1/deployments/{id}/stats
     * @secure
     * @response `200` `HistoryContainerStatSnapshot` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getDeploymentStats: (
      id: string,
      query?: {
        hours?: StatsHours;
      },
      params: RequestParams = {},
    ) =>
      this.request<HistoryContainerStatSnapshot, ProblemDetails>({
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
     * @tags GitAccounts
     * @name DeleteGitAccounts
     * @summary Delete Git accounts
     * @request DELETE:/api/v1/gitAccounts
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    deleteGitAccounts: (
      data: DeleteGitAccountsInput,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
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
     * @name ListGitAccounts
     * @summary List Git accounts
     * @request GET:/api/v1/gitAccounts
     * @secure
     * @response `200` `GitAccountsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listGitAccounts: (params: RequestParams = {}) =>
      this.request<GitAccountsResponse, ProblemDetails>({
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
     * @summary Create a Git account
     * @request POST:/api/v1/gitAccounts
     * @secure
     * @response `200` `GitAccountView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createGitAccount: (data: GitAccountInput, params: RequestParams = {}) =>
      this.request<GitAccountView, ProblemDetails>({
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
     * @name GetGitAccount
     * @summary Get a Git account
     * @request GET:/api/v1/gitAccounts/{id}
     * @secure
     * @response `200` `AuthorizedGitAccountView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getGitAccount: (id: string, params: RequestParams = {}) =>
      this.request<AuthorizedGitAccountView, ProblemDetails>({
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
     * @summary Update a Git account
     * @request PATCH:/api/v1/gitAccounts/{id}
     * @secure
     * @response `200` `GitAccountView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateGitAccount: (
      id: string,
      data: GitAccountPatch,
      params: RequestParams = {},
    ) =>
      this.request<GitAccountView, ProblemDetails>({
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
     * @summary Get Git account configuration
     * @request GET:/api/v1/gitAccounts/{id}/_cfg
     * @secure
     * @response `200` `GitAccountConfigView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getGitAccountConfig: (id: string, params: RequestParams = {}) =>
      this.request<GitAccountConfigView, ProblemDetails>({
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
     * @name DeleteGitRepositories
     * @summary Delete Git repositories
     * @request DELETE:/api/v1/gitRepositories
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    deleteGitRepositories: (
      data: DeleteResourcesInput,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
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
     * @name ListGitRepositories
     * @summary List Git repositories
     * @request GET:/api/v1/gitRepositories
     * @secure
     * @response `200` `GitRepositoriesResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listGitRepositories: (
      query?: {
        tags?: string[];
      },
      params: RequestParams = {},
    ) =>
      this.request<GitRepositoriesResponse, ProblemDetails>({
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
     * @summary Create a Git repository
     * @request POST:/api/v1/gitRepositories
     * @secure
     * @response `200` `GitRepositoryView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createGitRepository: (data: NewGitRepository, params: RequestParams = {}) =>
      this.request<GitRepositoryView, ProblemDetails>({
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
     * @name RenameGitRepository
     * @summary Rename a Git repository
     * @request POST:/api/v1/gitRepositories/rename
     * @secure
     * @response `200` `GitRepositoryView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    renameGitRepository: (
      data: RenameResourceInput,
      params: RequestParams = {},
    ) =>
      this.request<GitRepositoryView, ProblemDetails>({
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
     * @tags GitRepositories
     * @name GetGitRepository
     * @summary Get a Git repository
     * @request GET:/api/v1/gitRepositories/{id}
     * @secure
     * @response `200` `AuthorizedGitRepositoryView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getGitRepository: (id: string, params: RequestParams = {}) =>
      this.request<AuthorizedGitRepositoryView, ProblemDetails>({
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
     * @summary Update a Git repository
     * @request PATCH:/api/v1/gitRepositories/{id}
     * @secure
     * @response `200` `GitRepositoryView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateGitRepository: (
      id: string,
      data: GitRepositoryPatch,
      params: RequestParams = {},
    ) =>
      this.request<GitRepositoryView, ProblemDetails>({
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
     * @name GetGitRepositoryConfig
     * @summary Get Git repository configuration
     * @request GET:/api/v1/gitRepositories/{id}/_cfg
     * @secure
     * @response `200` `GitRepositoryConfigResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getGitRepositoryConfig: (id: string, params: RequestParams = {}) =>
      this.request<GitRepositoryConfigResponse, ProblemDetails>({
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
     * @name UpdateGitRepositoryMetadata
     * @summary Update Git repository metadata
     * @request PATCH:/api/v1/gitRepositories/{id}/_metadata
     * @secure
     * @response `200` `GitRepositoryView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateGitRepositoryMetadata: (
      id: string,
      data: PatchResourceMetadataInput,
      params: RequestParams = {},
    ) =>
      this.request<GitRepositoryView, ProblemDetails>({
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
     * @name DiscoverGitRepositoryBranches
     * @summary Discover remote Git branches
     * @request GET:/api/v1/gitRepositories/{id}/branches
     * @secure
     * @response `200` `GitRepositoryBranchesResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    discoverGitRepositoryBranches: (id: string, params: RequestParams = {}) =>
      this.request<GitRepositoryBranchesResponse, ProblemDetails>({
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
     * @name CompareGitRepositoryCommits
     * @summary Compare immutable Git commits
     * @request GET:/api/v1/gitRepositories/{id}/compare
     * @secure
     * @response `200` `GitCommitComparison` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    compareGitRepositoryCommits: (
      id: string,
      query: {
        baseCommitSha: string;
        headCommitSha: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<GitCommitComparison, ProblemDetails>({
        path: `/api/v1/gitRepositories/${id}/compare`,
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
     * @name DiscoverGitRepositoryComposeProjects
     * @summary Discover Compose projects in a Git repository
     * @request GET:/api/v1/gitRepositories/{id}/compose-projects
     * @secure
     * @response `200` `GitComposeDiscovery` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    discoverGitRepositoryComposeProjects: (
      id: string,
      query?: {
        branch?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<GitComposeDiscovery, ProblemDetails>({
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
     * @name ListGitRepositoryDirectory
     * @summary List files in an immutable Git tree
     * @request GET:/api/v1/gitRepositories/{id}/files
     * @secure
     * @response `200` `GitDirectoryListing` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    listGitRepositoryDirectory: (
      id: string,
      query?: {
        commitSha?: string;
        path?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<GitDirectoryListing, ProblemDetails>({
        path: `/api/v1/gitRepositories/${id}/files`,
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
     * @name GetGitRepositoryFileContent
     * @summary Read a bounded immutable Git file
     * @request GET:/api/v1/gitRepositories/{id}/files/content
     * @secure
     * @response `200` `GitFileContent` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    getGitRepositoryFileContent: (
      id: string,
      query: {
        commitSha?: string;
        path: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<GitFileContent, ProblemDetails>({
        path: `/api/v1/gitRepositories/${id}/files/content`,
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
     * @name GetGitRepositoryRefs
     * @summary Get synchronized Git repository references
     * @request GET:/api/v1/gitRepositories/{id}/refs
     * @secure
     * @response `200` `GitRepositoryRefsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getGitRepositoryRefs: (id: string, params: RequestParams = {}) =>
      this.request<GitRepositoryRefsResponse, ProblemDetails>({
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
     * @name SyncGitRepository
     * @summary Queue Git repository synchronization
     * @request POST:/api/v1/gitRepositories/{id}/sync
     * @secure
     * @response `200` `GitRepositoryView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    syncGitRepository: (
      id: string,
      query?: {
        branch?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<GitRepositoryView, ProblemDetails>({
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
     * @name GetGitRepositoryTags
     * @summary Get Git repository tags
     * @request GET:/api/v1/gitRepositories/{id}/tags
     * @secure
     * @response `200` `ResourceTagsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getGitRepositoryTags: (id: string, params: RequestParams = {}) =>
      this.request<ResourceTagsResponse, ProblemDetails>({
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
     * @summary Replace Git repository tags
     * @request PUT:/api/v1/gitRepositories/{id}/tags
     * @secure
     * @response `200` `ResourceTagsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    replaceGitRepositoryTags: (
      id: string,
      data: ReplaceResourceTagsInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceTagsResponse, ProblemDetails>({
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
     * @tags Images
     * @name DeleteImages
     * @summary Delete Images
     * @request DELETE:/api/v1/images
     * @secure
     * @response `200` `DeleteImagesView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    deleteImages: (data: DeleteImagesInput, params: RequestParams = {}) =>
      this.request<DeleteImagesView, ProblemDetails>({
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
     * @tags Images
     * @name GetDockerHubRepositories
     * @summary List Docker Hub repositories
     * @request GET:/api/v1/images/dockerhub/{registryName}/repositories
     * @secure
     * @response `200` `(DockerHubRepositoryInfo)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getDockerHubRepositories: (
      registryName: string,
      params: RequestParams = {},
    ) =>
      this.request<DockerHubRepositoryInfo[], ProblemDetails>({
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
     * @summary List Docker Hub repository tags
     * @request GET:/api/v1/images/dockerhub/{registryName}/{repositoryName}/tags
     * @secure
     * @response `200` `(DockerHubTag)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getDockerHubRepositoryTags: (
      registryName: string,
      repositoryName: string,
      params: RequestParams = {},
    ) =>
      this.request<DockerHubTag[], ProblemDetails>({
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
     * @name GetGhcrPackageVersions
     * @summary List GitHub package versions
     * @request GET:/api/v1/images/ghcr/{registryName}/{packageName}/versions
     * @secure
     * @response `200` `(GithubPackageVersion)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getGhcrPackageVersions: (
      registryName: string,
      packageName: string,
      params: RequestParams = {},
    ) =>
      this.request<GithubPackageVersion[], ProblemDetails>({
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
     * @name PullImage
     * @summary Pull a Docker image with progress
     * @request POST:/api/v1/images/pull
     * @secure
     * @response `200` `(PullImageStreamItem)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    pullImage: (data: PullImageInput, params: RequestParams = {}) =>
      this.request<PullImageStreamItem[], ProblemDetails>({
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
     * @name ListImages
     * @summary List Platform Images
     * @request GET:/api/v1/images/{platformId}
     * @secure
     * @response `200` `ImagesResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listImages: (platformId: string, params: RequestParams = {}) =>
      this.request<ImagesResponse, ProblemDetails>({
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
     * @name InspectImage
     * @summary Inspect an Image on its owning Docker node
     * @request GET:/api/v1/images/{platformId}/{imageId}
     * @secure
     * @response `200` `ImageInspectionView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    inspectImage: (
      platformId: string,
      imageId: string,
      query?: {
        dockerNodeId?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<ImageInspectionView, ProblemDetails>({
        path: `/api/v1/images/${platformId}/${imageId}`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Images
     * @name GetExposedPorts
     * @summary Read exposed ports using the Citadel Image ID
     * @request GET:/api/v1/images/{platformId}/{imageId}/_ports
     * @secure
     * @response `200` `ExposedPortsView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    getExposedPorts: (
      platformId: string,
      imageId: string,
      params: RequestParams = {},
    ) =>
      this.request<ExposedPortsView, ProblemDetails>({
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
     * @name GetExternalRepositories
     * @summary List Registry repositories
     * @request GET:/api/v1/images/{registryName}/repositories
     * @secure
     * @response `200` `(ExternalRepository)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getExternalRepositories: (
      registryName: string,
      params: RequestParams = {},
    ) =>
      this.request<ExternalRepository[], ProblemDetails>({
        path: `/api/v1/images/${registryName}/repositories`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags License
     * @name RemoveLicense
     * @summary Remove the installed license
     * @request DELETE:/api/v1/license
     * @secure
     * @response `200` `LicenseView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @name GetLicense
     * @summary Get installed license state
     * @request GET:/api/v1/license
     * @secure
     * @response `200` `LicenseView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @summary Install or replace a license
     * @request POST:/api/v1/license
     * @secure
     * @response `200` `LicenseView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    installLicense: (data: InstallLicenseRequest, params: RequestParams = {}) =>
      this.request<LicenseView, ProblemDetails>({
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
     * @name GetLicenseEntitlements
     * @summary Get effective license entitlements
     * @request GET:/api/v1/license/entitlements
     * @secure
     * @response `200` `LicenseEntitlementsView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getLicenseEntitlements: (params: RequestParams = {}) =>
      this.request<LicenseEntitlementsView, ProblemDetails>({
        path: `/api/v1/license/entitlements`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags License
     * @name GetLicenseRequest
     * @summary Get license request metadata
     * @request GET:/api/v1/license/request
     * @secure
     * @response `200` `LicenseRequestView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @tags Lookup
     * @name Lookup
     * @summary Look up accessible resources
     * @request GET:/api/v1/lookup
     * @secure
     * @response `200` `(LookupResourceInfo)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
      this.request<LookupResourceInfo[], ProblemDetails>({
        path: `/api/v1/lookup`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Networks
     * @name DeleteNetworks
     * @summary Delete Networks
     * @request DELETE:/api/v1/networks
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    deleteNetworks: (data: DeleteNetworksInput, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @tags Networks
     * @name CreateNetwork
     * @summary Create a Network
     * @request POST:/api/v1/networks
     * @secure
     * @response `200` `CreatedNetworkView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createNetwork: (data: CreateNetworkInput, params: RequestParams = {}) =>
      this.request<CreatedNetworkView, ProblemDetails>({
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
     * @name ListNetworks
     * @summary List Platform Networks
     * @request GET:/api/v1/networks/{platformId}
     * @secure
     * @response `200` `NetworksResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
      this.request<NetworksResponse, ProblemDetails>({
        path: `/api/v1/networks/${platformId}`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Networks
     * @name InspectNetwork
     * @summary Inspect a Platform Network
     * @request GET:/api/v1/networks/{platformId}/{networkId}
     * @secure
     * @response `200` `NetworkView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    inspectNetwork: (
      platformId: string,
      networkId: string,
      query?: {
        dockerNodeId?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<NetworkView, ProblemDetails>({
        path: `/api/v1/networks/${platformId}/${networkId}`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
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
     * @response `200` `OidcProvidersView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listOidcProviders: (params: RequestParams = {}) =>
      this.request<OidcProvidersView, ProblemDetails>({
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
     * @response `200` `OidcProviderView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createOidcProvider: (
      data: CreateOidcProviderRequest,
      params: RequestParams = {},
    ) =>
      this.request<OidcProviderView, ProblemDetails>({
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
     * @name RenameOidcProvider
     * @summary Rename OIDC provider
     * @request POST:/api/v1/oidcProviders/rename
     * @secure
     * @response `200` `OidcProviderView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    renameOidcProvider: (
      data: RenameOidcProviderRequest,
      params: RequestParams = {},
    ) =>
      this.request<OidcProviderView, ProblemDetails>({
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
     * @name TestOidcDiscovery
     * @summary Test OIDC discovery
     * @request POST:/api/v1/oidcProviders/testDiscovery
     * @secure
     * @response `200` `OidcDiscoveryView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    testOidcDiscovery: (
      data: TestOidcDiscoveryRequest,
      params: RequestParams = {},
    ) =>
      this.request<OidcDiscoveryView, ProblemDetails>({
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
     * @tags OidcProviders
     * @name DeleteOidcProvider
     * @summary Delete OIDC provider
     * @request DELETE:/api/v1/oidcProviders/{id}
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    deleteOidcProvider: (id: string, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/oidcProviders/${id}`,
        method: "DELETE",
        secure: true,
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
     * @response `200` `OidcProviderView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getOidcProvider: (id: string, params: RequestParams = {}) =>
      this.request<OidcProviderView, ProblemDetails>({
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
     * @response `200` `OidcProviderView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateOidcProvider: (
      id: string,
      data: PatchOidcProviderRequest,
      params: RequestParams = {},
    ) =>
      this.request<OidcProviderView, ProblemDetails>({
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
     * @name UpdateOidcProviderMetadata
     * @summary Update OIDC provider metadata
     * @request PATCH:/api/v1/oidcProviders/{id}/_metadata
     * @secure
     * @response `200` `OidcProviderView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateOidcProviderMetadata: (
      id: string,
      data: PatchOidcProviderMetadataRequest,
      params: RequestParams = {},
    ) =>
      this.request<OidcProviderView, ProblemDetails>({
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
     * @response `200` `OidcDiscoveryView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    testOidcProviderDiscovery: (id: string, params: RequestParams = {}) =>
      this.request<OidcDiscoveryView, ProblemDetails>({
        path: `/api/v1/oidcProviders/${id}/testDiscovery`,
        method: "POST",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name DeletePlatforms
     * @summary Delete Platform registrations
     * @request DELETE:/api/v1/platforms
     * @secure
     * @response `200` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    deletePlatforms: (data: DeletePlatformsInput, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @name ListPlatforms
     * @summary List authorized Platforms
     * @request GET:/api/v1/platforms
     * @secure
     * @response `200` `PlatformsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listPlatforms: (
      query?: {
        tags?: string[];
      },
      params: RequestParams = {},
    ) =>
      this.request<PlatformsResponse, ProblemDetails>({
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
     * @summary Create a Platform
     * @request POST:/api/v1/platforms
     * @secure
     * @response `200` `PlatformView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createPlatform: (data: CreatePlatformInput, params: RequestParams = {}) =>
      this.request<PlatformView, ProblemDetails>({
        path: `/api/v1/platforms`,
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
     * @name GetAgentSetup
     * @summary Get regular Agent setup instructions
     * @request GET:/api/v1/platforms/agent/setup
     * @secure
     * @response `200` `AgentSetupView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getAgentSetup: (params: RequestParams = {}) =>
      this.request<AgentSetupView, ProblemDetails>({
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
     * @summary Rotate the Agent signing key
     * @request POST:/api/v1/platforms/agent/setup/rotate-key
     * @secure
     * @response `200` `AgentSetupView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    rotateAgentHubKey: (params: RequestParams = {}) =>
      this.request<AgentSetupView, ProblemDetails>({
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
     * @name RenamePlatform
     * @summary Rename a Platform
     * @request POST:/api/v1/platforms/rename
     * @secure
     * @response `200` `PlatformView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    renamePlatform: (data: RenamePlatformInput, params: RequestParams = {}) =>
      this.request<PlatformView, ProblemDetails>({
        path: `/api/v1/platforms/rename`,
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
     * @name GetPlatfom
     * @summary Get a Platform
     * @request GET:/api/v1/platforms/{id}
     * @secure
     * @response `200` `PlatformView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getPlatfom: (id: string, params: RequestParams = {}) =>
      this.request<PlatformView, ProblemDetails>({
        path: `/api/v1/platforms/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name UpdatePlatform
     * @summary Update a Platform
     * @request PATCH:/api/v1/platforms/{id}
     * @secure
     * @response `200` `PlatformView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updatePlatform: (
      id: string,
      data: PlatformPatch,
      params: RequestParams = {},
    ) =>
      this.request<PlatformView, ProblemDetails>({
        path: `/api/v1/platforms/${id}`,
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
     * @tags Platforms
     * @name UpdatePlatformMetadata
     * @summary Update Platform metadata
     * @request PATCH:/api/v1/platforms/{id}/_metadata
     * @secure
     * @response `200` `PlatformView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updatePlatformMetadata: (
      id: string,
      data: PatchPlatformMetadataInput,
      params: RequestParams = {},
    ) =>
      this.request<PlatformView, ProblemDetails>({
        path: `/api/v1/platforms/${id}/_metadata`,
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
     * @tags Platforms
     * @name ListContainers
     * @summary List Platform Containers
     * @request GET:/api/v1/platforms/{id}/containers
     * @secure
     * @response `200` `ContainersResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listContainers: (id: string, params: RequestParams = {}) =>
      this.request<ContainersResponse, ProblemDetails>({
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
     * @name CreateEdgeAgentEnrollment
     * @summary Create an Edge Agent enrollment token
     * @request POST:/api/v1/platforms/{id}/edge/enrollments
     * @secure
     * @response `200` `EdgeEnrollmentView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createEdgeAgentEnrollment: (id: string, params: RequestParams = {}) =>
      this.request<EdgeEnrollmentView, ProblemDetails>({
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
     * @name RevokeEdgeAgent
     * @summary Revoke an Edge Agent binding
     * @request POST:/api/v1/platforms/{id}/edge/revoke
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @name GetEdgeAgentStatus
     * @summary Get Edge Agent connection status
     * @request GET:/api/v1/platforms/{id}/edge/status
     * @secure
     * @response `200` `EdgeStatusView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getEdgeAgentStatus: (id: string, params: RequestParams = {}) =>
      this.request<EdgeStatusView, ProblemDetails>({
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
     * @name GetSwarmNodeAgentCoverage
     * @summary Get Docker Swarm node-agent coverage
     * @request GET:/api/v1/platforms/{id}/node-agent-coverage
     * @secure
     * @response `200` `SwarmNodeAgentCoverage` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getSwarmNodeAgentCoverage: (id: string, params: RequestParams = {}) =>
      this.request<SwarmNodeAgentCoverage, ProblemDetails>({
        path: `/api/v1/platforms/${id}/node-agent-coverage`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name RemoveSwarmNodeAgents
     * @summary Remove Docker Swarm node agents
     * @request DELETE:/api/v1/platforms/{id}/node-agents
     * @secure
     * @response `200` `(NodeAgentProgress)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    removeSwarmNodeAgents: (id: string, params: RequestParams = {}) =>
      this.request<NodeAgentProgress[], ProblemDetails>({
        path: `/api/v1/platforms/${id}/node-agents`,
        method: "DELETE",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name InstallSwarmNodeAgents
     * @summary Install Docker Swarm node agents
     * @request POST:/api/v1/platforms/{id}/node-agents/install
     * @secure
     * @response `200` `(NodeAgentProgress)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    installSwarmNodeAgents: (id: string, params: RequestParams = {}) =>
      this.request<NodeAgentProgress[], ProblemDetails>({
        path: `/api/v1/platforms/${id}/node-agents/install`,
        method: "POST",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name RepairSwarmNodeAgents
     * @summary Repair Docker Swarm node-agent coverage
     * @request POST:/api/v1/platforms/{id}/node-agents/repair
     * @secure
     * @response `200` `(NodeAgentProgress)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    repairSwarmNodeAgents: (id: string, params: RequestParams = {}) =>
      this.request<NodeAgentProgress[], ProblemDetails>({
        path: `/api/v1/platforms/${id}/node-agents/repair`,
        method: "POST",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name UpgradeSwarmNodeAgents
     * @summary Upgrade Docker Swarm node agents
     * @request POST:/api/v1/platforms/{id}/node-agents/upgrade
     * @secure
     * @response `200` `(NodeAgentProgress)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    upgradeSwarmNodeAgents: (id: string, params: RequestParams = {}) =>
      this.request<NodeAgentProgress[], ProblemDetails>({
        path: `/api/v1/platforms/${id}/node-agents/upgrade`,
        method: "POST",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name PrunePlatform
     * @summary Prune unused Docker resources
     * @request POST:/api/v1/platforms/{id}/prune
     * @secure
     * @response `200` `PrunePlatformView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @name GetPlatformStats
     * @summary Get Platform statistics
     * @request GET:/api/v1/platforms/{id}/stats
     * @secure
     * @response `200` `HistoryPlatformStatSnapshot` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getPlatformStats: (
      id: string,
      query?: {
        hours?: StatsHours;
      },
      params: RequestParams = {},
    ) =>
      this.request<HistoryPlatformStatSnapshot, ProblemDetails>({
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
     * @name GetPlatformTags
     * @summary Get Platform tags
     * @request GET:/api/v1/platforms/{id}/tags
     * @secure
     * @response `200` `ResourceTagsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getPlatformTags: (id: string, params: RequestParams = {}) =>
      this.request<ResourceTagsResponse, ProblemDetails>({
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
     * @summary Replace Platform tags
     * @request PUT:/api/v1/platforms/{id}/tags
     * @secure
     * @response `200` `ResourceTagsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    replacePlatformTags: (
      id: string,
      data: ReplaceResourceTagsInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceTagsResponse, ProblemDetails>({
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
     * @name GetSwarmOverview
     * @summary Get Swarm inventory health and quorum
     * @request GET:/api/v1/platforms/{platformId}/swarm
     * @secure
     * @response `200` `SwarmOverviewView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    getSwarmOverview: (platformId: string, params: RequestParams = {}) =>
      this.request<SwarmOverviewView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name DeleteSwarmConfigs
     * @summary deleteSwarmConfigs
     * @request DELETE:/api/v1/platforms/{platformId}/swarm/configs
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    deleteSwarmConfigs: (
      platformId: string,
      data: DeleteSwarmResourcesInput,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/configs`,
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
     * @name ListSwarmConfigs
     * @summary List Swarm Configs
     * @request GET:/api/v1/platforms/{platformId}/swarm/configs
     * @secure
     * @response `200` `SwarmItemsResponseSwarmConfigView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listSwarmConfigs: (platformId: string, params: RequestParams = {}) =>
      this.request<SwarmItemsResponseSwarmConfigView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/configs`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name CreateSwarmConfig
     * @summary createSwarmConfig
     * @request POST:/api/v1/platforms/{platformId}/swarm/configs
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createSwarmConfig: (
      platformId: string,
      data: CreateSwarmMaterialInput,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/configs`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name GetSwarmConfig
     * @summary Get a Swarm Config
     * @request GET:/api/v1/platforms/{platformId}/swarm/configs/{resourceId}
     * @secure
     * @response `200` `SwarmConfigView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getSwarmConfig: (
      platformId: string,
      resourceId: string,
      params: RequestParams = {},
    ) =>
      this.request<SwarmConfigView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/configs/${resourceId}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name GetSwarmConfigData
     * @summary getSwarmConfigData
     * @request GET:/api/v1/platforms/{platformId}/swarm/configs/{resourceId}/content
     * @secure
     * @response `200` `ConfigContentView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getSwarmConfigData: (
      platformId: string,
      resourceId: string,
      params: RequestParams = {},
    ) =>
      this.request<ConfigContentView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/configs/${resourceId}/content`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name UpdateSwarmConfigLabels
     * @summary updateSwarmConfigLabels
     * @request PATCH:/api/v1/platforms/{platformId}/swarm/configs/{resourceId}/labels
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateSwarmConfigLabels: (
      platformId: string,
      resourceId: string,
      data: UpdateSwarmResourceLabelsInput,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/configs/${resourceId}/labels`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name ListSwarmNetworks
     * @summary List Swarm Networks
     * @request GET:/api/v1/platforms/{platformId}/swarm/networks
     * @secure
     * @response `200` `SwarmItemsResponseSwarmNetworkView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listSwarmNetworks: (platformId: string, params: RequestParams = {}) =>
      this.request<SwarmItemsResponseSwarmNetworkView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/networks`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name GetSwarmNetwork
     * @summary Get a Swarm Network
     * @request GET:/api/v1/platforms/{platformId}/swarm/networks/{resourceId}
     * @secure
     * @response `200` `SwarmNetworkView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getSwarmNetwork: (
      platformId: string,
      resourceId: string,
      params: RequestParams = {},
    ) =>
      this.request<SwarmNetworkView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/networks/${resourceId}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name ListSwarmNodes
     * @summary List Swarm Nodes
     * @request GET:/api/v1/platforms/{platformId}/swarm/nodes
     * @secure
     * @response `200` `SwarmItemsResponseSwarmNodeView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listSwarmNodes: (platformId: string, params: RequestParams = {}) =>
      this.request<SwarmItemsResponseSwarmNodeView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/nodes`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name UpdateSwarmNodesAvailability
     * @summary updateSwarmNodesAvailability
     * @request PATCH:/api/v1/platforms/{platformId}/swarm/nodes/availability
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateSwarmNodesAvailability: (
      platformId: string,
      data: UpdateSwarmNodesAvailabilityInput,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/nodes/availability`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name GetSwarmNode
     * @summary Get a Swarm Node
     * @request GET:/api/v1/platforms/{platformId}/swarm/nodes/{nodeId}
     * @secure
     * @response `200` `SwarmNodeView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getSwarmNode: (
      platformId: string,
      nodeId: string,
      params: RequestParams = {},
    ) =>
      this.request<SwarmNodeView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/nodes/${nodeId}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name UpdateSwarmNode
     * @summary updateSwarmNode
     * @request PATCH:/api/v1/platforms/{platformId}/swarm/nodes/{nodeId}
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateSwarmNode: (
      platformId: string,
      nodeId: string,
      data: UpdateSwarmNodeInput,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/nodes/${nodeId}`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name InspectSwarmNode
     * @summary inspectSwarmNode
     * @request GET:/api/v1/platforms/{platformId}/swarm/nodes/{nodeId}/inspect
     * @secure
     * @response `200` `NodeInspectionView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    inspectSwarmNode: (
      platformId: string,
      nodeId: string,
      params: RequestParams = {},
    ) =>
      this.request<NodeInspectionView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/nodes/${nodeId}/inspect`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name DeleteSwarmSecrets
     * @summary deleteSwarmSecrets
     * @request DELETE:/api/v1/platforms/{platformId}/swarm/secrets
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    deleteSwarmSecrets: (
      platformId: string,
      data: DeleteSwarmResourcesInput,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/secrets`,
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
     * @name ListSwarmSecrets
     * @summary List Swarm Secrets
     * @request GET:/api/v1/platforms/{platformId}/swarm/secrets
     * @secure
     * @response `200` `SwarmItemsResponseSwarmSecretView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listSwarmSecrets: (platformId: string, params: RequestParams = {}) =>
      this.request<SwarmItemsResponseSwarmSecretView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/secrets`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name CreateSwarmSecret
     * @summary createSwarmSecret
     * @request POST:/api/v1/platforms/{platformId}/swarm/secrets
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createSwarmSecret: (
      platformId: string,
      data: CreateSwarmMaterialInput,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/secrets`,
        method: "POST",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name GetSwarmSecret
     * @summary Get a Swarm Secret
     * @request GET:/api/v1/platforms/{platformId}/swarm/secrets/{resourceId}
     * @secure
     * @response `200` `SwarmSecretView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getSwarmSecret: (
      platformId: string,
      resourceId: string,
      params: RequestParams = {},
    ) =>
      this.request<SwarmSecretView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/secrets/${resourceId}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name UpdateSwarmSecretLabels
     * @summary updateSwarmSecretLabels
     * @request PATCH:/api/v1/platforms/{platformId}/swarm/secrets/{resourceId}/labels
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateSwarmSecretLabels: (
      platformId: string,
      resourceId: string,
      data: UpdateSwarmResourceLabelsInput,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/secrets/${resourceId}/labels`,
        method: "PATCH",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name DeleteSwarmInventoryServices
     * @summary deleteSwarmInventoryServices
     * @request DELETE:/api/v1/platforms/{platformId}/swarm/services
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    deleteSwarmInventoryServices: (
      platformId: string,
      data: DeleteSwarmResourcesInput,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/services`,
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
     * @name ListSwarmServices
     * @summary List Swarm Services
     * @request GET:/api/v1/platforms/{platformId}/swarm/services
     * @secure
     * @response `200` `SwarmItemsResponseSwarmServiceView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listSwarmServices: (platformId: string, params: RequestParams = {}) =>
      this.request<SwarmItemsResponseSwarmServiceView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/services`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name GetSwarmService
     * @summary Get a Swarm Service
     * @request GET:/api/v1/platforms/{platformId}/swarm/services/{resourceId}
     * @secure
     * @response `200` `SwarmServiceView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getSwarmService: (
      platformId: string,
      resourceId: string,
      params: RequestParams = {},
    ) =>
      this.request<SwarmServiceView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/services/${resourceId}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name AdoptSwarmService
     * @summary Adopt an existing Docker Service without changing Docker
     * @request POST:/api/v1/platforms/{platformId}/swarm/services/{resourceId}/adopt
     * @secure
     * @response `200` `ManagedSwarmServiceView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    adoptSwarmService: (
      platformId: string,
      resourceId: string,
      data: AdoptSwarmServiceInput,
      params: RequestParams = {},
    ) =>
      this.request<ManagedSwarmServiceView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/services/${resourceId}/adopt`,
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
     * @name GetSwarmServiceAdoptionDraft
     * @summary Review an unmanaged Docker Service for adoption
     * @request GET:/api/v1/platforms/{platformId}/swarm/services/{resourceId}/adoption-draft
     * @secure
     * @response `200` `SwarmServiceAdoptionDraftView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getSwarmServiceAdoptionDraft: (
      platformId: string,
      resourceId: string,
      params: RequestParams = {},
    ) =>
      this.request<SwarmServiceAdoptionDraftView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/services/${resourceId}/adoption-draft`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name InspectSwarmService
     * @summary inspectSwarmService
     * @request GET:/api/v1/platforms/{platformId}/swarm/services/{resourceId}/inspect
     * @secure
     * @response `200` `ServiceInspectionView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    inspectSwarmService: (
      platformId: string,
      resourceId: string,
      params: RequestParams = {},
    ) =>
      this.request<ServiceInspectionView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/services/${resourceId}/inspect`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name GetSwarmServiceLogs
     * @summary Read bounded Service logs
     * @request GET:/api/v1/platforms/{platformId}/swarm/services/{resourceId}/logs
     * @secure
     * @response `200` `LogSnapshot` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    getSwarmServiceLogs: (
      platformId: string,
      resourceId: string,
      query?: {
        /**
         * @format int32
         * @min 1
         * @max 200
         * @default 100
         */
        tail?: number;
      },
      params: RequestParams = {},
    ) =>
      this.request<LogSnapshot, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/services/${resourceId}/logs`,
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
     * @name RestartSwarmService
     * @summary restartSwarmService
     * @request POST:/api/v1/platforms/{platformId}/swarm/services/{resourceId}/restart
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    restartSwarmService: (
      platformId: string,
      resourceId: string,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/services/${resourceId}/restart`,
        method: "POST",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name GetSwarmServiceStats
     * @summary Get Service statistics and node coverage
     * @request GET:/api/v1/platforms/{platformId}/swarm/services/{resourceId}/stats
     * @secure
     * @response `200` `ServiceStatistics` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    getSwarmServiceStats: (
      platformId: string,
      resourceId: string,
      query?: {
        hours?: StatsHours;
      },
      params: RequestParams = {},
    ) =>
      this.request<ServiceStatistics, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/services/${resourceId}/stats`,
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
     * @name ListSwarmTasks
     * @summary List Swarm Tasks
     * @request GET:/api/v1/platforms/{platformId}/swarm/tasks
     * @secure
     * @response `200` `SwarmItemsResponseSwarmTaskView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listSwarmTasks: (
      platformId: string,
      query?: {
        /**
         * @format int32
         * @min 1
         * @max 200
         * @default 50
         */
        limit?: number;
        serviceId?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<SwarmItemsResponseSwarmTaskView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/tasks`,
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
     * @name GetSwarmTask
     * @summary Get a Swarm Task
     * @request GET:/api/v1/platforms/{platformId}/swarm/tasks/{resourceId}
     * @secure
     * @response `200` `SwarmTaskView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getSwarmTask: (
      platformId: string,
      resourceId: string,
      params: RequestParams = {},
    ) =>
      this.request<SwarmTaskView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/tasks/${resourceId}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name InspectSwarmTask
     * @summary Inspect the current Task container
     * @request GET:/api/v1/platforms/{platformId}/swarm/tasks/{resourceId}/inspect
     * @secure
     * @response `200` `ContainerInspectionView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    inspectSwarmTask: (
      platformId: string,
      resourceId: string,
      params: RequestParams = {},
    ) =>
      this.request<ContainerInspectionView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/tasks/${resourceId}/inspect`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name GetSwarmTaskLogs
     * @summary Read current Task logs on its owning node
     * @request GET:/api/v1/platforms/{platformId}/swarm/tasks/{resourceId}/logs
     * @secure
     * @response `200` `LogSnapshot` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    getSwarmTaskLogs: (
      platformId: string,
      resourceId: string,
      query?: {
        /**
         * @format int32
         * @min 1
         * @max 200
         * @default 100
         */
        tail?: number;
      },
      params: RequestParams = {},
    ) =>
      this.request<LogSnapshot, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/tasks/${resourceId}/logs`,
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
     * @name GetSwarmTaskStats
     * @summary Get current Task statistics
     * @request GET:/api/v1/platforms/{platformId}/swarm/tasks/{resourceId}/stats
     * @secure
     * @response `200` `TaskHistory` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    getSwarmTaskStats: (
      platformId: string,
      resourceId: string,
      query?: {
        hours?: StatsHours;
      },
      params: RequestParams = {},
    ) =>
      this.request<TaskHistory, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/tasks/${resourceId}/stats`,
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
     * @name GetSwarmTaskTerminalTarget
     * @summary Resolve the current Task terminal target
     * @request GET:/api/v1/platforms/{platformId}/swarm/tasks/{resourceId}/terminal
     * @secure
     * @response `200` `TaskTerminalView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    getSwarmTaskTerminalTarget: (
      platformId: string,
      resourceId: string,
      params: RequestParams = {},
    ) =>
      this.request<TaskTerminalView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/swarm/tasks/${resourceId}/terminal`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Platforms
     * @name GetComposeProjectImportDraft
     * @summary Get a Compose or Swarm Stack import draft
     * @request GET:/api/v1/platforms/{platformId}/unmanaged-compose-projects/{projectName}
     * @secure
     * @response `200` `ComposeProjectImportDraftView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getComposeProjectImportDraft: (
      platformId: string,
      projectName: string,
      query?: {
        importKind?: StackImportKind;
      },
      params: RequestParams = {},
    ) =>
      this.request<ComposeProjectImportDraftView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/unmanaged-compose-projects/${projectName}`,
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
     * @name ImportComposeProject
     * @summary Import a Compose project or Swarm Stack
     * @request POST:/api/v1/platforms/{platformId}/unmanaged-compose-projects/{projectName}/import
     * @secure
     * @response `200` `StackView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    importComposeProject: (
      platformId: string,
      projectName: string,
      data: ImportRequest,
      params: RequestParams = {},
    ) =>
      this.request<StackView, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/unmanaged-compose-projects/${projectName}/import`,
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
     * @name ValidateComposeProjectImportDraft
     * @summary Validate a source for an unmanaged Compose project
     * @request POST:/api/v1/platforms/{platformId}/unmanaged-compose-projects/{projectName}/import-draft
     * @secure
     * @response `200` `ComposeProjectImportValidation` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    validateComposeProjectImportDraft: (
      platformId: string,
      projectName: string,
      data: ValidateImportRequest,
      params: RequestParams = {},
    ) =>
      this.request<ComposeProjectImportValidation, ProblemDetails>({
        path: `/api/v1/platforms/${platformId}/unmanaged-compose-projects/${projectName}/import-draft`,
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
     * @name ListVolumeDirectory
     * @summary Browse a Volume directory
     * @request GET:/api/v1/platforms/{platformId}/volumes/{name}/files
     * @secure
     * @response `200` `VolumeDirectory` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    listVolumeDirectory: (
      platformId: string,
      name: string,
      query?: {
        path?: string;
        dockerNodeId?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<VolumeDirectory, ProblemDetails>({
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
     * @summary Download a Volume file or directory
     * @request GET:/api/v1/platforms/{platformId}/volumes/{name}/files/download
     * @secure
     * @response `200` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    downloadVolumePath: (
      platformId: string,
      name: string,
      query?: {
        path?: string;
        dockerNodeId?: string;
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
     * @tags Profile
     * @name GetCurrentProfile
     * @summary Get current profile
     * @request GET:/api/v1/profile
     * @secure
     * @response `200` `CurrentProfileView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @response `200` `CurrentProfileView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateCurrentProfile: (
      data: UpdateCurrentProfileRequest,
      params: RequestParams = {},
    ) =>
      this.request<CurrentProfileView, ProblemDetails>({
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
     * @name ChangeCurrentPassword
     * @summary Change current profile password
     * @request POST:/api/v1/profile/change-password
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    changeCurrentPassword: (
      data: ChangeCurrentPasswordRequest,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
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
     * @name GetProfileMfaStatus
     * @summary Get MFA status
     * @request GET:/api/v1/profile/mfa
     * @secure
     * @response `200` `ProfileMfaStatusView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @name DisableProfileMfa
     * @summary Disable MFA
     * @request POST:/api/v1/profile/mfa/disable
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    disableProfileMfa: (
      data: DisableProfileMfaInput,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
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
     * @summary Regenerate MFA recovery codes
     * @request POST:/api/v1/profile/mfa/recovery-codes
     * @secure
     * @response `200` `ProfileMfaRecoveryCodesView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    regenerateProfileMfaRecoveryCodes: (
      data: RegenerateProfileMfaRecoveryCodesInput,
      params: RequestParams = {},
    ) =>
      this.request<ProfileMfaRecoveryCodesView, ProblemDetails>({
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
     * @tags Profile
     * @name StartProfileMfaSetup
     * @summary Start MFA setup
     * @request POST:/api/v1/profile/mfa/setup
     * @secure
     * @response `200` `ProfileMfaSetupView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    startProfileMfaSetup: (
      data: StartProfileMfaSetupInput,
      params: RequestParams = {},
    ) =>
      this.request<ProfileMfaSetupView, ProblemDetails>({
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
     * @summary Complete MFA setup
     * @request POST:/api/v1/profile/mfa/setup/confirm
     * @secure
     * @response `200` `ProfileMfaRecoveryCodesView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    confirmProfileMfaSetup: (
      data: ConfirmProfileMfaSetupInput,
      params: RequestParams = {},
    ) =>
      this.request<ProfileMfaRecoveryCodesView, ProblemDetails>({
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
     * @name GetProfilePreferences
     * @summary Get current profile preferences
     * @request GET:/api/v1/profile/preferences
     * @secure
     * @response `200` `UserPreferencesView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @summary Update current profile preferences
     * @request PATCH:/api/v1/profile/preferences
     * @secure
     * @response `200` `UserPreferencesView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    patchProfilePreferences: (
      data: PatchUserPreferencesRequest,
      params: RequestParams = {},
    ) =>
      this.request<UserPreferencesView, ProblemDetails>({
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
     * @name RevokeOtherProfileSessions
     * @summary Revoke other profile sessions
     * @request DELETE:/api/v1/profile/sessions
     * @secure
     * @response `200` `RevokeOtherProfileSessionsView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    revokeOtherProfileSessions: (params: RequestParams = {}) =>
      this.request<RevokeOtherProfileSessionsView, ProblemDetails>({
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
     * @name ListProfileSessions
     * @summary List current profile sessions
     * @request GET:/api/v1/profile/sessions
     * @secure
     * @response `200` `UserSessionsView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @name RevokeProfileSession
     * @summary Revoke a profile session
     * @request DELETE:/api/v1/profile/sessions/{sessionId}
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @tags Registries
     * @name DeleteRegistries
     * @summary Delete Registries
     * @request DELETE:/api/v1/registries
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    deleteRegistries: (
      data: DeleteResourcesInput,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
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
     * @name ListRegistries
     * @summary List Registries
     * @request GET:/api/v1/registries
     * @secure
     * @response `200` `RegistriesResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listRegistries: (
      query?: {
        /** @default false */
        includeDisabled?: boolean;
        tags?: string[];
      },
      params: RequestParams = {},
    ) =>
      this.request<RegistriesResponse, ProblemDetails>({
        path: `/api/v1/registries`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Registries
     * @name CreateRegistry
     * @summary Create a Registry
     * @request POST:/api/v1/registries
     * @secure
     * @response `200` `RegistryView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createRegistry: (data: NewRegistry, params: RequestParams = {}) =>
      this.request<RegistryView, ProblemDetails>({
        path: `/api/v1/registries`,
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
     * @tags Registries
     * @name RenameRegistry
     * @summary Rename a Registry
     * @request POST:/api/v1/registries/rename
     * @secure
     * @response `200` `RegistryView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    renameRegistry: (data: RenameResourceInput, params: RequestParams = {}) =>
      this.request<RegistryView, ProblemDetails>({
        path: `/api/v1/registries/rename`,
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
     * @tags Registries
     * @name GetRegistry
     * @summary Get a Registry
     * @request GET:/api/v1/registries/{id}
     * @secure
     * @response `200` `AuthorizedRegistryView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getRegistry: (id: string, params: RequestParams = {}) =>
      this.request<AuthorizedRegistryView, ProblemDetails>({
        path: `/api/v1/registries/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Registries
     * @name UpdateRegistry
     * @summary Update a Registry
     * @request PATCH:/api/v1/registries/{id}
     * @secure
     * @response `200` `RegistryView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateRegistry: (
      id: string,
      data: RegistryPatch,
      params: RequestParams = {},
    ) =>
      this.request<RegistryView, ProblemDetails>({
        path: `/api/v1/registries/${id}`,
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
     * @tags Registries
     * @name GetRegistryConfig
     * @summary Get Registry configuration
     * @request GET:/api/v1/registries/{id}/_cfg
     * @secure
     * @response `200` `RegistryConfigResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getRegistryConfig: (id: string, params: RequestParams = {}) =>
      this.request<RegistryConfigResponse, ProblemDetails>({
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
     * @summary Update Registry metadata
     * @request PATCH:/api/v1/registries/{id}/_metadata
     * @secure
     * @response `200` `RegistryView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateRegistryMetadata: (
      id: string,
      data: PatchResourceMetadataInput,
      params: RequestParams = {},
    ) =>
      this.request<RegistryView, ProblemDetails>({
        path: `/api/v1/registries/${id}/_metadata`,
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
     * @tags Registries
     * @name GetRegistryTags
     * @summary Get Registry tags
     * @request GET:/api/v1/registries/{id}/tags
     * @secure
     * @response `200` `ResourceTagsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getRegistryTags: (id: string, params: RequestParams = {}) =>
      this.request<ResourceTagsResponse, ProblemDetails>({
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
     * @summary Replace Registry tags
     * @request PUT:/api/v1/registries/{id}/tags
     * @secure
     * @response `200` `ResourceTagsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    replaceRegistryTags: (
      id: string,
      data: ReplaceResourceTagsInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceTagsResponse, ProblemDetails>({
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
     * @tags ResourceBindings
     * @name GetGlobalResourceBindings
     * @summary Get global resource bindings
     * @request GET:/api/v1/resourceBindings/global
     * @secure
     * @response `200` `GlobalBindingsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getGlobalResourceBindings: (params: RequestParams = {}) =>
      this.request<GlobalBindingsResponse, ProblemDetails>({
        path: `/api/v1/resourceBindings/global`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name UpdateGlobalResourceBinding
     * @summary Update a global resource binding
     * @request PATCH:/api/v1/resourceBindings/global
     * @secure
     * @response `200` `ResourceBindingsView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateGlobalResourceBinding: (
      data: ResourceBindingInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceBindingsView, ProblemDetails>({
        path: `/api/v1/resourceBindings/global`,
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
     * @tags ResourceBindings
     * @name CreateGlobalResourceBinding
     * @summary Create a global resource binding
     * @request POST:/api/v1/resourceBindings/global
     * @secure
     * @response `200` `ResourceBindingsView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createGlobalResourceBinding: (
      data: NewResourceBinding,
      params: RequestParams = {},
    ) =>
      this.request<ResourceBindingsView, ProblemDetails>({
        path: `/api/v1/resourceBindings/global`,
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
     * @tags ResourceBindings
     * @name DeleteGlobalResourceBinding
     * @summary Delete a global resource binding
     * @request DELETE:/api/v1/resourceBindings/global/{id}
     * @secure
     * @response `200` `ResourceBindingsView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    deleteGlobalResourceBinding: (id: string, params: RequestParams = {}) =>
      this.request<ResourceBindingsView, ProblemDetails>({
        path: `/api/v1/resourceBindings/global/${id}`,
        method: "DELETE",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name ListSecretProviders
     * @summary List Secret providers
     * @request GET:/api/v1/resourceBindings/secret-providers
     * @secure
     * @response `200` `SecretProvidersResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listSecretProviders: (params: RequestParams = {}) =>
      this.request<SecretProvidersResponse, ProblemDetails>({
        path: `/api/v1/resourceBindings/secret-providers`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name CreateVaultKvV2SecretProvider
     * @summary Create a Vault-compatible KV v2 Secret provider
     * @request POST:/api/v1/resourceBindings/secret-providers/vault-kv2
     * @secure
     * @response `200` `SecretProviderView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createVaultKvV2SecretProvider: (
      data: SecretProviderInput,
      params: RequestParams = {},
    ) =>
      this.request<SecretProviderView, ProblemDetails>({
        path: `/api/v1/resourceBindings/secret-providers/vault-kv2`,
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
     * @tags ResourceBindings
     * @name TestVaultKvV2SecretProviderConnection
     * @summary Test a Vault-compatible KV v2 Secret provider connection
     * @request POST:/api/v1/resourceBindings/secret-providers/vault-kv2/test
     * @secure
     * @response `200` `SecretTestResult` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    testVaultKvV2SecretProviderConnection: (
      data: TestSecretProviderInput,
      params: RequestParams = {},
    ) =>
      this.request<SecretTestResult, ProblemDetails>({
        path: `/api/v1/resourceBindings/secret-providers/vault-kv2/test`,
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
     * @tags ResourceBindings
     * @name UpdateVaultKvV2SecretProvider
     * @summary Update a Vault-compatible KV v2 Secret provider
     * @request PATCH:/api/v1/resourceBindings/secret-providers/vault-kv2/{id}
     * @secure
     * @response `200` `SecretProviderView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateVaultKvV2SecretProvider: (
      id: string,
      data: SecretProviderPatch,
      params: RequestParams = {},
    ) =>
      this.request<SecretProviderView, ProblemDetails>({
        path: `/api/v1/resourceBindings/secret-providers/vault-kv2/${id}`,
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
     * @tags ResourceBindings
     * @name DeleteSecretProvider
     * @summary Delete a Secret provider
     * @request DELETE:/api/v1/resourceBindings/secret-providers/{id}
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    deleteSecretProvider: (id: string, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/resourceBindings/secret-providers/${id}`,
        method: "DELETE",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name ListSecretDefinitions
     * @summary List Secret definitions
     * @request GET:/api/v1/resourceBindings/secrets
     * @secure
     * @response `200` `SecretDefinitionsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listSecretDefinitions: (
      query?: {
        scope?: ResourceBindingScope;
        /** @format uuid */
        resourceId?: string;
        targetResourceType?: ResourceType;
        /** @format uuid */
        targetResourceId?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<SecretDefinitionsResponse, ProblemDetails>({
        path: `/api/v1/resourceBindings/secrets`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name CreateInternalSecret
     * @summary Create an internal encrypted Secret
     * @request POST:/api/v1/resourceBindings/secrets
     * @secure
     * @response `200` `SecretDefinitionView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createInternalSecret: (
      data: InternalSecretInput,
      query?: {
        scope?: string;
        /** @format uuid */
        resourceId?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<SecretDefinitionView, ProblemDetails>({
        path: `/api/v1/resourceBindings/secrets`,
        method: "POST",
        query: query,
        body: data,
        secure: true,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name CreateExternalSecret
     * @summary Create an external Secret definition
     * @request POST:/api/v1/resourceBindings/secrets/external
     * @secure
     * @response `200` `SecretDefinitionView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createExternalSecret: (
      data: ExternalSecretInput,
      params: RequestParams = {},
    ) =>
      this.request<SecretDefinitionView, ProblemDetails>({
        path: `/api/v1/resourceBindings/secrets/external`,
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
     * @tags ResourceBindings
     * @name TestExternalSecret
     * @summary Test an external Secret reference
     * @request POST:/api/v1/resourceBindings/secrets/external/test
     * @secure
     * @response `200` `SecretTestResult` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    testExternalSecret: (
      data: TestExternalSecretInput,
      params: RequestParams = {},
    ) =>
      this.request<SecretTestResult, ProblemDetails>({
        path: `/api/v1/resourceBindings/secrets/external/test`,
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
     * @tags ResourceBindings
     * @name UpdateExternalSecret
     * @summary Update an external Secret definition
     * @request PATCH:/api/v1/resourceBindings/secrets/external/{id}
     * @secure
     * @response `200` `SecretDefinitionView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateExternalSecret: (
      id: string,
      data: ExternalSecretPatch,
      params: RequestParams = {},
    ) =>
      this.request<SecretDefinitionView, ProblemDetails>({
        path: `/api/v1/resourceBindings/secrets/external/${id}`,
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
     * @tags ResourceBindings
     * @name DeleteSecretDefinition
     * @summary Delete an unused Secret definition
     * @request DELETE:/api/v1/resourceBindings/secrets/{id}
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    deleteSecretDefinition: (id: string, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/resourceBindings/secrets/${id}`,
        method: "DELETE",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags ResourceBindings
     * @name GetResourceBindings
     * @summary Get resource bindings
     * @request GET:/api/v1/resourceBindings/{scope}/{resourceId}
     * @secure
     * @response `200` `ResourceBindingsView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getResourceBindings: (
      scope: string,
      resourceId: string,
      params: RequestParams = {},
    ) =>
      this.request<ResourceBindingsView, ProblemDetails>({
        path: `/api/v1/resourceBindings/${scope}/${resourceId}`,
        method: "GET",
        secure: true,
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
     * @secure
     * @response `200` `ResourceBindingsView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateResourceBinding: (
      scope: string,
      resourceId: string,
      data: ResourceBindingInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceBindingsView, ProblemDetails>({
        path: `/api/v1/resourceBindings/${scope}/${resourceId}`,
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
     * @tags ResourceBindings
     * @name CreateResourceBinding
     * @summary Create a resource binding
     * @request POST:/api/v1/resourceBindings/{scope}/{resourceId}
     * @secure
     * @response `200` `ResourceBindingsView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createResourceBinding: (
      scope: string,
      resourceId: string,
      data: NewResourceBinding,
      params: RequestParams = {},
    ) =>
      this.request<ResourceBindingsView, ProblemDetails>({
        path: `/api/v1/resourceBindings/${scope}/${resourceId}`,
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
     * @tags ResourceBindings
     * @name DeleteResourceBinding
     * @summary Delete a resource binding
     * @request DELETE:/api/v1/resourceBindings/{scope}/{resourceId}/{id}
     * @secure
     * @response `200` `ResourceBindingsView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    deleteResourceBinding: (
      scope: string,
      resourceId: string,
      id: string,
      params: RequestParams = {},
    ) =>
      this.request<ResourceBindingsView, ProblemDetails>({
        path: `/api/v1/resourceBindings/${scope}/${resourceId}/${id}`,
        method: "DELETE",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Roles
     * @name DeleteRoles
     * @summary Delete Roles
     * @request DELETE:/api/v1/roles
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    deleteRoles: (data: DeleteRolesRequest, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @name ListRoles
     * @summary Get all Roles
     * @request GET:/api/v1/roles
     * @secure
     * @response `200` `RolesResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listRoles: (params: RequestParams = {}) =>
      this.request<RolesResponse, ProblemDetails>({
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
     * @summary Create a Role
     * @request POST:/api/v1/roles
     * @secure
     * @response `200` `RoleView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createRole: (data: CreateRoleRequest, params: RequestParams = {}) =>
      this.request<RoleView, ProblemDetails>({
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
     * @name GetPermissionMatrix
     * @summary Get permission matrix
     * @request GET:/api/v1/roles/permissions/matrix
     * @response `200` `Record<string,PermissionMatrixViewItem>` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @tags Roles
     * @name RenameRole
     * @summary Rename a Role
     * @request POST:/api/v1/roles/rename
     * @secure
     * @response `200` `RoleView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    renameRole: (data: RenameRoleRequest, params: RequestParams = {}) =>
      this.request<RoleView, ProblemDetails>({
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
     * @name GetRole
     * @summary Get a Role by ID
     * @request GET:/api/v1/roles/{id}
     * @secure
     * @response `200` `RoleView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getRole: (id: string, params: RequestParams = {}) =>
      this.request<RoleView, ProblemDetails>({
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
     * @summary Update Role permissions
     * @request PATCH:/api/v1/roles/{id}/permissions
     * @secure
     * @response `200` `RoleView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateRolePermissions: (
      id: string,
      data: PatchRolePermissionsRequest,
      params: RequestParams = {},
    ) =>
      this.request<RoleView, ProblemDetails>({
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
     * @tags Search
     * @name GlobalSearch
     * @summary Search authorized resources
     * @request GET:/api/v1/search
     * @secure
     * @response `200` `GlobalSearchResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    globalSearch: (
      query: {
        q: string;
        types?: string;
        /**
         * @format int32
         * @min 1
         * @max 10
         * @default 5
         */
        limitPerType?: number;
      },
      params: RequestParams = {},
    ) =>
      this.request<GlobalSearchResponse, ProblemDetails>({
        path: `/api/v1/search`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ServiceAccounts
     * @name ArchiveServiceAccounts
     * @summary Archive Service Accounts
     * @request DELETE:/api/v1/serviceAccounts
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    archiveServiceAccounts: (
      data: ArchiveServiceAccountsRequest,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/serviceAccounts`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags ServiceAccounts
     * @name ListServiceAccounts
     * @summary List Service Accounts
     * @request GET:/api/v1/serviceAccounts
     * @secure
     * @response `200` `ServiceAccountsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listServiceAccounts: (
      query?: {
        /**
         * @format int64
         * @min 1
         * @default 1
         */
        Page?: number;
        /**
         * @format int64
         * @min 1
         * @default 50
         */
        PageSize?: number;
        Name?: string;
        /** @default false */
        IncludeArchived?: boolean;
      },
      params: RequestParams = {},
    ) =>
      this.request<ServiceAccountsResponse, ProblemDetails>({
        path: `/api/v1/serviceAccounts`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ServiceAccounts
     * @name CreateServiceAccount
     * @summary Create a Service Account
     * @request POST:/api/v1/serviceAccounts
     * @secure
     * @response `200` `ServiceAccountView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createServiceAccount: (
      data: CreateServiceAccountRequest,
      params: RequestParams = {},
    ) =>
      this.request<ServiceAccountView, ProblemDetails>({
        path: `/api/v1/serviceAccounts`,
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
     * @tags ServiceAccounts
     * @name GetServiceAccountLimits
     * @summary Get Service Account limits
     * @request GET:/api/v1/serviceAccounts/limits
     * @secure
     * @response `200` `ServiceAccountLimitsView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getServiceAccountLimits: (params: RequestParams = {}) =>
      this.request<ServiceAccountLimitsView, ProblemDetails>({
        path: `/api/v1/serviceAccounts/limits`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ServiceAccounts
     * @name RenameServiceAccount
     * @summary Rename a Service Account
     * @request POST:/api/v1/serviceAccounts/rename
     * @secure
     * @response `200` `ServiceAccountView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    renameServiceAccount: (
      data: RenameServiceAccountRequest,
      params: RequestParams = {},
    ) =>
      this.request<ServiceAccountView, ProblemDetails>({
        path: `/api/v1/serviceAccounts/rename`,
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
     * @tags ServiceAccounts
     * @name GetServiceAccount
     * @summary Get a Service Account
     * @request GET:/api/v1/serviceAccounts/{id}
     * @secure
     * @response `200` `ServiceAccountDetailResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getServiceAccount: (id: string, params: RequestParams = {}) =>
      this.request<ServiceAccountDetailResponse, ProblemDetails>({
        path: `/api/v1/serviceAccounts/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ServiceAccounts
     * @name UpdateServiceAccount
     * @summary Update a Service Account
     * @request PATCH:/api/v1/serviceAccounts/{id}
     * @secure
     * @response `200` `ServiceAccountView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateServiceAccount: (
      id: string,
      data: UpdateServiceAccountRequest,
      params: RequestParams = {},
    ) =>
      this.request<ServiceAccountView, ProblemDetails>({
        path: `/api/v1/serviceAccounts/${id}`,
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
     * @tags ServiceAccounts
     * @name AddServiceAccountResourceAccess
     * @summary Add a resource override to a Service Account
     * @request POST:/api/v1/serviceAccounts/{id}/resource-accesses
     * @secure
     * @response `200` `ServiceAccountView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    addServiceAccountResourceAccess: (
      id: string,
      data: AddServiceAccountResourceAccessRequest,
      params: RequestParams = {},
    ) =>
      this.request<ServiceAccountView, ProblemDetails>({
        path: `/api/v1/serviceAccounts/${id}/resource-accesses`,
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
     * @tags ServiceAccounts
     * @name RemoveServiceAccountResourceAccess
     * @summary Remove a resource override from a Service Account
     * @request DELETE:/api/v1/serviceAccounts/{id}/resource-accesses/{resourceAccessId}
     * @secure
     * @response `200` `ServiceAccountView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    removeServiceAccountResourceAccess: (
      id: string,
      resourceAccessId: string,
      params: RequestParams = {},
    ) =>
      this.request<ServiceAccountView, ProblemDetails>({
        path: `/api/v1/serviceAccounts/${id}/resource-accesses/${resourceAccessId}`,
        method: "DELETE",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ServiceAccounts
     * @name AddServiceAccountRole
     * @summary Assign a Role to a Service Account
     * @request POST:/api/v1/serviceAccounts/{id}/roles
     * @secure
     * @response `200` `ServiceAccountView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    addServiceAccountRole: (
      id: string,
      data: AddServiceAccountRoleRequest,
      params: RequestParams = {},
    ) =>
      this.request<ServiceAccountView, ProblemDetails>({
        path: `/api/v1/serviceAccounts/${id}/roles`,
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
     * @tags ServiceAccounts
     * @name RemoveServiceAccountRole
     * @summary Remove a Role from a Service Account
     * @request DELETE:/api/v1/serviceAccounts/{id}/roles/{roleId}
     * @secure
     * @response `200` `ServiceAccountView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    removeServiceAccountRole: (
      id: string,
      roleId: string,
      params: RequestParams = {},
    ) =>
      this.request<ServiceAccountView, ProblemDetails>({
        path: `/api/v1/serviceAccounts/${id}/roles/${roleId}`,
        method: "DELETE",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ServiceAccounts
     * @name ListServiceAccountTokens
     * @summary List Service Account tokens
     * @request GET:/api/v1/serviceAccounts/{id}/tokens
     * @secure
     * @response `200` `ServiceAccountTokensResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listServiceAccountTokens: (
      id: string,
      query?: {
        /**
         * @format int64
         * @min 1
         * @default 1
         */
        Page?: number;
        /**
         * @format int64
         * @min 1
         * @default 50
         */
        PageSize?: number;
      },
      params: RequestParams = {},
    ) =>
      this.request<ServiceAccountTokensResponse, ProblemDetails>({
        path: `/api/v1/serviceAccounts/${id}/tokens`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags ServiceAccounts
     * @name CreateServiceAccountToken
     * @summary Create a Service Account token
     * @request POST:/api/v1/serviceAccounts/{id}/tokens
     * @secure
     * @response `201` `CreatedServiceAccountTokenView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createServiceAccountToken: (
      id: string,
      data: CreateServiceAccountTokenRequest,
      params: RequestParams = {},
    ) =>
      this.request<CreatedServiceAccountTokenView, ProblemDetails>({
        path: `/api/v1/serviceAccounts/${id}/tokens`,
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
     * @tags ServiceAccounts
     * @name RevokeServiceAccountToken
     * @summary Revoke a Service Account token
     * @request DELETE:/api/v1/serviceAccounts/{id}/tokens/{tokenId}
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    revokeServiceAccountToken: (
      id: string,
      tokenId: string,
      params: RequestParams = {},
    ) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/serviceAccounts/${id}/tokens/${tokenId}`,
        method: "DELETE",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags ServiceAccounts
     * @name ListServiceAccountUsages
     * @summary List Service Account execution usages
     * @request GET:/api/v1/serviceAccounts/{id}/usages
     * @secure
     * @response `200` `(RunAsActorUsageView)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listServiceAccountUsages: (id: string, params: RequestParams = {}) =>
      this.request<RunAsActorUsageView[], ProblemDetails>({
        path: `/api/v1/serviceAccounts/${id}/usages`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Setup
     * @name InitializeCitadel
     * @summary Initialize Citadel
     * @request POST:/api/v1/setup/initialize
     * @response `200` `LoginResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    initializeCitadel: (
      data: InitializeCitadelRequest,
      params: RequestParams = {},
    ) =>
      this.request<LoginResponse, ProblemDetails>({
        path: `/api/v1/setup/initialize`,
        method: "POST",
        body: data,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Setup
     * @name GetSetupStatus
     * @summary Get setup status
     * @request GET:/api/v1/setup/status
     * @response `200` `SetupStatusView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    getSetupStatus: (params: RequestParams = {}) =>
      this.request<SetupStatusView, ProblemDetails>({
        path: `/api/v1/setup/status`,
        method: "GET",
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name DeleteStacks
     * @summary Delete Stacks
     * @request DELETE:/api/v1/stacks
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    deleteStacks: (data: string[], params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @name ListStacks
     * @summary List authorized Stacks
     * @request GET:/api/v1/stacks
     * @secure
     * @response `200` `StacksView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listStacks: (
      query?: {
        tags?: string[];
        /** @format uuid */
        platformId?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<StacksView, ProblemDetails>({
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
     * @summary Create a Stack
     * @request POST:/api/v1/stacks
     * @secure
     * @response `200` `StackView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createStack: (data: CreateStackInput, params: RequestParams = {}) =>
      this.request<StackView, ProblemDetails>({
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
     * @name ApplyStack
     * @summary Apply a Stack and stream progress
     * @request POST:/api/v1/stacks/apply
     * @secure
     * @response `200` `(StackStreamItem)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    applyStack: (data: ApplyStackInput, params: RequestParams = {}) =>
      this.request<StackStreamItem[], ProblemDetails>({
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
     * @name PauseStacks
     * @summary Pause Stacks
     * @request POST:/api/v1/stacks/pause
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    pauseStacks: (data: string[], params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @name PreflightSwarmStack
     * @summary Validate Docker Swarm Stack compatibility
     * @request POST:/api/v1/stacks/preflight/swarm
     * @secure
     * @response `200` `SwarmStackCompatibilityReport` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    preflightSwarmStack: (
      data: SwarmPreflightInput,
      params: RequestParams = {},
    ) =>
      this.request<SwarmStackCompatibilityReport, ProblemDetails>({
        path: `/api/v1/stacks/preflight/swarm`,
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
     * @name RenameStack
     * @summary Rename a Stack
     * @request POST:/api/v1/stacks/rename
     * @secure
     * @response `200` `StackView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    renameStack: (data: RenameStackInput, params: RequestParams = {}) =>
      this.request<StackView, ProblemDetails>({
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
     * @name RestartStacks
     * @summary Restart Stacks
     * @request POST:/api/v1/stacks/restart
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    restartStacks: (data: string[], params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @name ResumeStacks
     * @summary Resume Stacks
     * @request POST:/api/v1/stacks/resume
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    resumeStacks: (data: string[], params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @name RollbackStack
     * @summary Roll back a Stack and stream progress
     * @request POST:/api/v1/stacks/rollback
     * @secure
     * @response `200` `(StackStreamItem)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    rollbackStack: (data: RollbackStackInput, params: RequestParams = {}) =>
      this.request<StackStreamItem[], ProblemDetails>({
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
     * @name StartStacks
     * @summary Start Stacks
     * @request POST:/api/v1/stacks/start
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    startStacks: (data: string[], params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @name StopStacks
     * @summary Stop Stacks
     * @request POST:/api/v1/stacks/stop
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    stopStacks: (data: string[], params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @name UpdateStack
     * @summary Update Stack configuration
     * @request PATCH:/api/v1/stacks/{id}
     * @secure
     * @response `200` `StackView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateStack: (
      id: string,
      data: PatchStackInput,
      params: RequestParams = {},
    ) =>
      this.request<StackView, ProblemDetails>({
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
     * @summary Update Stack metadata
     * @request PATCH:/api/v1/stacks/{id}/_metadata
     * @secure
     * @response `200` `StackView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateStackMetadata: (
      id: string,
      data: PatchStackMetadataInput,
      params: RequestParams = {},
    ) =>
      this.request<StackView, ProblemDetails>({
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
     * @name GetStack
     * @summary Get a Stack
     * @request GET:/api/v1/stacks/{stackId}
     * @secure
     * @response `200` `StackView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getStack: (stackId: string, params: RequestParams = {}) =>
      this.request<StackView, ProblemDetails>({
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
     * @name GetStackConfig
     * @summary Get Stack configuration
     * @request GET:/api/v1/stacks/{stackId}/_cfg
     * @secure
     * @response `200` `StackConfigView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getStackConfig: (stackId: string, params: RequestParams = {}) =>
      this.request<StackConfigView, ProblemDetails>({
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
     * @summary Preview Stack Backup volumes
     * @request GET:/api/v1/stacks/{stackId}/backup-source-preview
     * @secure
     * @response `200` `BackupSourcePreview` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    getStackBackupSourcePreview: (
      stackId: string,
      params: RequestParams = {},
    ) =>
      this.request<BackupSourcePreview, ProblemDetails>({
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
     * @name CheckStackUpdates
     * @summary Check a Stack source for updates
     * @request POST:/api/v1/stacks/{stackId}/check-updates
     * @secure
     * @response `200` `StackView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    checkStackUpdates: (stackId: string, params: RequestParams = {}) =>
      this.request<StackView, ProblemDetails>({
        path: `/api/v1/stacks/${stackId}/check-updates`,
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
     * @summary Inspect a Stack Container with sensitive environment values redacted
     * @request GET:/api/v1/stacks/{stackId}/containers/{containerId}/inspect
     * @secure
     * @response `200` `ContainerInspectionView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    inspectStackContainer: (
      stackId: string,
      containerId: string,
      params: RequestParams = {},
    ) =>
      this.request<ContainerInspectionView, ProblemDetails>({
        path: `/api/v1/stacks/${stackId}/containers/${containerId}/inspect`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Stacks
     * @name GetContainersData
     * @summary Get Stack runtime containers
     * @request GET:/api/v1/stacks/{stackId}/data
     * @secure
     * @response `200` `ContainerRuntimeListView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getContainersData: (stackId: string, params: RequestParams = {}) =>
      this.request<ContainerRuntimeListView, ProblemDetails>({
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
     * @name GetStackDrift
     * @summary Get Stack drift
     * @request GET:/api/v1/stacks/{stackId}/drift
     * @secure
     * @response `200` `StackDriftReport` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getStackDrift: (stackId: string, params: RequestParams = {}) =>
      this.request<StackDriftReport, ProblemDetails>({
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
     * @summary Update Stack drift policy
     * @request PUT:/api/v1/stacks/{stackId}/drift-policy
     * @secure
     * @response `200` `StackView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @name GetStackDuplicateDraft
     * @summary Build a Stack duplicate draft
     * @request GET:/api/v1/stacks/{stackId}/duplicate-draft
     * @secure
     * @response `200` `StackDuplicateDraftView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getStackDuplicateDraft: (stackId: string, params: RequestParams = {}) =>
      this.request<StackDuplicateDraftView, ProblemDetails>({
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
     * @name ReconcileStack
     * @summary Reconcile safe Stack drift
     * @request POST:/api/v1/stacks/{stackId}/reconcile
     * @secure
     * @response `200` `StackReconciliationResult` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @name ListStackReleases
     * @summary List previous healthy Stack releases
     * @request GET:/api/v1/stacks/{stackId}/releases
     * @secure
     * @response `200` `StackReleasesView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listStackReleases: (stackId: string, params: RequestParams = {}) =>
      this.request<StackReleasesView, ProblemDetails>({
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
     * @name GetStackStats
     * @summary Get Stack Container statistics
     * @request GET:/api/v1/stacks/{stackId}/stats
     * @secure
     * @response `200` `StackHistory` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getStackStats: (
      stackId: string,
      query?: {
        hours?: StatsHours;
      },
      params: RequestParams = {},
    ) =>
      this.request<StackHistory, ProblemDetails>({
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
     * @name GetStackTags
     * @summary Get Stack tags
     * @request GET:/api/v1/stacks/{stackId}/tags
     * @secure
     * @response `200` `ResourceTagsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getStackTags: (stackId: string, params: RequestParams = {}) =>
      this.request<ResourceTagsResponse, ProblemDetails>({
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
     * @summary Replace Stack tags
     * @request PUT:/api/v1/stacks/{stackId}/tags
     * @secure
     * @response `200` `ResourceTagsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    replaceStackTags: (
      stackId: string,
      data: ReplaceResourceTagsInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceTagsResponse, ProblemDetails>({
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
     * @tags SwarmServices
     * @name DeleteSwarmServices
     * @summary Delete managed Docker Swarm Services
     * @request DELETE:/api/v1/swarmServices
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    deleteSwarmServices: (data: string[], params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/swarmServices`,
        method: "DELETE",
        body: data,
        secure: true,
        type: ContentType.Json,
        ...params,
      }),

    /**
     * No description
     *
     * @tags SwarmServices
     * @name ListManagedSwarmServices
     * @summary List managed Docker Swarm Services
     * @request GET:/api/v1/swarmServices
     * @secure
     * @response `200` `ManagedSwarmServicesView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listManagedSwarmServices: (
      query?: {
        tags?: string[];
        /** @format uuid */
        platformId?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<ManagedSwarmServicesView, ProblemDetails>({
        path: `/api/v1/swarmServices`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags SwarmServices
     * @name CreateSwarmService
     * @summary Create a managed Docker Swarm Service
     * @request POST:/api/v1/swarmServices
     * @secure
     * @response `200` `ManagedSwarmServiceView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createSwarmService: (
      data: CreateSwarmServiceInput,
      params: RequestParams = {},
    ) =>
      this.request<ManagedSwarmServiceView, ProblemDetails>({
        path: `/api/v1/swarmServices`,
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
     * @tags SwarmServices
     * @name RenameSwarmService
     * @summary Rename a managed Docker Swarm Service
     * @request POST:/api/v1/swarmServices/rename
     * @secure
     * @response `200` `ManagedSwarmServiceView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    renameSwarmService: (
      data: RenameSwarmServiceInput,
      params: RequestParams = {},
    ) =>
      this.request<ManagedSwarmServiceView, ProblemDetails>({
        path: `/api/v1/swarmServices/rename`,
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
     * @tags SwarmServices
     * @name GetManagedSwarmService
     * @summary Get a managed Docker Swarm Service
     * @request GET:/api/v1/swarmServices/{id}
     * @secure
     * @response `200` `ManagedSwarmServiceView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getManagedSwarmService: (id: string, params: RequestParams = {}) =>
      this.request<ManagedSwarmServiceView, ProblemDetails>({
        path: `/api/v1/swarmServices/${id}`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags SwarmServices
     * @name UpdateSwarmService
     * @summary Update managed Docker Swarm Service configuration
     * @request PATCH:/api/v1/swarmServices/{id}
     * @secure
     * @response `200` `ManagedSwarmServiceView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateSwarmService: (
      id: string,
      data: UpdateSwarmServiceInput,
      params: RequestParams = {},
    ) =>
      this.request<ManagedSwarmServiceView, ProblemDetails>({
        path: `/api/v1/swarmServices/${id}`,
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
     * @tags SwarmServices
     * @name UpdateSwarmServiceMetadata
     * @summary Update managed Service metadata
     * @request PATCH:/api/v1/swarmServices/{id}/_metadata
     * @secure
     * @response `200` `ManagedSwarmServiceView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateSwarmServiceMetadata: (
      id: string,
      data: ServiceMetadataInput,
      params: RequestParams = {},
    ) =>
      this.request<ManagedSwarmServiceView, ProblemDetails>({
        path: `/api/v1/swarmServices/${id}/_metadata`,
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
     * @tags SwarmServices
     * @name ApplySwarmService
     * @summary Apply a managed Docker Swarm Service and stream progress
     * @request POST:/api/v1/swarmServices/{id}/apply
     * @secure
     * @response `200` `(SwarmServiceProgressItem)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    applySwarmService: (id: string, params: RequestParams = {}) =>
      this.request<SwarmServiceProgressItem[], ProblemDetails>({
        path: `/api/v1/swarmServices/${id}/apply`,
        method: "POST",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags SwarmServices
     * @name GetSwarmServiceBackupSourcePreview
     * @summary Preview Service Backup volumes
     * @request GET:/api/v1/swarmServices/{id}/backup-source-preview
     * @secure
     * @response `200` `BackupSourcePreview` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    getSwarmServiceBackupSourcePreview: (
      id: string,
      params: RequestParams = {},
    ) =>
      this.request<BackupSourcePreview, ProblemDetails>({
        path: `/api/v1/swarmServices/${id}/backup-source-preview`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags SwarmServices
     * @name CheckSwarmServiceUpdates
     * @summary Check the applied Service image for updates
     * @request POST:/api/v1/swarmServices/{id}/check-updates
     * @secure
     * @response `200` `ManagedSwarmServiceView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `default` `ProblemDetails` Request failed
     */
    checkSwarmServiceUpdates: (id: string, params: RequestParams = {}) =>
      this.request<ManagedSwarmServiceView, ProblemDetails>({
        path: `/api/v1/swarmServices/${id}/check-updates`,
        method: "POST",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags SwarmServices
     * @name GetSwarmServiceDuplicateDraft
     * @summary Prepare a managed Service duplicate
     * @request GET:/api/v1/swarmServices/{id}/duplicate-draft
     * @secure
     * @response `200` `SwarmServiceDuplicateDraftView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getSwarmServiceDuplicateDraft: (id: string, params: RequestParams = {}) =>
      this.request<SwarmServiceDuplicateDraftView, ProblemDetails>({
        path: `/api/v1/swarmServices/${id}/duplicate-draft`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags SwarmServices
     * @name ForceUpdateSwarmService
     * @summary Force a managed Docker Swarm Service task update and stream progress
     * @request POST:/api/v1/swarmServices/{id}/force-update
     * @secure
     * @response `200` `(SwarmServiceProgressItem)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    forceUpdateSwarmService: (id: string, params: RequestParams = {}) =>
      this.request<SwarmServiceProgressItem[], ProblemDetails>({
        path: `/api/v1/swarmServices/${id}/force-update`,
        method: "POST",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags SwarmServices
     * @name InspectManagedSwarmService
     * @summary Inspect the deployed managed Service
     * @request GET:/api/v1/swarmServices/{id}/inspect
     * @secure
     * @response `200` `ServiceInspectionView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    inspectManagedSwarmService: (id: string, params: RequestParams = {}) =>
      this.request<ServiceInspectionView, ProblemDetails>({
        path: `/api/v1/swarmServices/${id}/inspect`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags SwarmServices
     * @name GetManagedSwarmServiceLogs
     * @summary Read managed Service logs
     * @request GET:/api/v1/swarmServices/{id}/logs
     * @secure
     * @response `200` `LogSnapshot` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `502` `ProblemDetails` Bad Gateway
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    getManagedSwarmServiceLogs: (
      id: string,
      query?: {
        /**
         * @format int32
         * @min 1
         * @max 200
         * @default 100
         */
        tail?: number;
      },
      params: RequestParams = {},
    ) =>
      this.request<LogSnapshot, ProblemDetails>({
        path: `/api/v1/swarmServices/${id}/logs`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags SwarmServices
     * @name ScaleSwarmService
     * @summary Scale a managed Docker Swarm Service and stream progress
     * @request POST:/api/v1/swarmServices/{id}/scale
     * @secure
     * @response `200` `(SwarmServiceProgressItem)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    scaleSwarmService: (
      id: string,
      data: ScaleSwarmServiceInput,
      params: RequestParams = {},
    ) =>
      this.request<SwarmServiceProgressItem[], ProblemDetails>({
        path: `/api/v1/swarmServices/${id}/scale`,
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
     * @tags SwarmServices
     * @name GetSwarmServiceTags
     * @summary Get SwarmService tags
     * @request GET:/api/v1/swarmServices/{id}/tags
     * @secure
     * @response `200` `ResourceTagsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getSwarmServiceTags: (id: string, params: RequestParams = {}) =>
      this.request<ResourceTagsResponse, ProblemDetails>({
        path: `/api/v1/swarmServices/${id}/tags`,
        method: "GET",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags SwarmServices
     * @name ReplaceSwarmServiceTags
     * @summary Replace SwarmService tags
     * @request PUT:/api/v1/swarmServices/{id}/tags
     * @secure
     * @response `200` `ResourceTagsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    replaceSwarmServiceTags: (
      id: string,
      data: ReplaceResourceTagsInput,
      params: RequestParams = {},
    ) =>
      this.request<ResourceTagsResponse, ProblemDetails>({
        path: `/api/v1/swarmServices/${id}/tags`,
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
     * @tags Tags
     * @name ListTags
     * @summary List resource tags
     * @request GET:/api/v1/tags
     * @secure
     * @response `200` `TagsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listTags: (params: RequestParams = {}) =>
      this.request<TagsResponse, ProblemDetails>({
        path: `/api/v1/tags`,
        method: "GET",
        secure: true,
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
     * @response `200` `TagView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createTag: (data: NewTag, params: RequestParams = {}) =>
      this.request<TagView, ProblemDetails>({
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
     * @name DeleteTag
     * @summary Delete a resource tag
     * @request DELETE:/api/v1/tags/{id}
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
     * @tags Tags
     * @name PatchTag
     * @summary Update a resource tag
     * @request PATCH:/api/v1/tags/{id}
     * @secure
     * @response `200` `TagView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    patchTag: (id: string, data: TagPatch, params: RequestParams = {}) =>
      this.request<TagView, ProblemDetails>({
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
     * @tags Teams
     * @name DeleteTeams
     * @summary Delete Teams
     * @request DELETE:/api/v1/teams
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    deleteTeams: (data: DeleteTeamsRequest, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @name ListTeams
     * @summary Get all Teams
     * @request GET:/api/v1/teams
     * @secure
     * @response `200` `TeamsResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listTeams: (
      query?: {
        Name?: string;
        /**
         * @format int32
         * @min 1
         * @default 1
         */
        Page?: number;
        /**
         * @format int32
         * @min 1
         * @max 500
         * @default 50
         */
        PageSize?: number;
      },
      params: RequestParams = {},
    ) =>
      this.request<TeamsResponse, ProblemDetails>({
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
     * @summary Create a Team
     * @request POST:/api/v1/teams
     * @secure
     * @response `200` `TeamView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createTeam: (data: CreateTeamRequest, params: RequestParams = {}) =>
      this.request<TeamView, ProblemDetails>({
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
     * @name RenameTeam
     * @summary Rename a Team
     * @request POST:/api/v1/teams/rename
     * @secure
     * @response `200` `TeamView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    renameTeam: (data: RenameTeamRequest, params: RequestParams = {}) =>
      this.request<TeamView, ProblemDetails>({
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
     * @tags Teams
     * @name SearchTeams
     * @summary Search Teams for assignment
     * @request GET:/api/v1/teams/search
     * @secure
     * @response `200` `(TeamSearchItemView)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    searchTeams: (
      query?: {
        Query?: string;
        /**
         * @format int32
         * @min 1
         * @max 50
         * @default 20
         */
        Limit?: number;
      },
      params: RequestParams = {},
    ) =>
      this.request<TeamSearchItemView[], ProblemDetails>({
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
     * @summary Get a Team by ID
     * @request GET:/api/v1/teams/{id}
     * @secure
     * @response `200` `TeamView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getTeam: (id: string, params: RequestParams = {}) =>
      this.request<TeamView, ProblemDetails>({
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
     * @summary Update a Team
     * @request PATCH:/api/v1/teams/{id}
     * @secure
     * @response `200` `TeamView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateTeam: (
      id: string,
      data: PatchTeamRequest,
      params: RequestParams = {},
    ) =>
      this.request<TeamView, ProblemDetails>({
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
     * @name AddTeamMember
     * @summary Add an Actor to a Team
     * @request POST:/api/v1/teams/{id}/members
     * @secure
     * @response `200` `TeamView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    addTeamMember: (
      id: string,
      data: AddTeamMemberRequest,
      params: RequestParams = {},
    ) =>
      this.request<TeamView, ProblemDetails>({
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
     * @summary Remove an Actor from a Team
     * @request DELETE:/api/v1/teams/{id}/members/{memberActorId}
     * @secure
     * @response `200` `TeamView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    removeTeamMember: (
      id: string,
      memberActorId: string,
      params: RequestParams = {},
    ) =>
      this.request<TeamView, ProblemDetails>({
        path: `/api/v1/teams/${id}/members/${memberActorId}`,
        method: "DELETE",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Teams
     * @name RemoveTeamResourceAccess
     * @summary Remove a resource override from a Team
     * @request DELETE:/api/v1/teams/{id}/resource-accesses
     * @secure
     * @response `200` `TeamView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    removeTeamResourceAccess: (
      id: string,
      data: ResourceAccessInput,
      params: RequestParams = {},
    ) =>
      this.request<TeamView, ProblemDetails>({
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
     * @name AddTeamResourceAccess
     * @summary Add a resource override to a Team
     * @request POST:/api/v1/teams/{id}/resource-accesses
     * @secure
     * @response `200` `TeamView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    addTeamResourceAccess: (
      id: string,
      data: ResourceAccessInput,
      params: RequestParams = {},
    ) =>
      this.request<TeamView, ProblemDetails>({
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
     * @name AddTeamRole
     * @summary Assign a Role to a Team
     * @request POST:/api/v1/teams/{id}/roles
     * @secure
     * @response `200` `TeamView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    addTeamRole: (
      id: string,
      data: AddTeamRoleRequest,
      params: RequestParams = {},
    ) =>
      this.request<TeamView, ProblemDetails>({
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
     * @summary Remove a Role from a Team
     * @request DELETE:/api/v1/teams/{id}/roles/{roleId}
     * @secure
     * @response `200` `TeamView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    removeTeamRole: (id: string, roleId: string, params: RequestParams = {}) =>
      this.request<TeamView, ProblemDetails>({
        path: `/api/v1/teams/${id}/roles/${roleId}`,
        method: "DELETE",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Users
     * @name DeleteUsers
     * @summary Delete Users
     * @request DELETE:/api/v1/users
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    deleteUsers: (data: DeleteUsersRequest, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @name ListUsers
     * @summary Get all Users
     * @request GET:/api/v1/users
     * @secure
     * @response `200` `UsersResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    listUsers: (
      query?: {
        Name?: string;
        /**
         * @format int32
         * @min 1
         * @default 1
         */
        Page?: number;
        /**
         * @format int32
         * @min 1
         * @max 500
         * @default 50
         */
        PageSize?: number;
      },
      params: RequestParams = {},
    ) =>
      this.request<UsersResponse, ProblemDetails>({
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
     * @summary Create a User
     * @request POST:/api/v1/users
     * @secure
     * @response `200` `UserView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createUser: (data: CreateUserRequest, params: RequestParams = {}) =>
      this.request<UserView, ProblemDetails>({
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
     * @name RenameUser
     * @summary Rename a User
     * @request POST:/api/v1/users/rename
     * @secure
     * @response `200` `UserView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    renameUser: (data: RenameUserRequest, params: RequestParams = {}) =>
      this.request<UserView, ProblemDetails>({
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
     * @name SearchUsers
     * @summary Search Users for assignment
     * @request GET:/api/v1/users/search
     * @secure
     * @response `200` `(UserSearchItemView)[]` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    searchUsers: (
      query?: {
        Query?: string;
        /**
         * @format int32
         * @min 1
         * @max 50
         * @default 20
         */
        Limit?: number;
      },
      params: RequestParams = {},
    ) =>
      this.request<UserSearchItemView[], ProblemDetails>({
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
     * @summary Get a User by ID
     * @request GET:/api/v1/users/{id}
     * @secure
     * @response `200` `UserView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getUser: (id: string, params: RequestParams = {}) =>
      this.request<UserView, ProblemDetails>({
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
     * @summary Update a User
     * @request PATCH:/api/v1/users/{id}
     * @secure
     * @response `200` `UserView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    updateUser: (
      id: string,
      data: PatchUserRequest,
      params: RequestParams = {},
    ) =>
      this.request<UserView, ProblemDetails>({
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
     * @name ResetUserMfa
     * @summary Reset a user's MFA
     * @request DELETE:/api/v1/users/{id}/mfa
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    resetUserMfa: (id: string, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
        path: `/api/v1/users/${id}/mfa`,
        method: "DELETE",
        secure: true,
        ...params,
      }),

    /**
     * No description
     *
     * @tags Users
     * @name RemoveUserResourceAccess
     * @summary Remove a resource override from a User
     * @request DELETE:/api/v1/users/{id}/resource-accesses
     * @secure
     * @response `200` `UserView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    removeUserResourceAccess: (
      id: string,
      data: ResourceAccessInput,
      params: RequestParams = {},
    ) =>
      this.request<UserView, ProblemDetails>({
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
     * @name AddUserResourceAccess
     * @summary Add a resource override to a User
     * @request POST:/api/v1/users/{id}/resource-accesses
     * @secure
     * @response `200` `UserView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    addUserResourceAccess: (
      id: string,
      data: ResourceAccessInput,
      params: RequestParams = {},
    ) =>
      this.request<UserView, ProblemDetails>({
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
     * @name AddUserRole
     * @summary Assign a Role to a User
     * @request POST:/api/v1/users/{id}/roles
     * @secure
     * @response `200` `UserView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    addUserRole: (
      id: string,
      data: AddUserRoleRequest,
      params: RequestParams = {},
    ) =>
      this.request<UserView, ProblemDetails>({
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
     * @summary Remove a Role from a User
     * @request DELETE:/api/v1/users/{id}/roles/{roleId}
     * @secure
     * @response `200` `UserView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    removeUserRole: (id: string, roleId: string, params: RequestParams = {}) =>
      this.request<UserView, ProblemDetails>({
        path: `/api/v1/users/${id}/roles/${roleId}`,
        method: "DELETE",
        secure: true,
        format: "json",
        ...params,
      }),

    /**
     * No description
     *
     * @tags Volumes
     * @name DeleteVolumes
     * @summary Delete Volumes
     * @request DELETE:/api/v1/volumes
     * @secure
     * @response `204` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    deleteVolumes: (data: DeleteVolumesInput, params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
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
     * @tags Volumes
     * @name CreateVolume
     * @summary Create a Volume
     * @request POST:/api/v1/volumes
     * @secure
     * @response `200` `VolumeView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    createVolume: (data: CreateVolumeInput, params: RequestParams = {}) =>
      this.request<VolumeView, ProblemDetails>({
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
     * @name ListVolumes
     * @summary List Platform Volumes
     * @request GET:/api/v1/volumes/{platformId}
     * @secure
     * @response `200` `VolumesResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
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
      this.request<VolumesResponse, ProblemDetails>({
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
     * @summary Inspect a Platform Volume
     * @request GET:/api/v1/volumes/{platformId}/{name}
     * @secure
     * @response `200` `VolumeView` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    inspectVolume: (
      platformId: string,
      name: string,
      query?: {
        dockerNodeId?: string;
      },
      params: RequestParams = {},
    ) =>
      this.request<VolumeView, ProblemDetails>({
        path: `/api/v1/volumes/${platformId}/${name}`,
        method: "GET",
        query: query,
        secure: true,
        format: "json",
        ...params,
      }),
  };
  health = {
    /**
     * No description
     *
     * @tags Diagnostics
     * @name GetHealth
     * @summary Process liveness
     * @request GET:/health
     * @response `200` `HealthResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getHealth: (params: RequestParams = {}) =>
      this.request<HealthResponse, ProblemDetails>({
        path: `/health`,
        method: "GET",
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
     * @response `202` `WebhookResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `401` `ProblemDetails` Unauthorized
     * @response `403` `ProblemDetails` Forbidden
     * @response `404` `ProblemDetails` Not Found
     * @response `409` `ProblemDetails` Conflict
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    receiveWebhook: (
      authType: string,
      resourceType: string,
      id: string,
      execution: string,
      data: any,
      params: RequestParams = {},
    ) =>
      this.request<WebhookResponse, ProblemDetails>({
        path: `/listener/${authType}/${resourceType}/${id}/${execution}`,
        method: "POST",
        body: data,
        type: ContentType.Json,
        format: "json",
        ...params,
      }),
  };
  metrics = {
    /**
     * No description
     *
     * @tags Diagnostics
     * @name GetMetrics
     * @summary OpenMetrics diagnostics
     * @request GET:/metrics
     * @response `200` `void` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `default` `ProblemDetails` Request failed
     */
    getMetrics: (params: RequestParams = {}) =>
      this.request<void, ProblemDetails>({
        path: `/metrics`,
        method: "GET",
        ...params,
      }),
  };
  ready = {
    /**
     * No description
     *
     * @tags Diagnostics
     * @name GetReadiness
     * @summary Dependency readiness
     * @request GET:/ready
     * @response `200` `ReadinessResponse` Success
     * @response `400` `ProblemDetails` Bad Request
     * @response `429` `ProblemDetails` Too Many Requests
     * @response `500` `ProblemDetails` Internal Server Error
     * @response `503` `ProblemDetails` Service Unavailable
     * @response `default` `ProblemDetails` Request failed
     */
    getReadiness: (params: RequestParams = {}) =>
      this.request<ReadinessResponse, ProblemDetails>({
        path: `/ready`,
        method: "GET",
        format: "json",
        ...params,
      }),
  };
}
