using Domain.Contracts.Interfaces;
using Domain.Entities.Identity;
using Domain.Entities.Platforms;

namespace Domain.Entities.Activities;

public sealed class ActivityEvent : IAuditedEntity
{
    public Guid Id { get; private set; } = Guid.CreateVersion7();
    public Guid? PlatformId { get; private set; }
    public Guid? ResourceId { get; private set; }
    public string ResourceName { get; private set; }
    public ActivityResourceType ResourceType { get; }
    public ActivityStatus Status { get; private set; }
    public ActivityEventType EventType { get; private set; }
    public ActivityEventInfo Info { get; private set; }

    #region IAuditedEntity
    public Guid CreatedByActorId { get; private set; }
    public DateTime CreatedAt { get; private set; }
    #endregion

    public Platform? Platform { get; private set; }
    public Actor? Actor { get; private set; }

    public ActivityEvent(
        Guid? platformId,
        Guid? resourceId,
        Guid actorId,
        string resourceName,
        ActivityEventType eventType,
        ActivityStatus status,
        ActivityEventInfo info)
    {
        if (platformId == Guid.Empty)
            throw new ArgumentException("PlatformId is required", nameof(platformId));

        if (resourceId == Guid.Empty)
            throw new ArgumentException("ResourceId is required", nameof(resourceId));

        if (actorId == Guid.Empty)
            throw new ArgumentException("ActorId is required", nameof(actorId));

        if (string.IsNullOrWhiteSpace(resourceName))
            throw new ArgumentException("ResourceName is required", nameof(resourceName));

        if (info != null && !IsValidInfoForEvent(eventType, info))
            throw new ArgumentException(
                $"EventInfo type '{info.GetType().Name}' does not match EventType '{eventType}'");

        PlatformId = platformId;
        ResourceId = resourceId;
        CreatedByActorId = actorId;
        ResourceName = resourceName;
        EventType = eventType;
        Info = info;
        Status = status;
        ResourceType = GetResourceType(eventType);
        CreatedAt = DateTime.UtcNow;
    }

