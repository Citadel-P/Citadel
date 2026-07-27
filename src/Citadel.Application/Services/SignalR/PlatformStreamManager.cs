using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities.Platforms;
using Hosting.Common;

namespace Application.Services.SignalR;

internal interface IPlatformStreamManager : IStreamGroupManager
{
    bool HasStatsSubscribers { get; }
    Task PushPlatformUpdate(Platform platform);
    Task PlatformDeleted(Guid platformId);
    Task PushPlatformsUpdates(IEnumerable<Platform> platforms);
    Task PushPlatformStats(Guid platformId, PlatformStatsResult platform);
}

internal class PlatformStreamManager(IApplicationHubDispatcher dispatcher) : BaseStreamManager<StreamContext>, IPlatformStreamManager
{
    public bool HasStatsSubscribers => streams.ContainsKey(Constants.WellKnownSignalRGroups.PlatformsGroup);

    public Task PushPlatformUpdate(Platform platform)
    {
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }
        return dispatcher.PushPlatformUpdate(platform);
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
