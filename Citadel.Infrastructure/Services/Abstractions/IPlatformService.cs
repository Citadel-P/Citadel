using Gplatform;

namespace Infrastructure.Services.Abstractions;

public interface IPlatformService
{
    Task OnSystemInfoMessage(SystemInfoMessage message, CancellationToken cancellationToken);
}
