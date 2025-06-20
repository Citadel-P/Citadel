using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using LightResults;

namespace Domain.Contracts.Interfaces;

public interface IPlatformConnector
{
    Task<PlatformHealthResult> CheckHealthAsync(string platformAddress, CancellationToken cancellationToken);
    Task<Result<PlatformResult>> GetPlatformAsync(GetPlatformCommand command, CancellationToken cancellationToken);

    IAsyncEnumerable<PlatformStatsResult> StreamStatsAsync(StreamPlatformStatsCommand command, CancellationToken cancellationToken);
}
