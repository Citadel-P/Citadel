using Domain.Contracts.Resources.Platforms;
using LightResults;

namespace Domain.Contracts.Interfaces;

/// <summary>
/// Defines methods for managing a platform.
/// </summary>
public interface IPlatformConnector
{
    Task<PlatformHealthResult> CheckHealthAsync(string platformAddress, CancellationToken cancellationToken);
    Task<Result<PlatformResult>> GetPlatformAsync(GetPlatformCommand command, CancellationToken cancellationToken);

    IAsyncEnumerable<PlatformStatsResult> StreamStatsAsync(StreamPlatformStatsCommand command, CancellationToken cancellationToken);
}
