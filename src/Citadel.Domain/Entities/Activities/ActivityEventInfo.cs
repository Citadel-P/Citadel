using Domain.Contracts.Resources.Alerts;
using Domain.Contracts.Resources.Deployments;
using Domain.Contracts.Resources.Git;
using Domain.Contracts.Resources.Platforms;
using Domain.Contracts.Resources.Registries;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Automation;
using Domain.Entities.Licensing;
using Domain.Entities.Stacks;
using System.Diagnostics.CodeAnalysis;
using System.Text.Json.Serialization;

namespace Domain.Entities.Activities;

[JsonPolymorphic]
[DynamicallyAccessedMembers(DynamicallyAccessedMemberTypes.All)]
[JsonDerivedType(typeof(DeploymentCreated), nameof(ActivityEventType.DeploymentCreated))]
[JsonDerivedType(typeof(DeploymentDuplicated), nameof(ActivityEventType.DeploymentDuplicated))]
[JsonDerivedType(typeof(DeploymentUpdated), nameof(ActivityEventType.DeploymentUpdated))]
[JsonDerivedType(typeof(DeploymentRenamed), nameof(ActivityEventType.DeploymentRenamed))]
[JsonDerivedType(typeof(DeploymentDeleted), nameof(ActivityEventType.DeploymentDeleted))]
[JsonDerivedType(typeof(DeploymentStarted), nameof(ActivityEventType.DeploymentStarted))]
[JsonDerivedType(typeof(DeploymentStopped), nameof(ActivityEventType.DeploymentStopped))]
[JsonDerivedType(typeof(DeploymentPaused), nameof(ActivityEventType.DeploymentPaused))]
[JsonDerivedType(typeof(DeploymentApplied), nameof(ActivityEventType.DeploymentApplied))]
[JsonDerivedType(typeof(DeploymentDegraded), nameof(ActivityEventType.DeploymentDegraded))]
[JsonDerivedType(typeof(StackCreated), nameof(ActivityEventType.StackCreated))]
[JsonDerivedType(typeof(StackDuplicated), nameof(ActivityEventType.StackDuplicated))]
[JsonDerivedType(typeof(StackUpdated), nameof(ActivityEventType.StackUpdated))]
[JsonDerivedType(typeof(StackRenamed), nameof(ActivityEventType.StackRenamed))]
[JsonDerivedType(typeof(StackDeleted), nameof(ActivityEventType.StackDeleted))]
[JsonDerivedType(typeof(StackStarted), nameof(ActivityEventType.StackStarted))]
[JsonDerivedType(typeof(StackStopped), nameof(ActivityEventType.StackStopped))]
[JsonDerivedType(typeof(StackPaused), nameof(ActivityEventType.StackPaused))]
[JsonDerivedType(typeof(StackApplied), nameof(ActivityEventType.StackApplied))]
[JsonDerivedType(typeof(StackRollback), nameof(ActivityEventType.StackRollback))]
[JsonDerivedType(typeof(StackDegraded), nameof(ActivityEventType.StackDegraded))]
[JsonDerivedType(typeof(StackDriftDetected), nameof(ActivityEventType.StackDriftDetected))]
[JsonDerivedType(typeof(StackDriftResolved), nameof(ActivityEventType.StackDriftResolved))]
[JsonDerivedType(typeof(StackReconciliationAttempted), nameof(ActivityEventType.StackReconciliationAttempted))]
[JsonDerivedType(typeof(StackGitUpdateAvailable), nameof(ActivityEventType.StackGitUpdateAvailable))]
[JsonDerivedType(typeof(StackGitAutoUpdated), nameof(ActivityEventType.StackGitAutoUpdated))]
[JsonDerivedType(typeof(StackGitAutoDeployFailed), nameof(ActivityEventType.StackGitAutoDeployFailed))]
[JsonDerivedType(typeof(AlertRuleCreated), nameof(ActivityEventType.AlertRuleCreated))]
[JsonDerivedType(typeof(AlertRuleUpdated), nameof(ActivityEventType.AlertRuleUpdated))]
[JsonDerivedType(typeof(AlertRuleDeleted), nameof(ActivityEventType.AlertRuleDeleted))]
[JsonDerivedType(typeof(AlertRuleRenamed), nameof(ActivityEventType.AlertRuleRenamed))]
[JsonDerivedType(typeof(PlatformCreated), nameof(ActivityEventType.PlatformCreated))]
[JsonDerivedType(typeof(PlatformDeleted), nameof(ActivityEventType.PlatformDeleted))]
[JsonDerivedType(typeof(PlatformConnected), nameof(ActivityEventType.PlatformConnected))]
[JsonDerivedType(typeof(PlatformDisconnected), nameof(ActivityEventType.PlatformDisconnected))]
[JsonDerivedType(typeof(PlatformRenamed), nameof(ActivityEventType.PlatformRenamed))]
[JsonDerivedType(typeof(RegistryRenamed), nameof(ActivityEventType.RegistryRenamed))]
[JsonDerivedType(typeof(RegistryCreated), nameof(ActivityEventType.RegistryCreated))]
[JsonDerivedType(typeof(RegistryUpdated), nameof(ActivityEventType.RegistryUpdated))]
[JsonDerivedType(typeof(RegistryDeleted), nameof(ActivityEventType.RegistryDeleted))]
[JsonDerivedType(typeof(GitRepoCreated), nameof(ActivityEventType.GitRepoCreated))]
[JsonDerivedType(typeof(GitRepoUpdated), nameof(ActivityEventType.GitRepoUpdated))]
[JsonDerivedType(typeof(GitRepoRenamed), nameof(ActivityEventType.GitRepoRenamed))]
[JsonDerivedType(typeof(GitRepoDeleted), nameof(ActivityEventType.GitRepoDeleted))]
[JsonDerivedType(typeof(GitRepoCloned), nameof(ActivityEventType.GitRepoCloned))]
[JsonDerivedType(typeof(GitRepoPulled), nameof(ActivityEventType.GitRepoPulled))]
[JsonDerivedType(typeof(GitRepoWebhookReceived), nameof(ActivityEventType.GitRepoWebhookReceived))]
[JsonDerivedType(typeof(OidcProviderCreated), nameof(ActivityEventType.OidcProviderCreated))]
[JsonDerivedType(typeof(OidcProviderUpdated), nameof(ActivityEventType.OidcProviderUpdated))]
[JsonDerivedType(typeof(OidcProviderRenamed), nameof(ActivityEventType.OidcProviderRenamed))]
[JsonDerivedType(typeof(OidcProviderDeleted), nameof(ActivityEventType.OidcProviderDeleted))]
[JsonDerivedType(typeof(AutomationActionCreated), nameof(ActivityEventType.ActionCreated))]
[JsonDerivedType(typeof(AutomationActionUpdated), nameof(ActivityEventType.ActionUpdated))]
[JsonDerivedType(typeof(AutomationActionRenamed), nameof(ActivityEventType.ActionRenamed))]
[JsonDerivedType(typeof(AutomationActionDeleted), nameof(ActivityEventType.ActionDeleted))]
[JsonDerivedType(typeof(AutomationActionRunQueued), nameof(ActivityEventType.ActionRunQueued))]
[JsonDerivedType(typeof(AutomationActionRunStarted), nameof(ActivityEventType.ActionRunStarted))]
[JsonDerivedType(typeof(AutomationActionRunSucceeded), nameof(ActivityEventType.ActionRunSucceeded))]
[JsonDerivedType(typeof(AutomationActionRunFailed), nameof(ActivityEventType.ActionRunFailed))]
[JsonDerivedType(typeof(AutomationActionRunTimedOut), nameof(ActivityEventType.ActionRunTimedOut))]
[JsonDerivedType(typeof(AutomationActionRunCancelled), nameof(ActivityEventType.ActionRunCancelled))]
[JsonDerivedType(typeof(AutomationActionRunRejected), nameof(ActivityEventType.ActionRunRejected))]
[JsonDerivedType(typeof(StackWebhookReceived), nameof(ActivityEventType.StackWebhookReceived))]
[JsonDerivedType(typeof(UserProfileUpdated), nameof(ActivityEventType.UserProfileUpdated))]
[JsonDerivedType(typeof(UserPreferencesUpdated), nameof(ActivityEventType.UserPreferencesUpdated))]
[JsonDerivedType(typeof(UserPasswordChanged), nameof(ActivityEventType.UserPasswordChanged))]
[JsonDerivedType(typeof(UserSessionRevoked), nameof(ActivityEventType.UserSessionRevoked))]
[JsonDerivedType(typeof(UserOtherSessionsRevoked), nameof(ActivityEventType.UserOtherSessionsRevoked))]
[JsonDerivedType(typeof(LicenseInstalled), nameof(ActivityEventType.LicenseInstalled))]
[JsonDerivedType(typeof(LicenseReplaced), nameof(ActivityEventType.LicenseReplaced))]
[JsonDerivedType(typeof(LicenseRemoved), nameof(ActivityEventType.LicenseRemoved))]
[JsonDerivedType(typeof(LicenseEnteredGracePeriod), nameof(ActivityEventType.LicenseEnteredGracePeriod))]
[JsonDerivedType(typeof(LicenseExpired), nameof(ActivityEventType.LicenseExpired))]
[JsonDerivedType(typeof(LicenseValidationFailed), nameof(ActivityEventType.LicenseValidationFailed))]

