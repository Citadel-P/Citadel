using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using LightResults;

namespace Domain.Contracts.Interfaces;

public interface IPlatformConnector
{
    Task<PlatformHealth> CheckHealthAsync(string platformAddress, CancellationToken cancellationToken);
    Task<Result<Platform>> GetPlatformAsync(GetPlatformCommand command, CancellationToken cancellationToken);

    IAsyncEnumerable<PlatformStatsBatch> StreamStatsAsync(StreamPlatformStatsCommand command, CancellationToken cancellationToken);
}
