using System.Collections.Concurrent;
using Application.Services.Abstractions;
using Application.Services.SignalR.Context;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;

namespace Application.Services.SignalR;

internal interface IPlatformsStreamManager : IStreamGroupManager
{
    Task PushPlatformUpdate(Platform platform);
    Task PlatformDeleted(Guid platformId);
    Task PushPlatformsUpdates(IEnumerable<Platform> platforms);
    Task PushPlatformStats(Guid platformId, PlatformStatsResult platform);
}

internal class PlatformsStreamManager(IDockerHubDispatcher dispatcher) : BaseStreamManager<StreamContext>, IPlatformsStreamManager
{
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
        if (streams.IsEmpty)
        {
            return Task.CompletedTask;
        }
        return dispatcher.PushPlatformStats(platformId, platform);
    }
}