    public static ActivityResourceType GetResourceType(ActivityEventType eventType)
    {
        return eventType switch
        {
            ActivityEventType.DeploymentCreated
            or ActivityEventType.DeploymentDuplicated
            or ActivityEventType.DeploymentUpdated
            or ActivityEventType.DeploymentRenamed
            or ActivityEventType.DeploymentDeleted
            or ActivityEventType.DeploymentStarted
            or ActivityEventType.DeploymentStopped
            or ActivityEventType.DeploymentPaused
            or ActivityEventType.DeploymentApplied
            or ActivityEventType.DeploymentDegraded
            or ActivityEventType.DeploymentAdopted
                => ActivityResourceType.Deployment,

            ActivityEventType.StackCreated
            or ActivityEventType.StackDuplicated
            or ActivityEventType.StackUpdated
            or ActivityEventType.StackRenamed
            or ActivityEventType.StackDeleted
            or ActivityEventType.StackStarted
            or ActivityEventType.StackStopped
            or ActivityEventType.StackPaused
            or ActivityEventType.StackApplied
            or ActivityEventType.StackRollback
            or ActivityEventType.StackDegraded
            or ActivityEventType.StackDriftDetected
            or ActivityEventType.StackDriftResolved
            or ActivityEventType.StackReconciliationAttempted
            or ActivityEventType.StackGitUpdateAvailable
            or ActivityEventType.StackGitAutoUpdated
            or ActivityEventType.StackGitAutoDeployFailed
            or ActivityEventType.StackWebhookReceived
            or ActivityEventType.StackImported
                => ActivityResourceType.Stack,

            ActivityEventType.PlatformCreated
            or ActivityEventType.PlatformDeleted
            or ActivityEventType.PlatformConnected
            or ActivityEventType.PlatformDisconnected
            or ActivityEventType.PlatformRenamed
                => ActivityResourceType.Platform,

            ActivityEventType.RegistryCreated
            or ActivityEventType.RegistryRenamed
            or ActivityEventType.RegistryUpdated
            or ActivityEventType.RegistryDeleted
                => ActivityResourceType.Registry,

            ActivityEventType.AlertRuleCreated
            or ActivityEventType.AlertRuleUpdated
            or ActivityEventType.AlertRuleDeleted
            or ActivityEventType.AlertRuleRenamed
                => ActivityResourceType.AlertRule,

            ActivityEventType.GitRepoCreated
            or ActivityEventType.GitRepoUpdated
            or ActivityEventType.GitRepoDeleted
            or ActivityEventType.GitRepoRenamed
            or ActivityEventType.GitRepoPulled
            or ActivityEventType.GitRepoCloned
            or ActivityEventType.GitRepoWebhookReceived
                => ActivityResourceType.GitRepository,

            ActivityEventType.OidcProviderCreated
            or ActivityEventType.OidcProviderUpdated
            or ActivityEventType.OidcProviderRenamed
            or ActivityEventType.OidcProviderDeleted
                => ActivityResourceType.OidcProvider,

            ActivityEventType.ActionCreated
            or ActivityEventType.ActionUpdated
            or ActivityEventType.ActionRenamed
            or ActivityEventType.ActionDeleted
            or ActivityEventType.ActionRunQueued
            or ActivityEventType.ActionRunStarted
            or ActivityEventType.ActionRunSucceeded
            or ActivityEventType.ActionRunFailed
            or ActivityEventType.ActionRunTimedOut
            or ActivityEventType.ActionRunCancelled
            or ActivityEventType.ActionRunRejected
                => ActivityResourceType.AutomationAction,

            ActivityEventType.InitialAdministratorCreated
            or ActivityEventType.UserProfileUpdated
            or ActivityEventType.UserPreferencesUpdated
            or ActivityEventType.UserPasswordChanged
            or ActivityEventType.UserSessionRevoked
            or ActivityEventType.UserOtherSessionsRevoked
            or ActivityEventType.UserMfaEnabled
            or ActivityEventType.UserMfaDisabled
            or ActivityEventType.UserMfaVerificationFailed
            or ActivityEventType.UserMfaRecoveryCodeUsed
            or ActivityEventType.UserMfaRecoveryCodesRegenerated
            or ActivityEventType.UserMfaResetByAdministrator
                => ActivityResourceType.User,

            ActivityEventType.LicenseInstalled
            or ActivityEventType.LicenseReplaced
            or ActivityEventType.LicenseRemoved
            or ActivityEventType.LicenseEnteredGracePeriod
            or ActivityEventType.LicenseExpired
            or ActivityEventType.LicenseValidationFailed
                => ActivityResourceType.License,

            ActivityEventType.VolumeContentDownloaded
                => ActivityResourceType.Volume,

            ActivityEventType.BuildCreated
            or ActivityEventType.BuildUpdated
            or ActivityEventType.BuildRenamed
            or ActivityEventType.BuildDeleted
            or ActivityEventType.BuildRunQueued
            or ActivityEventType.BuildRunStarted
            or ActivityEventType.BuildRunSucceeded
            or ActivityEventType.BuildRunFailed
            or ActivityEventType.BuildRunTimedOut
            or ActivityEventType.BuildRunCancelled
            or ActivityEventType.BuildWebhookReceived
                => ActivityResourceType.Build,

            ActivityEventType.BuildAgentPoolCreated
            or ActivityEventType.BuildAgentPoolUpdated
            or ActivityEventType.BuildAgentPoolRenamed
            or ActivityEventType.BuildAgentPoolDeleted
            or ActivityEventType.BuildAgentPoolTested
                => ActivityResourceType.BuildAgentPool,

            ActivityEventType.BackupPolicyCreated
            or ActivityEventType.BackupPolicyUpdated
            or ActivityEventType.BackupPolicyRenamed
            or ActivityEventType.BackupPolicyArchived
                => ActivityResourceType.BackupPolicy,

            ActivityEventType.SwarmServiceCreated
            or ActivityEventType.SwarmServiceUpdated
            or ActivityEventType.SwarmServiceRenamed
            or ActivityEventType.SwarmServiceDeleted
            or ActivityEventType.SwarmServiceApplied
            or ActivityEventType.SwarmServiceScaled
            or ActivityEventType.SwarmServiceForceUpdated
            or ActivityEventType.SwarmServiceOperationFailed
            or ActivityEventType.SwarmServiceDuplicated
                => ActivityResourceType.SwarmService,

            _ => throw new InvalidOperationException(
                $"EventType '{eventType}' does not map to a ResourceType.")
        };
    }