public abstract record ActivityEventInfo;

public sealed record ActivitySourceResource(
    ActivityResourceType ResourceType,
    Guid ResourceId,
    string ResourceName);

public sealed record ActivityChangedField(string Name, string? OldValue, string? NewValue);

public sealed record DeploymentCreated(DeploymentSnapshot Deployment) : ActivityEventInfo;
public sealed record DeploymentDuplicated(DeploymentSnapshot Deployment, ActivitySourceResource Source) : ActivityEventInfo;
public sealed record DeploymentUpdated(DeploymentSnapshot OldDeployment, DeploymentSnapshot NewDeployment) : ActivityEventInfo;
public sealed record DeploymentRenamed(string OldName, string NewName) : ActivityEventInfo;
public sealed record DeploymentDeleted(DeploymentSnapshot Deployment) : ActivityEventInfo;
public sealed record DeploymentStarted(IEnumerable<string> ContainerIds) : ActivityEventInfo;
public sealed record DeploymentStopped(IEnumerable<string> ContainerIds) : ActivityEventInfo;
public sealed record DeploymentPaused(IEnumerable<string> ContainerIds) : ActivityEventInfo;
public sealed record DeploymentDegraded(string Reason) : ActivityEventInfo;
public sealed record DeploymentApplied(DeploymentSnapshot? Deployment, DeploymentResultSnapshot Result) : ActivityEventInfo;

