using Domain;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Extensions;
using Hosting.Common.Pipelines.Interfaces;
using System.Security.Claims;

namespace Application.Services.SignalR;

public interface ISignalRGroupAuthorizationService
{
    Task<bool> CanJoinAsync(
        ClaimsPrincipal user,
        string groupId,
        CancellationToken cancellationToken);
}

internal sealed class SignalRGroupAuthorizationService(
    IUnitOfWork unitOfWork,
    IPermissionService permissionService) : ISignalRGroupAuthorizationService
{
    public async Task<bool> CanJoinAsync(
        ClaimsPrincipal user,
        string groupId,
        CancellationToken cancellationToken)
    {
        if (!TryParseTarget(groupId, out var target))
            return false;

        if (user.Identity?.IsAuthenticated != true)
            return false;

        Guid userId;
        try
        {
            userId = user.GetUserId();
        }
        catch (Exception exception) when (exception is ArgumentException or FormatException)
        {
            return false;
        }

        if (userId == Guid.Empty)
            return false;

        // Hub connections can outlive permission and account changes. Resolve the
        // current database state for every join instead of trusting JWT-era roles.
        var authInfo = await unitOfWork.Users.GetUserAuthInfoByIdAsync(userId, cancellationToken);
        if (authInfo is null)
            return false;

        if (authInfo.Roles.Any(static role =>
                string.Equals(role, "admin", StringComparison.OrdinalIgnoreCase)))
        {
            return true;
        }

        return target.Kind switch
        {
            TargetKind.Permission => await HasPermissionAsync(
                userId,
                target.ResourceType,
                target.ResourceId,
                target.SpecificPermission,
                cancellationToken),
            TargetKind.Container => await HasContainerPermissionAsync(
                userId,
                target.ContainerId!,
                target.SpecificPermission,
                cancellationToken),
            TargetKind.BackupRun => await HasBackupRunPermissionAsync(
                userId,
                target.ResourceId!.Value,
                target.SpecificPermission,
                cancellationToken),
            TargetKind.BackupRestoreRun => await HasBackupRestoreRunPermissionAsync(
                userId,
                target.ResourceId!.Value,
                cancellationToken),
            TargetKind.BuildRun => await HasBuildRunPermissionAsync(
                userId,
                target.ResourceId!.Value,
                cancellationToken),
            TargetKind.SwarmService => await HasSwarmServicePermissionAsync(
                userId,
                target.ResourceId!.Value,
                cancellationToken),
            TargetKind.SwarmServicesForPlatform => await HasSwarmServicesPlatformPermissionAsync(
                userId,
                target.ResourceId!.Value,
                cancellationToken),
            // Alert dispatch already selects database-authorized user IDs and sends
            // only to user-specific groups.
            TargetKind.AlertEvents => true,
            TargetKind.AdminOnly => false,
            _ => false
        };
    }

    private async Task<bool> HasContainerPermissionAsync(
        Guid userId,
        string containerId,
        SpecificPermission specificPermission,
        CancellationToken cancellationToken)
    {
        var container = Guid.TryParse(containerId, out var resourceId)
            ? await unitOfWork.Containers.GetByIdAsync(resourceId, cancellationToken)
            : await unitOfWork.Containers.GetByIdAsync(containerId, cancellationToken);
        if (container is null)
            return false;

        if (container.DeploymentId.HasValue &&
            await HasPermissionAsync(
                userId,
                ResourceType.Deployment,
                container.DeploymentId,
                specificPermission,
                cancellationToken))
        {
            return true;
        }

        if (container.StackId.HasValue &&
            await HasPermissionAsync(
                userId,
                ResourceType.Stack,
                container.StackId,
                specificPermission,
                cancellationToken))
        {
            return true;
        }

        return await HasPermissionAsync(
            userId,
            ResourceType.Platform,
            container.PlatformId,
            specificPermission,
            cancellationToken);
    }

    private async Task<bool> HasSwarmServicePermissionAsync(
        Guid userId,
        Guid serviceId,
        CancellationToken cancellationToken)
    {
        var service = await unitOfWork.SwarmServices.GetAsync(serviceId, cancellationToken);
        return service is not null
            && await HasPermissionAsync(
                userId, ResourceType.SwarmService, serviceId, SpecificPermission.None, cancellationToken)
            && await HasPermissionAsync(
                userId, ResourceType.Platform, service.PlatformId, SpecificPermission.None, cancellationToken);
    }

    private async Task<bool> HasSwarmServicesPlatformPermissionAsync(
        Guid userId,
        Guid platformId,
        CancellationToken cancellationToken) =>
        await HasPermissionAsync(
            userId, ResourceType.SwarmService, null, SpecificPermission.None, cancellationToken)
        && await HasPermissionAsync(
            userId, ResourceType.Platform, platformId, SpecificPermission.None, cancellationToken);

    private async Task<bool> HasBackupRunPermissionAsync(
        Guid userId,
        Guid runId,
        SpecificPermission specificPermission,
        CancellationToken cancellationToken)
    {
        var run = await unitOfWork.BackupRuns.GetAsync(runId, cancellationToken);
        return run is not null &&
               await HasPermissionAsync(
                   userId,
                   ResourceType.BackupPolicy,
                   run.BackupPolicyId,
                   specificPermission,
                   cancellationToken);
    }

    private async Task<bool> HasBackupRestoreRunPermissionAsync(
        Guid userId,
        Guid restoreRunId,
        CancellationToken cancellationToken)
    {
        var restoreRun = await unitOfWork.BackupRestoreRuns.GetAsync(restoreRunId, cancellationToken);
        if (restoreRun is null)
            return false;

        return await HasBackupRunPermissionAsync(
            userId,
            restoreRun.BackupRunId,
            SpecificPermission.Restore,
            cancellationToken);
    }

    private async Task<bool> HasBuildRunPermissionAsync(
        Guid userId,
        Guid runId,
        CancellationToken cancellationToken)
    {
        var run = await unitOfWork.BuildRuns.GetAsync(runId, cancellationToken);
        return run is not null &&
               await HasPermissionAsync(
                   userId,
                   ResourceType.Build,
                   run.BuildProjectId,
                   SpecificPermission.None,
                   cancellationToken);
    }

    private async Task<bool> HasPermissionAsync(
        Guid userId,
        ResourceType resourceType,
        Guid? resourceId,
        SpecificPermission specificPermission,
        CancellationToken cancellationToken)
    {
        var permissions = await permissionService.ResolvePermissionsAsync(
            userId,
            resourceType,
            resourceId,
            cancellationToken);

        return permissions.Has(PermissionLevel.Read, specificPermission);
    }

    private static bool TryParseTarget(string groupId, out AuthorizationTarget target)
    {
        target = default;
        if (string.IsNullOrWhiteSpace(groupId) ||
            groupId.Length > 256 ||
            !string.Equals(groupId, groupId.Trim(), StringComparison.Ordinal) ||
            groupId.Any(char.IsControl))
        {
            return false;
        }

        target = groupId switch
        {
            "stacks" => Permission(ResourceType.Stack),
            "platforms" => Permission(ResourceType.Platform),
            "deployments" => Permission(ResourceType.Deployment),
            "git-repositories" => Permission(ResourceType.GitRepository),
            "backup-repositories" => Permission(ResourceType.BackupRepository),
            "backup-policies" => Permission(ResourceType.BackupPolicy),
            "build-projects" => Permission(ResourceType.Build),
            "build-agent-pools" => Permission(ResourceType.BuildAgentPool),
            "automation-actions" => Permission(ResourceType.AutomationAction),
            "alert-events" => new AuthorizationTarget(TargetKind.AlertEvents),
            _ => default
        };

        if (target.Kind != TargetKind.Unknown)
            return true;

        var parts = groupId.Split(':');
        if (parts.Length == 2 && Guid.TryParse(parts[1], out var resourceId))
        {
            target = parts[0] switch
            {
                "stack" or "stack-info" => Permission(ResourceType.Stack, resourceId),
                "stack-log" => Permission(ResourceType.Stack, resourceId, SpecificPermission.Logs),
                "deployment" => Permission(ResourceType.Deployment, resourceId),
                "swarm-service" => new AuthorizationTarget(TargetKind.SwarmService, ResourceId: resourceId),
                "swarm-services" => new AuthorizationTarget(TargetKind.SwarmServicesForPlatform, ResourceId: resourceId),
                "git-repo" => Permission(ResourceType.GitRepository, resourceId),
                "backup-repository" => Permission(ResourceType.BackupRepository, resourceId),
                "backup-policy" or "backup-runs" => Permission(ResourceType.BackupPolicy, resourceId),
                "backup-restore-runs" => Permission(
                    ResourceType.BackupPolicy,
                    resourceId,
                    SpecificPermission.Restore),
                "backup-run" => new AuthorizationTarget(
                    TargetKind.BackupRun,
                    ResourceId: resourceId),
                "backup-restore-run" => new AuthorizationTarget(
                    TargetKind.BackupRestoreRun,
                    ResourceId: resourceId),
                "build-project" or "build-runs" => Permission(ResourceType.Build, resourceId),
                "build-agent-pool" => Permission(ResourceType.BuildAgentPool, resourceId),
                "build-run" => new AuthorizationTarget(
                    TargetKind.BuildRun,
                    ResourceId: resourceId),
                "automation-action" => Permission(ResourceType.AutomationAction, resourceId),
                "docker-daemon" or "containers" or "images" => Permission(
                    ResourceType.Platform,
                    resourceId),
                _ => default
            };

            if (target.Kind != TargetKind.Unknown)
                return true;
        }

        if (parts.Length == 2 && IsValidOpaqueSegment(parts[1]))
        {
            target = parts[0] switch
            {
                "container-info" => Container(parts[1]),
                "container-log" => Container(parts[1], SpecificPermission.Logs),
                _ => default
            };

            if (target.Kind != TargetKind.Unknown)
                return true;
        }

        if (parts.Length == 3 &&
            parts[0] == "container-exec" &&
            IsValidOpaqueSegment(parts[1]) &&
            IsValidOpaqueSegment(parts[2]))
        {
            target = Container(parts[1], SpecificPermission.Terminal);
            return true;
        }

        if (parts.Length == 4
            && parts[0] == "swarm-task-exec"
            && Guid.TryParse(parts[1], out resourceId)
            && IsValidOpaqueSegment(parts[2])
            && IsValidOpaqueSegment(parts[3]))
        {
            target = Permission(ResourceType.Platform, resourceId, SpecificPermission.Terminal);
            return true;
        }

        if (parts.Length == 3 &&
            parts[0] == "activity" &&
            Guid.TryParse(parts[2], out resourceId) &&
            Enum.TryParse<ActivityResourceType>(parts[1], ignoreCase: false, out var activityType))
        {
            target = Activity(activityType, resourceId);
            return target.Kind != TargetKind.Unknown;
        }

        return false;
    }

    private static AuthorizationTarget Activity(ActivityResourceType activityType, Guid resourceId) =>
        activityType switch
        {
            ActivityResourceType.Platform => Permission(ResourceType.Platform, resourceId),
            ActivityResourceType.Registry => Permission(ResourceType.Registry, resourceId),
            ActivityResourceType.Deployment => Permission(ResourceType.Deployment, resourceId),
            ActivityResourceType.Stack => Permission(ResourceType.Stack, resourceId),
            ActivityResourceType.AlertRule => Permission(ResourceType.Alert, resourceId),
            ActivityResourceType.GitRepository => Permission(ResourceType.GitRepository, resourceId),
            ActivityResourceType.OidcProvider => new AuthorizationTarget(TargetKind.AdminOnly),
            ActivityResourceType.AutomationAction => Permission(ResourceType.AutomationAction, resourceId),
            ActivityResourceType.User => new AuthorizationTarget(TargetKind.AdminOnly),
            ActivityResourceType.Team => new AuthorizationTarget(TargetKind.AdminOnly),
            ActivityResourceType.Role => new AuthorizationTarget(TargetKind.AdminOnly),
            ActivityResourceType.License => new AuthorizationTarget(TargetKind.AdminOnly),
            ActivityResourceType.Build => Permission(ResourceType.Build, resourceId),
            ActivityResourceType.BuildAgentPool => Permission(ResourceType.BuildAgentPool, resourceId),
            ActivityResourceType.Volume => Permission(ResourceType.Platform, resourceId),
            ActivityResourceType.BackupPolicy => Permission(ResourceType.BackupPolicy, resourceId),
            ActivityResourceType.SwarmService => Permission(ResourceType.SwarmService, resourceId),
            ActivityResourceType.ServiceAccount => Permission(ResourceType.ServiceAccount, resourceId),
            _ => default
        };

    private static AuthorizationTarget Permission(
        ResourceType resourceType,
        Guid? resourceId = null,
        SpecificPermission specificPermission = SpecificPermission.None) =>
        new(
            TargetKind.Permission,
            resourceType,
            resourceId,
            specificPermission);

    private static AuthorizationTarget Container(
        string containerId,
        SpecificPermission specificPermission = SpecificPermission.None) =>
        new(
            TargetKind.Container,
            SpecificPermission: specificPermission,
            ContainerId: containerId);

    private static bool IsValidOpaqueSegment(string value) =>
        !string.IsNullOrWhiteSpace(value) &&
        value.Length <= 128 &&
        value.All(static character => !char.IsControl(character) && character != ':');

    private enum TargetKind
    {
        Unknown,
        Permission,
        Container,
        BackupRun,
        BackupRestoreRun,
        BuildRun,
        SwarmService,
        SwarmServicesForPlatform,
        AlertEvents,
        AdminOnly
    }

    private readonly record struct AuthorizationTarget(
        TargetKind Kind,
        ResourceType ResourceType = default,
        Guid? ResourceId = null,
        SpecificPermission SpecificPermission = SpecificPermission.None,
        string? ContainerId = null);
}
