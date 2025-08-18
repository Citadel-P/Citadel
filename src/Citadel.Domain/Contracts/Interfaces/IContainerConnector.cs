using Domain.Contracts.Resources.Containers;
using Hosting.Common.ObjectPoolManager;
using LightResults;

namespace Domain.Contracts.Interfaces;

/// <summary>
/// Defines methods for managing and monitoring containers in a container runtime environment.
/// </summary>
public interface IContainerConnector
{
    Task<Result<IReadOnlyDictionary<string, DockerContainer>>> ListContainersAsync(ContainerFilterCommand containerFilterCommand, CancellationToken cancellationToken);
    Task<Result<ContainerInspectionInfo>> InspectAsync(InspectContainerCommand inspectContainerCommand, CancellationToken cancellationToken);
    Task<Result<string>> CreateAsync (CreateContainerCommand createContainerCommand, CancellationToken cancellationToken);
    Task<Result> PatchAsync(PatchContainerCommand patchContainerCommand, CancellationToken cancellationToken);
    Task<Result> DeleteAsync(DeleteContainerCommand deleteContainerCommand, CancellationToken cancellationToken);

    IAsyncEnumerable<ReadOnlyMemory<byte>> StreamLogsAsync(StreamContainerLogsCommand streamContainerLogsCommand, CancellationToken cancellationToken);
    IAsyncEnumerable<PooledHandle<DockerContainer>> StreamContainerStatsAsync(StreamContainerStatsCommand streamStatsCommand, CancellationToken cancellationToken);
    IAsyncEnumerable<PooledHandle<Dictionary<string, DockerContainerStat>>> StreamContainersStatsAsync(StreamContainersStatsCommand streamStatsCommand, CancellationToken cancellationToken);

}