public sealed record StackCreated(StackSnapshot Stack) : ActivityEventInfo;
public sealed record StackDuplicated(StackSnapshot Stack, ActivitySourceResource Source) : ActivityEventInfo;
public sealed record StackUpdated(StackSnapshot OldStack, StackSnapshot NewStack) : ActivityEventInfo;
public sealed record StackRenamed(string OldName, string NewName) : ActivityEventInfo;
public sealed record StackDeleted(StackSnapshot Stack) : ActivityEventInfo;
public sealed record StackStarted(IEnumerable<string> ContainerIds) : ActivityEventInfo;
public sealed record StackStopped(IEnumerable<string> ContainerIds) : ActivityEventInfo;
public sealed record StackPaused(IEnumerable<string> ContainerIds) : ActivityEventInfo;
public sealed record StackDegraded(string Reason) : ActivityEventInfo;
public sealed record StackDriftDetected(string Reason, string Fingerprint) : ActivityEventInfo;
public sealed record StackDriftResolved(string PreviousFingerprint) : ActivityEventInfo;
public sealed record StackReconciliationAttempted(
    StackReconciliationStatus Status,
    IReadOnlyList<StackReconciliationAction> Actions,
    string DriftFingerprint) : ActivityEventInfo;
public sealed record StackApplied(StackSnapshot? Stack, StackResultSnapshot Result) : ActivityEventInfo;
public sealed record StackRollback(StackSnapshot? OldStack, StackSnapshot? NewStack, StackResultSnapshot Result) : ActivityEventInfo;
public sealed record StackGitUpdateAvailable(
    string GitRepositoryName,
    string Branch,
    string CurrentCommitSha,
    string RemoteCommitSha) : ActivityEventInfo;
