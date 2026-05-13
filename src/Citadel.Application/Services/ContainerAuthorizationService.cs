using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Extensions;
using Hosting.Common.Pipelines.Interfaces;
using Microsoft.AspNetCore.Http;

namespace Application.Services;

internal interface IContainerAuthorizationService
{
    Task<bool> HasAccessAsync(IEnumerable<string> containerIds, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken);
    Task<bool> HasTerminalAccessAsync(ResourceType resourceType, string groupId, CancellationToken cancellationToken);
}

internal sealed class ContainerAuthorizationService(
    IUnitOfWork unitOfWork,
    IPermissionService permissionService,
    IHttpContextAccessor httpContextAccessor,
    IPlatformContainerCache platformContainerCache) : IContainerAuthorizationService
{
    public async Task<bool> HasAccessAsync(IEnumerable<string> containerIds, ResourceType resourceType, PermissionLevel permissionLevel, SpecificPermission specificPermission, CancellationToken cancellationToken)
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
                resourceType,
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

    public Task<bool> HasTerminalAccessAsync(ResourceType resourceType, string groupId, CancellationToken cancellationToken)
    {
        var containerId = TryGetContainerId(groupId);
        return containerId is null
            ? Task.FromResult(false)
            : HasAccessAsync([containerId], resourceType, PermissionLevel.Read, SpecificPermission.Terminal, cancellationToken);
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
        const string prefix = "container-exec:";

        if (!groupId.StartsWith(prefix, StringComparison.Ordinal))
            return null;

        var start = prefix.Length;
        var end = groupId.IndexOf(':', start);

        return end < 0
            ? null
            : groupId[start..end];
    }
}