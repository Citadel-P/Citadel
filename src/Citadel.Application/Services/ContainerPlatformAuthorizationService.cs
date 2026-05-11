using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Extensions;
using Hosting.Common.Pipelines.Interfaces;
using Microsoft.AspNetCore.Http;

namespace Application.Services;

public interface IContainerPlatformAuthorizationService
{
    Task<bool> HasAccessAsync(IEnumerable<string> containerIds, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken);
    Task<bool> HasTerminalAccessAsync(string groupId, CancellationToken cancellationToken);
}

internal sealed class ContainerPlatformAuthorizationService(
    IHttpContextAccessor httpContextAccessor,
    IPlatformContainerCache platformContainerCache,
    IUnitOfWork unitOfWork,
    IPermissionService permissionService) : IContainerPlatformAuthorizationService
{
    public async Task<bool> HasAccessAsync(IEnumerable<string> containerIds, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken)
    {
        var ids = containerIds as string[] ?? [.. containerIds];
        if (ids.Length == 0)
        {
            return false;
        }

        var user = httpContextAccessor.HttpContext?.User;
        if (user is null)
        {
            return false;
        }

        var platformIds = await ResolvePlatformIdsAsync(ids, cancellationToken);
        if (platformIds is null)
        {
            return false;
        }

        if (user.IsAdmin())
        {
            return true;
        }

        var userId = user.GetUserId();
        if (userId == Guid.Empty)
        {
            return false;
        }

        foreach (var platformId in platformIds)
        {
            var hasPermission = await permissionService.HasPermissionAsync(
                userId,
                ResourceType.Platform,
                permissionLevel,
                specificPermission,
                platformId,
                cancellationToken);

            if (!hasPermission)
            {
                return false;
            }
        }

        return true;
    }

    public Task<bool> HasTerminalAccessAsync(string groupId, CancellationToken cancellationToken)
    {
        var containerId = TryGetContainerId(groupId);
        return containerId is null
            ? Task.FromResult(false)
            : HasAccessAsync([containerId], PermissionLevel.Execute, SpecificPermission.Terminal, cancellationToken);
    }

    private async Task<Guid?> ResolvePlatformIdAsync(string containerId, CancellationToken cancellationToken)
    {
        if (platformContainerCache.TryGetPlatformWithContainer(containerId, out var platform))
        {
            return platform.Id;
        }

        var persistedPlatform = await unitOfWork.Platforms.GetPlatformByContainerIdAsync(containerId, cancellationToken);
        return persistedPlatform?.Id;
    }

    private async Task<HashSet<Guid>?> ResolvePlatformIdsAsync(string[] containerIds, CancellationToken cancellationToken)
    {
        if (TryResolvePlatformIdsFromCache(containerIds, out var cachedPlatformIds))
        {
            return cachedPlatformIds;
        }

        var platformIds = new HashSet<Guid>();
        foreach (var containerId in containerIds)
        {
            var platformId = await ResolvePlatformIdAsync(containerId, cancellationToken);
            if (platformId is null)
            {
                return null;
            }

            platformIds.Add(platformId.Value);
        }

        return platformIds;
    }

    private bool TryResolvePlatformIdsFromCache(string[] containerIds, out HashSet<Guid> platformIds)
    {
        platformIds = [];

        if (!platformContainerCache.TryGetPlatformsWithContainers(containerIds, out var platforms))
        {
            return false;
        }

        var resolvedContainerCount = 0;
        foreach (var platform in platforms)
        {
            platformIds.Add(platform.Id);
            resolvedContainerCount += platform.Containers.Count;
        }

        return resolvedContainerCount == containerIds.Length;
    }

    private static string? TryGetContainerId(string groupId)
    {
        var parts = groupId.Split(':', StringSplitOptions.RemoveEmptyEntries);
        return parts.Length >= 3 && parts[0] == "container-exec"
            ? parts[1]
            : null;
    }
}