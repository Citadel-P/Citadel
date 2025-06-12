using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using LightResults;

namespace Domain.Contracts.Interfaces;

public interface IContainerService
{
    Task<Result<IReadOnlyDictionary<string, Container>>> ListContainersAsync(ContainerFilterCommand containerFilterCommand, CancellationToken cancellationToken);
    Task<Result<ContainerInspectionInfo>> InspectAsync(InspectContainerCommand inspectContainerCommand, CancellationToken cancellationToken);
    Task<Result> PatchAsync (PatchContainerCommand patchContainerCommand, CancellationToken cancellationToken);
    Task<Result> DeleteAsync(DeleteContainerCommand deleteContainerCommand, CancellationToken cancellationToken);

    IAsyncEnumerable<ContainerLogInfo> StreamLogsAsync(StreamContainerLogsCommand streamContainerLogsCommand, CancellationToken cancellationToken);
    IAsyncEnumerable<ContainerStats> StreamContainerStatsAsync(StreamContainerStatsCommand streamStatsCommand, CancellationToken cancellationToken);
    IAsyncEnumerable<DaemonEventInfo> StreamDaemonEventAsync(StreamDaemonEventCommand streamContainerLogsCommand, CancellationToken cancellationToken);
}
