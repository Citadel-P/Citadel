using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using System.Collections.Concurrent;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Logging;

namespace Application.Services;

internal interface IAgentRuntimeImageResolver
{
    Task<string?> TryResolveAsync(
        IContainerConnector containerConnector,
        string platformAddress,
        Guid platformId,
        PlatformConnectorType connectorType,
        CancellationToken cancellationToken);
}

internal sealed class AgentRuntimeImageResolver(
    IServiceScopeFactory scopeFactory,
    IConnectorFactory<IPlatformConnector> platformConnectorFactory,
    ILogger<AgentRuntimeImageResolver> logger) : IAgentRuntimeImageResolver
{
    private static readonly TimeSpan ResolvedImageCacheDuration = TimeSpan.FromMinutes(10);
    private static readonly TimeSpan MissingImageCacheDuration = TimeSpan.FromSeconds(30);
    private readonly ConcurrentDictionary<RuntimeImageCacheKey, RuntimeImageCacheEntry> cache = new();

    public async Task<string?> TryResolveAsync(
        IContainerConnector containerConnector,
        string platformAddress,
        Guid platformId,
        PlatformConnectorType connectorType,
        CancellationToken cancellationToken)
    {
        if (connectorType is not (PlatformConnectorType.Agent or PlatformConnectorType.EdgeAgent))
            return null;

        var cacheKey = new RuntimeImageCacheKey(platformId, platformAddress, connectorType);
        if (cache.TryGetValue(cacheKey, out var cached) && cached.ExpiresAtUtc > DateTimeOffset.UtcNow)
            return cached.Image;
        if (cached is not null)
            cache.TryRemove(cacheKey, out _);

        var runtimeImage = await TryResolveFromPlatformInfoAsync(
            platformAddress,
            connectorType,
            cancellationToken);

        if (!string.IsNullOrWhiteSpace(runtimeImage))
        {
            Cache(cacheKey, runtimeImage, ResolvedImageCacheDuration);
            return runtimeImage;
        }

        if (connectorType != PlatformConnectorType.EdgeAgent)
        {
            Cache(cacheKey, null, MissingImageCacheDuration);
            return null;
        }

        runtimeImage = await TryResolveEdgeAgentImageFromBindingAsync(
            containerConnector,
            platformAddress,
            platformId,
            cancellationToken);

        Cache(
            cacheKey,
            runtimeImage,
            string.IsNullOrWhiteSpace(runtimeImage) ? MissingImageCacheDuration : ResolvedImageCacheDuration);

        return runtimeImage;
    }

    private async Task<string?> TryResolveFromPlatformInfoAsync(
        string platformAddress,
        PlatformConnectorType connectorType,
        CancellationToken cancellationToken)
    {
        try
        {
            var platformConnector = platformConnectorFactory.GetConnector(connectorType);
            var platformInfo = await platformConnector.GetPlatformAsync(
                new GetPlatformCommand(platformAddress, string.Empty),
                cancellationToken);

            if (!platformInfo.IsSuccess(out var platform))
                return null;

            return string.IsNullOrWhiteSpace(platform.AgentRuntimeImage)
                ? null
                : platform.AgentRuntimeImage.Trim();
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception ex)
        {
            logger.LogDebug(ex, "Could not resolve Agent runtime image from platform info for {PlatformAddress}", platformAddress);
            return null;
        }
    }

    private async Task<string?> TryResolveEdgeAgentImageFromBindingAsync(
        IContainerConnector containerConnector,
        string platformAddress,
        Guid platformId,
        CancellationToken cancellationToken)
    {
        var hostname = await GetLastSeenHostnameAsync(platformId, cancellationToken);
        if (string.IsNullOrWhiteSpace(hostname))
            return null;

        try
        {
            var inspected = await containerConnector.InspectAsync(
                new InspectContainerCommand(platformAddress, hostname),
                cancellationToken);

            if (!inspected.IsSuccess(out var container, out var error))
            {
                logger.LogDebug(
                    "Could not resolve Edge Agent runtime image for platform {PlatformId} from container {ContainerId}: {Error}",
                    platformId,
                    hostname,
                    error?.Message);
                return null;
            }

            var image = string.IsNullOrWhiteSpace(container.Config?.Image)
                ? container.Image
                : container.Config.Image;
            return string.IsNullOrWhiteSpace(image) ? null : image.Trim();
        }
        catch (OperationCanceledException) when (cancellationToken.IsCancellationRequested)
        {
            throw;
        }
        catch (Exception ex)
        {
            logger.LogDebug(ex, "Could not resolve Edge Agent runtime image for platform {PlatformId}", platformId);
            return null;
        }
    }

    private async Task<string?> GetLastSeenHostnameAsync(Guid platformId, CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var binding = await unitOfWork.EdgeAgents.GetBindingByPlatformIdAsync(platformId, cancellationToken);
        return binding?.LastSeenHostname;
    }

    private void Cache(RuntimeImageCacheKey key, string? image, TimeSpan duration)
        => cache[key] = new RuntimeImageCacheEntry(
            string.IsNullOrWhiteSpace(image) ? null : image.Trim(),
            DateTimeOffset.UtcNow.Add(duration));

    private sealed record RuntimeImageCacheKey(
        Guid PlatformId,
        string PlatformAddress,
        PlatformConnectorType ConnectorType);

    private sealed record RuntimeImageCacheEntry(string? Image, DateTimeOffset ExpiresAtUtc);
}