public sealed record StackGitAutoUpdated(
    string GitRepositoryName,
    string Branch,
    string PreviousCommitSha,
    string UpdatedCommitSha) : ActivityEventInfo;
public sealed record StackGitAutoDeployFailed(
    string GitRepositoryName,
    string Branch,
    string CurrentCommitSha,
    string RemoteCommitSha,
    string Reason) : ActivityEventInfo;
public sealed record StackWebhookReceived(
    Guid RequestId,
    string AuthType,
    string Execution,
    string Status,
    string? Reason,
    string? EventType,
    string? DeliveryId,
    string? Branch,
    string? CommitSha,
    string? RepositoryFullName,
    string? DispatchedBranch = null,
    string? DispatchedCommitSha = null) : ActivityEventInfo;


public sealed record AlertRuleCreated(AlertRuleSnapshot AlertRule) : ActivityEventInfo;
public sealed record AlertRuleUpdated(AlertRuleSnapshot OldRule, AlertRuleSnapshot NewRule) : ActivityEventInfo;
public sealed record AlertRuleDeleted(AlertRuleSnapshot AlertRule) : ActivityEventInfo;
public sealed record AlertRuleRenamed(string OldName, string NewName) : ActivityEventInfo;

public sealed record PlatformCreated(PlatformSnapshot Platform) : ActivityEventInfo;
public sealed record PlatformDeleted(PlatformSnapshot Platform) : ActivityEventInfo;
public sealed record PlatformConnected(PlatformSnapshot Platform, PlatformStatus PreviousStatus) : ActivityEventInfo;
public sealed record PlatformDisconnected(PlatformSnapshot Platform, PlatformStatus PreviousStatus) : ActivityEventInfo;
public sealed record PlatformRenamed(string OldName, string NewName) : ActivityEventInfo;

public sealed record RegistryRenamed(string OldName, string NewName) : ActivityEventInfo;
public sealed record RegistryDeleted(RegistrySnapshot Registry) : ActivityEventInfo;
public sealed record RegistryUpdated(RegistrySnapshot OldRegistry, RegistrySnapshot NewRegistry) : ActivityEventInfo;
public sealed record RegistryCreated(RegistrySnapshot Registry) : ActivityEventInfo;
public sealed record GitRepoCreated(GitRepositorySnapshot GitRepo) : ActivityEventInfo;
public sealed record GitRepoUpdated(GitRepositorySnapshot OldGitRepo, GitRepositorySnapshot NewGitRepo) : ActivityEventInfo;
public sealed record GitRepoRenamed(string OldName, string NewName) : ActivityEventInfo;
public sealed record GitRepoDeleted(GitRepositorySnapshot GitRepo) : ActivityEventInfo;
public sealed record GitRepoCloned(GitRepositorySnapshot GitRepo, RepoSyncResultSnapshot Result) : ActivityEventInfo;
public sealed record GitRepoPulled(GitRepositorySnapshot GitRepo, RepoSyncResultSnapshot Result) : ActivityEventInfo;
public sealed record GitRepoWebhookReceived(
    Guid RequestId,
    string AuthType,
    string Execution,
    string Status,
    string? Reason,
    string? EventType,
    string? DeliveryId,
    string? Branch,
    string? CommitSha,
    string? RepositoryFullName,
    string? DispatchedBranch = null,
    string? DispatchedCommitSha = null) : ActivityEventInfo;

public sealed record OidcProviderActivitySnapshot(
    Guid Id,
    string Name,
    string? Description,
    string DisplayName,
    string Issuer,
    string ClientId,
    string Scopes,
    bool Enabled,
    bool AutoProvisionUsers,
    bool AllowEmailAutoLink,
    bool RequireEmailVerified,
    string? AllowedEmailDomains,
    string? RequiredClaimName,
    string? RequiredClaimValues,
    Guid? DefaultRoleId);

