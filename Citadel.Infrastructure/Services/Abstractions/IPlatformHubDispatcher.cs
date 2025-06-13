using Domain.Contracts.Resources.Platforms;
using Domain.Entities;

namespace Infrastructure.Services.Abstractions;

public interface IPlatformHubDispatcher
{
    Task PushPlatformUpdate(Platform platform);
    Task PlatformDeleted(Guid platformId);
    Task PushPlatformsUpdates(IEnumerable<Platform> platforms);
    Task PushPlatformStats(PlatformStatsBatch platform);
}
