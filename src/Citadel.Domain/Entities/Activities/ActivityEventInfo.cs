using Domain.Contracts.Resources.Alerts;
using Domain.Contracts.Resources.Deployments;
using Domain.Contracts.Resources.Git;
using Domain.Contracts.Resources.Platforms;
using Domain.Contracts.Resources.Registries;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Automation;
using Domain.Entities.Builds;
using Domain.Entities.Licensing;
using Domain.Entities.Stacks;
using Domain.Entities.SwarmServices;
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
[JsonDerivedType(typeof(DeploymentAdopted), nameof(ActivityEventType.DeploymentAdopted))]
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
[JsonDerivedType(typeof(StackImported), nameof(ActivityEventType.StackImported))]
[JsonDerivedType(typeof(AlertRuleCreated), nameof(ActivityEventType.AlertRuleCreated))]
[JsonDerivedType(typeof(AlertRuleUpdated), nameof(ActivityEventType.AlertRuleUpdated))]
[JsonDerivedType(typeof(AlertRuleDeleted), nameof(ActivityEventType.AlertRuleDeleted))]
[JsonDerivedType(typeof(AlertRuleRenamed), nameof(ActivityEventType.AlertRuleRenamed))]
[JsonDerivedType(typeof(PlatformCreated), nameof(ActivityEventType.PlatformCreated))]
[JsonDerivedType(typeof(PlatformDeleted), nameof(ActivityEventType.PlatformDeleted))]
[JsonDerivedType(typeof(PlatformConnected), nameof(ActivityEventType.PlatformConnected))]
[JsonDerivedType(typeof(PlatformDisconnected), nameof(ActivityEventType.PlatformDisconnected))]
[JsonDerivedType(typeof(PlatformRenamed), nameof(ActivityEventType.PlatformRenamed))]
[JsonDerivedType(typeof(PlatformNodeAgentLifecycle), nameof(ActivityEventType.PlatformNodeAgentLifecycle))]
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
[JsonDerivedType(typeof(InitialAdministratorCreated), nameof(ActivityEventType.InitialAdministratorCreated))]
[JsonDerivedType(typeof(UserProfileUpdated), nameof(ActivityEventType.UserProfileUpdated))]
[JsonDerivedType(typeof(UserPreferencesUpdated), nameof(ActivityEventType.UserPreferencesUpdated))]
[JsonDerivedType(typeof(UserPasswordChanged), nameof(ActivityEventType.UserPasswordChanged))]
[JsonDerivedType(typeof(UserSessionRevoked), nameof(ActivityEventType.UserSessionRevoked))]
[JsonDerivedType(typeof(UserOtherSessionsRevoked), nameof(ActivityEventType.UserOtherSessionsRevoked))]
[JsonDerivedType(typeof(UserMfaEnabled), nameof(ActivityEventType.UserMfaEnabled))]
[JsonDerivedType(typeof(UserMfaDisabled), nameof(ActivityEventType.UserMfaDisabled))]
[JsonDerivedType(typeof(UserMfaVerificationFailed), nameof(ActivityEventType.UserMfaVerificationFailed))]
[JsonDerivedType(typeof(UserMfaRecoveryCodeUsed), nameof(ActivityEventType.UserMfaRecoveryCodeUsed))]
[JsonDerivedType(typeof(UserMfaRecoveryCodesRegenerated), nameof(ActivityEventType.UserMfaRecoveryCodesRegenerated))]
[JsonDerivedType(typeof(UserMfaResetByAdministrator), nameof(ActivityEventType.UserMfaResetByAdministrator))]
[JsonDerivedType(typeof(UserCreated), nameof(ActivityEventType.UserCreated))]
[JsonDerivedType(typeof(UserUpdated), nameof(ActivityEventType.UserUpdated))]
[JsonDerivedType(typeof(UserRenamed), nameof(ActivityEventType.UserRenamed))]
[JsonDerivedType(typeof(UserDeleted), nameof(ActivityEventType.UserDeleted))]
[JsonDerivedType(typeof(TeamCreated), nameof(ActivityEventType.TeamCreated))]
[JsonDerivedType(typeof(TeamUpdated), nameof(ActivityEventType.TeamUpdated))]
[JsonDerivedType(typeof(TeamRenamed), nameof(ActivityEventType.TeamRenamed))]
[JsonDerivedType(typeof(TeamDeleted), nameof(ActivityEventType.TeamDeleted))]
[JsonDerivedType(typeof(RoleCreated), nameof(ActivityEventType.RoleCreated))]
[JsonDerivedType(typeof(RoleUpdated), nameof(ActivityEventType.RoleUpdated))]
[JsonDerivedType(typeof(RoleRenamed), nameof(ActivityEventType.RoleRenamed))]
[JsonDerivedType(typeof(RoleDeleted), nameof(ActivityEventType.RoleDeleted))]
[JsonDerivedType(typeof(LicenseInstalled), nameof(ActivityEventType.LicenseInstalled))]
[JsonDerivedType(typeof(LicenseReplaced), nameof(ActivityEventType.LicenseReplaced))]
[JsonDerivedType(typeof(LicenseRemoved), nameof(ActivityEventType.LicenseRemoved))]
[JsonDerivedType(typeof(LicenseEnteredGracePeriod), nameof(ActivityEventType.LicenseEnteredGracePeriod))]
[JsonDerivedType(typeof(LicenseExpired), nameof(ActivityEventType.LicenseExpired))]
[JsonDerivedType(typeof(LicenseValidationFailed), nameof(ActivityEventType.LicenseValidationFailed))]
[JsonDerivedType(typeof(VolumeContentDownloaded), nameof(ActivityEventType.VolumeContentDownloaded))]
[JsonDerivedType(typeof(BuildCreated), nameof(ActivityEventType.BuildCreated))]
[JsonDerivedType(typeof(BuildUpdated), nameof(ActivityEventType.BuildUpdated))]
[JsonDerivedType(typeof(BuildRenamed), nameof(ActivityEventType.BuildRenamed))]
[JsonDerivedType(typeof(BuildDeleted), nameof(ActivityEventType.BuildDeleted))]
[JsonDerivedType(typeof(BuildRunQueued), nameof(ActivityEventType.BuildRunQueued))]
[JsonDerivedType(typeof(BuildRunStarted), nameof(ActivityEventType.BuildRunStarted))]
[JsonDerivedType(typeof(BuildRunSucceeded), nameof(ActivityEventType.BuildRunSucceeded))]
[JsonDerivedType(typeof(BuildRunFailed), nameof(ActivityEventType.BuildRunFailed))]
[JsonDerivedType(typeof(BuildRunTimedOut), nameof(ActivityEventType.BuildRunTimedOut))]
[JsonDerivedType(typeof(BuildRunCancelled), nameof(ActivityEventType.BuildRunCancelled))]
[JsonDerivedType(typeof(BuildWebhookReceived), nameof(ActivityEventType.BuildWebhookReceived))]
[JsonDerivedType(typeof(BuildAgentPoolCreated), nameof(ActivityEventType.BuildAgentPoolCreated))]
[JsonDerivedType(typeof(BuildAgentPoolUpdated), nameof(ActivityEventType.BuildAgentPoolUpdated))]
[JsonDerivedType(typeof(BuildAgentPoolRenamed), nameof(ActivityEventType.BuildAgentPoolRenamed))]
[JsonDerivedType(typeof(BuildAgentPoolDeleted), nameof(ActivityEventType.BuildAgentPoolDeleted))]
[JsonDerivedType(typeof(BuildAgentPoolTested), nameof(ActivityEventType.BuildAgentPoolTested))]
[JsonDerivedType(typeof(BackupPolicyCreated), nameof(ActivityEventType.BackupPolicyCreated))]
[JsonDerivedType(typeof(BackupPolicyUpdated), nameof(ActivityEventType.BackupPolicyUpdated))]
[JsonDerivedType(typeof(BackupPolicyRenamed), nameof(ActivityEventType.BackupPolicyRenamed))]
[JsonDerivedType(typeof(BackupPolicyArchived), nameof(ActivityEventType.BackupPolicyArchived))]
[JsonDerivedType(typeof(SwarmServiceCreated), nameof(ActivityEventType.SwarmServiceCreated))]
[JsonDerivedType(typeof(SwarmServiceAdopted), nameof(ActivityEventType.SwarmServiceAdopted))]
[JsonDerivedType(typeof(SwarmServiceUpdated), nameof(ActivityEventType.SwarmServiceUpdated))]
[JsonDerivedType(typeof(SwarmServiceRenamed), nameof(ActivityEventType.SwarmServiceRenamed))]
[JsonDerivedType(typeof(SwarmServiceDeleted), nameof(ActivityEventType.SwarmServiceDeleted))]
[JsonDerivedType(typeof(SwarmServiceApplied), nameof(ActivityEventType.SwarmServiceApplied))]
[JsonDerivedType(typeof(SwarmServiceScaled), nameof(ActivityEventType.SwarmServiceScaled))]
[JsonDerivedType(typeof(SwarmServiceForceUpdated), nameof(ActivityEventType.SwarmServiceForceUpdated))]
[JsonDerivedType(typeof(SwarmServiceOperationFailed), nameof(ActivityEventType.SwarmServiceOperationFailed))]
[JsonDerivedType(typeof(SwarmServiceDuplicated), nameof(ActivityEventType.SwarmServiceDuplicated))]
[JsonDerivedType(typeof(SwarmServiceWebhookReceived), nameof(ActivityEventType.SwarmServiceWebhookReceived))]
[JsonDerivedType(typeof(ServiceAccountCreated), nameof(ActivityEventType.ServiceAccountCreated))]
[JsonDerivedType(typeof(ServiceAccountUpdated), nameof(ActivityEventType.ServiceAccountUpdated))]
[JsonDerivedType(typeof(ServiceAccountRenamed), nameof(ActivityEventType.ServiceAccountRenamed))]
[JsonDerivedType(typeof(ServiceAccountEnabled), nameof(ActivityEventType.ServiceAccountEnabled))]
[JsonDerivedType(typeof(ServiceAccountDisabled), nameof(ActivityEventType.ServiceAccountDisabled))]
[JsonDerivedType(typeof(ServiceAccountArchived), nameof(ActivityEventType.ServiceAccountArchived))]
[JsonDerivedType(typeof(ServiceAccountTokenCreated), nameof(ActivityEventType.ServiceAccountTokenCreated))]
[JsonDerivedType(typeof(ServiceAccountTokenRevoked), nameof(ActivityEventType.ServiceAccountTokenRevoked))]

