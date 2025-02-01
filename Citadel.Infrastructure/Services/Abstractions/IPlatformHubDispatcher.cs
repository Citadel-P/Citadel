using Infrastructure.Entities;

namespace Infrastructure.Services.Abstractions;

public interface IPlatformHubDispatcher
{
    Task PushPlatformsUpdates(IEnumerable<Platform> platforms);
}
