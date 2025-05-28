using Infrastructure.Entities;

namespace Infrastructure.Services.Abstractions;

public interface IPlatformHubDispatcher
{
    Task PushPlatformUpdate(Platform platform);
    Task PushPlatformsUpdates(IEnumerable<Platform> platforms);
}
