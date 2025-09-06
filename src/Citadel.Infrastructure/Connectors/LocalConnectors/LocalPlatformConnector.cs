using System.Runtime.CompilerServices;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Contracts.Resources.Platforms;
using Hosting.DockerClient.Services;
using Hosting.Extensions;
using Infrastructure.Connectors.Mappers;
using LightResults;

namespace Infrastructure.Connectors.LocalConnectors;

internal class LocalPlatformConnector(IPlatformService platformService, IMonitorEventsService monitorEventsService) : IPlatformConnector
{
    public async Task<PlatformHealthResult> CheckHealthAsync(string platformAddress, CancellationToken cancellationToken)
    {
        var result = await platformService.HealthCheck(cancellationToken);
        return result.IsSuccess(out var res)
            ? new PlatformHealthResult(Healthy: res.Healthy)
            : new PlatformHealthResult(Healthy: false);
    }

    public async Task<Result<PlatformResult>> GetPlatformAsync(GetPlatformCommand command, CancellationToken cancellationToken)
    {
        var result = await platformService.GetPlatformInfo(cancellationToken);
        return ServiceResultHandlers.HandleResult(result, p => PlatformMappers.Map(p, command.PlatformName, command.PlatformAddress));
    }

    public async IAsyncEnumerable<DaemonEventInfo> StreamDaemonEventAsync(StreamDaemonEventCommand streamContainerLogsCommand, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var eventInfo in monitorEventsService.StreamEvents(cancellationToken))
        {
            yield return eventInfo.Map();
        }
    }

    public async IAsyncEnumerable<PlatformStatsResult> StreamStatsAsync(StreamPlatformStatsCommand command, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var result in platformService.StreamPlatformStatsAsync(command.FetchIntervalMs, cancellationToken))
        {
            yield return result.Map();
        }
    }
}