public abstract record ActivityEventInfo;

public sealed record ActivitySourceResource(
    ActivityResourceType ResourceType,
    Guid ResourceId,
    string ResourceName);

public sealed record ActivityChangedField(string Name, string? OldValue, string? NewValue);

public sealed record IdentityResourceAccessSnapshot(
    Hosting.Common.ResourceType ResourceType,
    Guid ResourceId,
    Hosting.Common.PermissionLevel PermissionLevel,
    int SpecificPermissions);

public sealed record UserActivitySnapshot(
    string Email,
    bool IsEnabled,
    IReadOnlyCollection<Guid> TeamIds,
    IReadOnlyCollection<Guid> RoleIds,
    IReadOnlyCollection<IdentityResourceAccessSnapshot> ResourceAccesses);

public sealed record TeamActivitySnapshot(
    bool IsEnabled,
    IReadOnlyCollection<Guid> MemberActorIds,
    IReadOnlyCollection<Guid> RoleIds,
    IReadOnlyCollection<IdentityResourceAccessSnapshot> ResourceAccesses);

public sealed record RolePermissionActivitySnapshot(
    Hosting.Common.ResourceType ResourceType,
    Hosting.Common.PermissionLevel PermissionLevel,
    int SpecificPermissions);

public sealed record RoleActivitySnapshot(
    RoleType RoleType,
    IReadOnlyCollection<RolePermissionActivitySnapshot> Permissions);