    public static ActivityEvent FromPersistence(
        Guid id,
        Guid? platformId,
        Guid? resourceId,
        string resourceName,
        ActivityResourceType resourceType,
        ActivityEventType eventType,
        ActivityStatus status,
        ActivityEventInfo info,
        Guid createdByActorId,
        DateTime createdAt,
        Platform? platform = null,
        Actor? actor = null)
    {
        if (info != null && !IsValidInfoForEvent(eventType, info))
            throw new ArgumentException(
                $"EventInfo type '{info.GetType().Name}' does not match EventType '{eventType}'");
        if (resourceType != GetResourceType(eventType))
            throw new ArgumentException(
                $"EventType {eventType} does not derive from ResourceType {resourceType}");
        return new ActivityEvent(
            platformId: platformId,
            resourceId: resourceId,
            actorId: createdByActorId,
            resourceName: resourceName,
            eventType: eventType,
            status: status,
            info: info)
        {
            Id = id,
            CreatedAt = createdAt,
            Platform = platform,
            Actor = actor
        };
    }

    public async Task<ActivityEvent> AssignActor(IUnitOfWork uow, CancellationToken ct)
    {
        var actor = await uow.Actors.GetById(CreatedByActorId, ct);
        Actor = actor;
        return this;
    }

    private static bool IsValidInfoForEvent(ActivityEventType type, ActivityEventInfo info)
    {
        return (type, info) switch
        {
            (ActivityEventType.DeploymentCreated, DeploymentCreated) => true,
            (ActivityEventType.DeploymentDuplicated, DeploymentDuplicated) => true,
            (ActivityEventType.DeploymentUpdated, DeploymentUpdated) => true,
            (ActivityEventType.DeploymentDeleted, DeploymentDeleted) => true,
            (ActivityEventType.DeploymentRenamed, DeploymentRenamed) => true,
            (ActivityEventType.DeploymentStarted, DeploymentStarted) => true,
            (ActivityEventType.DeploymentStopped, DeploymentStopped) => true,
            (ActivityEventType.DeploymentApplied, DeploymentApplied) => true,
            (ActivityEventType.DeploymentPaused, DeploymentPaused) => true,
            (ActivityEventType.DeploymentDegraded, DeploymentDegraded) => true,
            (ActivityEventType.DeploymentAdopted, DeploymentAdopted) => true,

            (ActivityEventType.StackCreated, StackCreated) => true,
            (ActivityEventType.StackDuplicated, StackDuplicated) => true,
            (ActivityEventType.StackUpdated, StackUpdated) => true,
            (ActivityEventType.StackDeleted, StackDeleted) => true,
            (ActivityEventType.StackRenamed, StackRenamed) => true,
            (ActivityEventType.StackStarted, StackStarted) => true,
            (ActivityEventType.StackStopped, StackStopped) => true,
            (ActivityEventType.StackApplied, StackApplied) => true,
            (ActivityEventType.StackRollback, StackRollback) => true,
            (ActivityEventType.StackPaused, StackPaused) => true,
            (ActivityEventType.StackDegraded, StackDegraded) => true,
            (ActivityEventType.StackDriftDetected, StackDriftDetected) => true,
            (ActivityEventType.StackDriftResolved, StackDriftResolved) => true,
            (ActivityEventType.StackReconciliationAttempted, StackReconciliationAttempted) => true,
            (ActivityEventType.StackGitUpdateAvailable, StackGitUpdateAvailable) => true,
            (ActivityEventType.StackGitAutoUpdated, StackGitAutoUpdated) => true,
            (ActivityEventType.StackGitAutoDeployFailed, StackGitAutoDeployFailed) => true,
            (ActivityEventType.StackWebhookReceived, StackWebhookReceived) => true,
            (ActivityEventType.StackImported, StackImported) => true,

            (ActivityEventType.AlertRuleCreated, AlertRuleCreated) => true,
            (ActivityEventType.AlertRuleUpdated, AlertRuleUpdated) => true,
            (ActivityEventType.AlertRuleDeleted, AlertRuleDeleted) => true,
            (ActivityEventType.AlertRuleRenamed, AlertRuleRenamed) => true,

            (ActivityEventType.PlatformCreated, PlatformCreated) => true,
            (ActivityEventType.PlatformDeleted, PlatformDeleted) => true,
            (ActivityEventType.PlatformConnected, PlatformConnected) => true,
            (ActivityEventType.PlatformDisconnected, PlatformDisconnected) => true,
            (ActivityEventType.PlatformRenamed, PlatformRenamed) => true,

            (ActivityEventType.RegistryRenamed, RegistryRenamed) => true,
            (ActivityEventType.RegistryDeleted, RegistryDeleted) => true,
            (ActivityEventType.RegistryCreated, RegistryCreated) => true,
            (ActivityEventType.RegistryUpdated, RegistryUpdated) => true,

            (ActivityEventType.GitRepoCreated, GitRepoCreated) => true,
            (ActivityEventType.GitRepoUpdated, GitRepoUpdated) => true,
            (ActivityEventType.GitRepoRenamed, GitRepoRenamed) => true,
            (ActivityEventType.GitRepoDeleted, GitRepoDeleted) => true,
            (ActivityEventType.GitRepoCloned, GitRepoCloned) => true,
            (ActivityEventType.GitRepoPulled, GitRepoPulled) => true,
            (ActivityEventType.GitRepoWebhookReceived, GitRepoWebhookReceived) => true,

            (ActivityEventType.OidcProviderCreated, OidcProviderCreated) => true,
            (ActivityEventType.OidcProviderUpdated, OidcProviderUpdated) => true,
            (ActivityEventType.OidcProviderRenamed, OidcProviderRenamed) => true,
            (ActivityEventType.OidcProviderDeleted, OidcProviderDeleted) => true,

            (ActivityEventType.ActionCreated, AutomationActionCreated) => true,
            (ActivityEventType.ActionUpdated, AutomationActionUpdated) => true,
            (ActivityEventType.ActionRenamed, AutomationActionRenamed) => true,
            (ActivityEventType.ActionDeleted, AutomationActionDeleted) => true,
            (ActivityEventType.ActionRunQueued, AutomationActionRunQueued) => true,
            (ActivityEventType.ActionRunStarted, AutomationActionRunStarted) => true,
            (ActivityEventType.ActionRunSucceeded, AutomationActionRunSucceeded) => true,
            (ActivityEventType.ActionRunFailed, AutomationActionRunFailed) => true,
            (ActivityEventType.ActionRunTimedOut, AutomationActionRunTimedOut) => true,
            (ActivityEventType.ActionRunCancelled, AutomationActionRunCancelled) => true,
            (ActivityEventType.ActionRunRejected, AutomationActionRunRejected) => true,

            (ActivityEventType.InitialAdministratorCreated, InitialAdministratorCreated) => true,
            (ActivityEventType.UserProfileUpdated, UserProfileUpdated) => true,
            (ActivityEventType.UserPreferencesUpdated, UserPreferencesUpdated) => true,
            (ActivityEventType.UserPasswordChanged, UserPasswordChanged) => true,
            (ActivityEventType.UserSessionRevoked, UserSessionRevoked) => true,
            (ActivityEventType.UserOtherSessionsRevoked, UserOtherSessionsRevoked) => true,
            (ActivityEventType.UserMfaEnabled, UserMfaEnabled) => true,
            (ActivityEventType.UserMfaDisabled, UserMfaDisabled) => true,
            (ActivityEventType.UserMfaVerificationFailed, UserMfaVerificationFailed) => true,
            (ActivityEventType.UserMfaRecoveryCodeUsed, UserMfaRecoveryCodeUsed) => true,
            (ActivityEventType.UserMfaRecoveryCodesRegenerated, UserMfaRecoveryCodesRegenerated) => true,
            (ActivityEventType.UserMfaResetByAdministrator, UserMfaResetByAdministrator) => true,

            (ActivityEventType.LicenseInstalled, LicenseInstalled) => true,
            (ActivityEventType.LicenseReplaced, LicenseReplaced) => true,
            (ActivityEventType.LicenseRemoved, LicenseRemoved) => true,
            (ActivityEventType.LicenseEnteredGracePeriod, LicenseEnteredGracePeriod) => true,
            (ActivityEventType.LicenseExpired, LicenseExpired) => true,
            (ActivityEventType.LicenseValidationFailed, LicenseValidationFailed) => true,

            (ActivityEventType.VolumeContentDownloaded, VolumeContentDownloaded) => true,

            (ActivityEventType.BuildCreated, BuildCreated) => true,
            (ActivityEventType.BuildUpdated, BuildUpdated) => true,
            (ActivityEventType.BuildRenamed, BuildRenamed) => true,
            (ActivityEventType.BuildDeleted, BuildDeleted) => true,
            (ActivityEventType.BuildRunQueued, BuildRunQueued) => true,
            (ActivityEventType.BuildRunStarted, BuildRunStarted) => true,
            (ActivityEventType.BuildRunSucceeded, BuildRunSucceeded) => true,
            (ActivityEventType.BuildRunFailed, BuildRunFailed) => true,
            (ActivityEventType.BuildRunTimedOut, BuildRunTimedOut) => true,
            (ActivityEventType.BuildRunCancelled, BuildRunCancelled) => true,
            (ActivityEventType.BuildWebhookReceived, BuildWebhookReceived) => true,

            (ActivityEventType.BuildAgentPoolCreated, BuildAgentPoolCreated) => true,
            (ActivityEventType.BuildAgentPoolUpdated, BuildAgentPoolUpdated) => true,
            (ActivityEventType.BuildAgentPoolRenamed, BuildAgentPoolRenamed) => true,
            (ActivityEventType.BuildAgentPoolDeleted, BuildAgentPoolDeleted) => true,
            (ActivityEventType.BuildAgentPoolTested, BuildAgentPoolTested) => true,

            (ActivityEventType.BackupPolicyCreated, BackupPolicyCreated) => true,
            (ActivityEventType.BackupPolicyUpdated, BackupPolicyUpdated) => true,
            (ActivityEventType.BackupPolicyRenamed, BackupPolicyRenamed) => true,
            (ActivityEventType.BackupPolicyArchived, BackupPolicyArchived) => true,

            (ActivityEventType.SwarmServiceCreated, SwarmServiceCreated) => true,
            (ActivityEventType.SwarmServiceUpdated, SwarmServiceUpdated) => true,
            (ActivityEventType.SwarmServiceRenamed, SwarmServiceRenamed) => true,
            (ActivityEventType.SwarmServiceDeleted, SwarmServiceDeleted) => true,
            (ActivityEventType.SwarmServiceApplied, SwarmServiceApplied) => true,
            (ActivityEventType.SwarmServiceScaled, SwarmServiceScaled) => true,
            (ActivityEventType.SwarmServiceForceUpdated, SwarmServiceForceUpdated) => true,
            (ActivityEventType.SwarmServiceOperationFailed, SwarmServiceOperationFailed) => true,
            (ActivityEventType.SwarmServiceDuplicated, SwarmServiceDuplicated) => true,

            _ => false
        };
    }
}



