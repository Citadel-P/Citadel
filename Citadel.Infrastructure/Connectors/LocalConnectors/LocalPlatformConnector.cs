using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using LightResults;

namespace Infrastructure.Connectors.LocalConnectors;

internal class LocalPlatformConnector : IPlatformConnector
{
    public Task<PlatformHealthResult> CheckHealthAsync(string platformAddress, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
    }

    public Task<Result<PlatformResult>> GetPlatformAsync(GetPlatformCommand command, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
    }

    public IAsyncEnumerable<PlatformStatsResult> StreamStatsAsync(StreamPlatformStatsCommand command, CancellationToken cancellationToken)
    {
        throw new NotImplementedException();
    }
}