public sealed record UserCreated(UserActivitySnapshot User) : ActivityEventInfo;
public sealed record UserUpdated(UserActivitySnapshot OldUser, UserActivitySnapshot NewUser, bool PasswordChanged) : ActivityEventInfo;
public sealed record UserRenamed(string OldName, string NewName) : ActivityEventInfo;
public sealed record UserDeleted(UserActivitySnapshot User) : ActivityEventInfo;

public sealed record TeamCreated(TeamActivitySnapshot Team) : ActivityEventInfo;
public sealed record TeamUpdated(TeamActivitySnapshot OldTeam, TeamActivitySnapshot NewTeam) : ActivityEventInfo;
public sealed record TeamRenamed(string OldName, string NewName) : ActivityEventInfo;
public sealed record TeamDeleted(TeamActivitySnapshot Team) : ActivityEventInfo;

public sealed record RoleCreated(RoleActivitySnapshot Role) : ActivityEventInfo;
public sealed record RoleUpdated(RoleActivitySnapshot OldRole, RoleActivitySnapshot NewRole) : ActivityEventInfo;
public sealed record RoleRenamed(string OldName, string NewName) : ActivityEventInfo;
public sealed record RoleDeleted(RoleActivitySnapshot Role) : ActivityEventInfo;

public sealed record SwarmServiceActivitySnapshot(
    Guid Id,
    Guid PlatformId,
    string Name,
    string? Description,
    string DockerName,
    string? DockerServiceId,
    SwarmServiceSpec Spec);

public sealed record SwarmServiceCreated(SwarmServiceActivitySnapshot Service) : ActivityEventInfo;
public sealed record SwarmServiceAdopted(SwarmServiceActivitySnapshot Service, string DockerServiceId) : ActivityEventInfo;
public sealed record SwarmServiceDuplicated(
    SwarmServiceActivitySnapshot Service,
    ActivitySourceResource Source) : ActivityEventInfo;
