using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using Hosting.Common.ObjectPoolManager;
using LightResults;

namespace Domain.Contracts.Interfaces;

/// <summary>
/// Defines methods for managing a platform.
/// </summary>
public interface IPlatformConnector
{
    Task<PlatformHealthResult> CheckHealthAsync(string platformAddress, CancellationToken cancellationToken);
    Task<Result<PlatformResult>> GetPlatformAsync(GetPlatformCommand command, CancellationToken cancellationToken);

    IAsyncEnumerable<PooledHandle<PlatformStatsResult>> StreamStatsAsync(StreamPlatformStatsCommand command, CancellationToken cancellationToken);
    IAsyncEnumerable<DaemonEventInfo> StreamDaemonEventAsync(StreamDaemonEventCommand streamContainerLogsCommand, CancellationToken cancellationToken);
}
