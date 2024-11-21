using Contracts.Broker.Models;

namespace Infrastructure.Services.Abstractions;

public interface IPlatformService
{
    /// <summary>
    /// Handles the reception of <see cref="SystemInfoMessage"/> from the broker
    /// </summary>
    Task OnSystemInfoMessage(SystemInfoMessage message, CancellationToken cancellationToken = default);
}