public sealed record SwarmServiceUpdated(SwarmServiceActivitySnapshot OldService, SwarmServiceActivitySnapshot NewService) : ActivityEventInfo;
public sealed record SwarmServiceRenamed(string OldName, string NewName) : ActivityEventInfo;
public sealed record SwarmServiceDeleted(SwarmServiceActivitySnapshot Service) : ActivityEventInfo;
public sealed record SwarmServiceApplied(Guid OperationId, IReadOnlyList<string> Warnings) : ActivityEventInfo;
public sealed record SwarmServiceScaled(Guid OperationId, int Replicas, IReadOnlyList<string> Warnings) : ActivityEventInfo;
public sealed record SwarmServiceForceUpdated(Guid OperationId, IReadOnlyList<string> Warnings) : ActivityEventInfo;
public sealed record SwarmServiceOperationFailed(Guid OperationId, SwarmServiceOperationKind Kind, string Reason) : ActivityEventInfo;
public sealed record SwarmServiceWebhookReceived(
    Guid RequestId,
    string AuthType,
    string Execution,
    string Status,
    string? Reason,
    string? DeliveryId) : ActivityEventInfo;

public static class SwarmServiceActivityExtensions
{
    public static SwarmServiceActivitySnapshot ToActivitySnapshot(this SwarmService service) => new(
        service.Id,
        service.PlatformId,
        service.Name,
        service.Description,
        service.DockerName,
        service.DockerServiceId,
        service.Spec with
        {
            Webhook = service.Spec.Webhook is null
                ? null
                : service.Spec.Webhook with { Secret = null },
        });
}

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
public sealed record DeploymentAdopted(
    DeploymentSnapshot Deployment,
    string ContainerId,
    string ContainerName) : ActivityEventInfo;

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
public sealed record StackImported(
    StackSnapshot Stack,
    string ComposeProject,
    IReadOnlyList<string> ServiceNames) : ActivityEventInfo;
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
public sealed record PlatformNodeAgentLifecycle(
    SwarmNodeAgentOperationKind Kind,
    Guid OperationId,
    SwarmNodeAgentOperationState State,
    string? Message) : ActivityEventInfo;

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

public sealed record BuildProjectSnapshot(
    Guid Id,
    string Name,
    string? Description,
    bool Enabled,
    Guid GitRepositoryId,
    string Branch,
    string ContextPath,
    string DockerfilePath,
    string? Target,
    BuildProjectBuilderKind BuilderKind,
    Guid? PlatformId,
    Guid? BuildAgentPoolId,
    Guid RegistryId,
    string ImageRepository,
    IReadOnlyList<string> TagTemplates,
    BuildWebhookConfig? Webhook,
    int TimeoutSeconds,
    int RetentionRunCount,
    IReadOnlyList<BuildSecretSpec>? BuildSecrets = null);

public sealed record BuildCreated(BuildProjectSnapshot Build) : ActivityEventInfo;
public sealed record BuildUpdated(BuildProjectSnapshot OldBuild, BuildProjectSnapshot NewBuild) : ActivityEventInfo;
public sealed record BuildRenamed(string OldName, string NewName) : ActivityEventInfo;
public sealed record BuildDeleted(BuildProjectSnapshot Build) : ActivityEventInfo;
public sealed record BuildRunQueued(Guid RunId, BuildRunTrigger Trigger) : ActivityEventInfo;
public sealed record BuildRunStarted(Guid RunId, BuildRunTrigger Trigger) : ActivityEventInfo;
public sealed record BuildRunSucceeded(
    Guid RunId,
    BuildRunTrigger Trigger,
    int? ExitCode,
    long? DurationMs,
    string? ImageDigest) : ActivityEventInfo;
public sealed record BuildRunFailed(
    Guid RunId,
    BuildRunTrigger Trigger,
    BuildRunStatus Status,
    int? ExitCode,
    long? DurationMs,
    string? ErrorMessage) : ActivityEventInfo;
public sealed record BuildRunTimedOut(
    Guid RunId,
    BuildRunTrigger Trigger,
    long? DurationMs,
    string? ErrorMessage) : ActivityEventInfo;