public sealed record OidcProviderCreated(OidcProviderActivitySnapshot Provider) : ActivityEventInfo;
public sealed record OidcProviderUpdated(
    OidcProviderActivitySnapshot OldProvider,
    OidcProviderActivitySnapshot NewProvider) : ActivityEventInfo;
public sealed record OidcProviderRenamed(string OldName, string NewName) : ActivityEventInfo;
public sealed record OidcProviderDeleted(OidcProviderActivitySnapshot Provider) : ActivityEventInfo;

public sealed record AutomationActionSnapshot(
    Guid Id,
    string Name,
    string? Description,
    string Code,
    string DefaultArgsJson,
    bool Enabled,
    bool ScheduleEnabled,
    string? ScheduleCron,
    string ScheduleTimeZone,
    AutomationWebhookConfig? Webhook,
    int TimeoutSeconds,
    bool AlertOnFailure,
    Guid RunAsActorId);

public sealed record AutomationActionCreated(AutomationActionSnapshot Action) : ActivityEventInfo;
public sealed record AutomationActionUpdated(AutomationActionSnapshot OldAction, AutomationActionSnapshot NewAction) : ActivityEventInfo;
public sealed record AutomationActionRenamed(string OldName, string NewName) : ActivityEventInfo;
public sealed record AutomationActionDeleted(AutomationActionSnapshot Action) : ActivityEventInfo;
public sealed record AutomationActionRunQueued(Guid RunId, ActionRunTrigger Trigger) : ActivityEventInfo;
public sealed record AutomationActionRunStarted(Guid RunId, ActionRunTrigger Trigger) : ActivityEventInfo;
public sealed record AutomationActionRunSucceeded(
    Guid RunId,
    ActionRunTrigger Trigger,
    int? ExitCode,
    long? DurationMs) : ActivityEventInfo;
public sealed record AutomationActionRunFailed(
    Guid RunId,
    ActionRunTrigger Trigger,
    int? ExitCode,
    long? DurationMs,
    string? ErrorMessage) : ActivityEventInfo;
public sealed record AutomationActionRunTimedOut(
    Guid RunId,
    ActionRunTrigger Trigger,
    long? DurationMs,
    string? ErrorMessage) : ActivityEventInfo;
public sealed record AutomationActionRunCancelled(Guid RunId, ActionRunTrigger Trigger) : ActivityEventInfo;
public sealed record AutomationActionRunRejected(Guid RunId, ActionRunTrigger Trigger, string Reason) : ActivityEventInfo;

public sealed record UserProfileUpdated(IReadOnlyCollection<ActivityChangedField> Changes) : ActivityEventInfo;
public sealed record UserPreferencesUpdated(IReadOnlyCollection<ActivityChangedField> Changes) : ActivityEventInfo;
public sealed record UserPasswordChanged() : ActivityEventInfo;
public sealed record UserSessionRevoked(Guid SessionId) : ActivityEventInfo;
public sealed record UserOtherSessionsRevoked(int Count) : ActivityEventInfo;

public sealed record LicenseActivitySnapshot(
    string? LicenseId,
    string? ReplacedLicenseId,
    string Edition,
    string? CustomerId,
    string? CustomerName,
    string? Fingerprint,
    LicenseStatus Status,
    DateTimeOffset? ExpiresAt,
    DateTimeOffset? GraceUntil);

public sealed record LicenseInstalled(LicenseActivitySnapshot License) : ActivityEventInfo;
public sealed record LicenseReplaced(LicenseActivitySnapshot OldLicense, LicenseActivitySnapshot NewLicense) : ActivityEventInfo;
public sealed record LicenseRemoved(LicenseActivitySnapshot License) : ActivityEventInfo;
public sealed record LicenseEnteredGracePeriod(LicenseActivitySnapshot License) : ActivityEventInfo;
public sealed record LicenseExpired(LicenseActivitySnapshot License) : ActivityEventInfo;
public sealed record LicenseValidationFailed(string? Fingerprint, LicenseStatus Status, string? ErrorCode) : ActivityEventInfo;
