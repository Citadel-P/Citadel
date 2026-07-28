using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;
using Hosting.Common;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services.SignalR;

internal interface IPlatformStreamManager : IStreamGroupManager
{
    bool HasStatsSubscribers { get; }
    Task PushPlatformUpdate(Platform platform);
    Task RefreshPlatform(Guid platformId);
    Task PlatformDeleted(Guid platformId);
    Task PushPlatformsUpdates(IEnumerable<Platform> platforms);
    Task PushPlatformStats(Guid platformId, PlatformStatsResult platform);
}

internal class PlatformStreamManager(
    IApplicationHubDispatcher dispatcher,
    IServiceScopeFactory scopeFactory) : BaseStreamManager<StreamContext>, IPlatformStreamManager
{
    public bool HasStatsSubscribers => streams.ContainsKey(Constants.WellKnownSignalRGroups.PlatformsGroup);

    public Task PushPlatformUpdate(Platform platform) => RefreshPlatform(platform.Id);

    public async Task RefreshPlatform(Guid platformId)
    {
        if (!HasStatsSubscribers)
        {
            return;
        }

        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var platform = await unitOfWork.Platforms.GetPlatformWithLatestStatAsync(platformId, CancellationToken.None);
        if (platform is not null)
        {
            await dispatcher.PushPlatformUpdate(platform);
        }
    }

    public Task PlatformDeleted(Guid platformId)
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }
        return dispatcher.PlatformDeleted(platformId);
    }

    public Task PushPlatformsUpdates(IEnumerable<Platform> platforms)
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }
        return dispatcher.PushPlatformsUpdates(platforms);
    }

    public Task PushPlatformStats(Guid platformId, PlatformStatsResult platform)
    {
        if (!HasStatsSubscribers)
        {
            return Task.CompletedTask;
        }
        return dispatcher.PushPlatformStats(platformId, platform);
    }
}