public sealed record BuildRunCancelled(Guid RunId, BuildRunTrigger Trigger) : ActivityEventInfo;
public sealed record BuildWebhookReceived(
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

public sealed record BuildAgentPoolCreated(BuildAgentPoolSnapshot Pool) : ActivityEventInfo;
public sealed record BuildAgentPoolUpdated(BuildAgentPoolSnapshot OldPool, BuildAgentPoolSnapshot NewPool) : ActivityEventInfo;
public sealed record BuildAgentPoolRenamed(string OldName, string NewName) : ActivityEventInfo;
public sealed record BuildAgentPoolDeleted(BuildAgentPoolSnapshot Pool) : ActivityEventInfo;
public sealed record BuildAgentPoolTested(BuildAgentPoolSnapshot Pool, BuildAgentPoolValidationStatus Status, string? Message) : ActivityEventInfo;

public sealed record BackupPolicyActivitySnapshot(
    Guid Id,
    string Name,
    string? Description,
    string SourceType,
    string SourceKey,
    Guid BackupRepositoryId,
    bool Enabled,
    string? Cron,
    string? TimeZone,
    bool WebhookEnabled,
    int KeepLastSuccessful,
    int TimeoutSeconds,
    bool AlertOnFailure,
    Guid RunAsActorId);

public sealed record BackupPolicyCreated(BackupPolicyActivitySnapshot Policy) : ActivityEventInfo;
public sealed record BackupPolicyUpdated(
    BackupPolicyActivitySnapshot OldPolicy,
    BackupPolicyActivitySnapshot NewPolicy) : ActivityEventInfo;
public sealed record BackupPolicyRenamed(string OldName, string NewName) : ActivityEventInfo;
public sealed record BackupPolicyArchived(BackupPolicyActivitySnapshot Policy) : ActivityEventInfo;

public sealed record InitialAdministratorCreated(Guid UserId, string UserName, string Mode) : ActivityEventInfo;
public sealed record UserProfileUpdated(IReadOnlyCollection<ActivityChangedField> Changes) : ActivityEventInfo;
public sealed record UserPreferencesUpdated(IReadOnlyCollection<ActivityChangedField> Changes) : ActivityEventInfo;
public sealed record UserPasswordChanged() : ActivityEventInfo;
public sealed record UserSessionRevoked(Guid SessionId) : ActivityEventInfo;
public sealed record UserOtherSessionsRevoked(int Count) : ActivityEventInfo;
public sealed record UserMfaEnabled() : ActivityEventInfo;
public sealed record UserMfaDisabled() : ActivityEventInfo;
public sealed record UserMfaVerificationFailed() : ActivityEventInfo;
public sealed record UserMfaRecoveryCodeUsed() : ActivityEventInfo;
public sealed record UserMfaRecoveryCodesRegenerated() : ActivityEventInfo;
public sealed record UserMfaResetByAdministrator(Guid TargetUserId) : ActivityEventInfo;

public sealed record LicenseActivitySnapshot(
    int? Schema,
    string? LicenseId,
    string? ReplacedLicenseId,
    string? LicensedEdition,
    string EffectiveEdition,
    IReadOnlyList<LicenseCapability> EffectiveCapabilities,
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

public sealed record VolumeContentDownloaded(
    string VolumeName,
    string Path,
    bool IsDirectory,
    string FileName) : ActivityEventInfo;

public sealed record ServiceAccountActivitySnapshot(
    Guid Id,
    string Name,
    string? Description,
    bool IsEnabled,
    IReadOnlyCollection<Guid> TeamIds,
    IReadOnlyCollection<Guid> RoleIds,
    IReadOnlyCollection<ServiceAccountResourceAccessSnapshot> ResourceAccesses);

public sealed record ServiceAccountResourceAccessSnapshot(
    Hosting.Common.ResourceType ResourceType,
    Guid ResourceId,
    Hosting.Common.PermissionLevel PermissionLevel,
    int SpecificPermissions);

public sealed record ServiceAccountCreated(ServiceAccountActivitySnapshot Account) : ActivityEventInfo;
public sealed record ServiceAccountUpdated(
    ServiceAccountActivitySnapshot OldAccount,
    ServiceAccountActivitySnapshot NewAccount) : ActivityEventInfo;
public sealed record ServiceAccountRenamed(string OldName, string NewName) : ActivityEventInfo;
public sealed record ServiceAccountEnabled(Guid AccountId) : ActivityEventInfo;
public sealed record ServiceAccountDisabled(Guid AccountId) : ActivityEventInfo;
public sealed record ServiceAccountArchived(Guid AccountId) : ActivityEventInfo;
public sealed record ServiceAccountTokenCreated(
    Guid AccountId,
    Guid TokenId,
    string TokenName,
    string PublicHint,
    DateTime? ExpiresAtUtc) : ActivityEventInfo;
public sealed record ServiceAccountTokenRevoked(Guid AccountId, Guid TokenId, string PublicHint) : ActivityEventInfo;
