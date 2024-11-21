using Infrastructure.Entities;

namespace Application.Services.Abstractions;

public interface IPlatformHubDispatcher
{
    Task SendPlatformUpdated(Platform platform);
}
