using Domain.Contracts.Resources.Containers;
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
    Task<IExecSession> ExecAsync(string platformAddress, string containerId, string cmd, CancellationToken cancellationToken);
    Task<Result<ContainerBinaryExecResult>> ExecBinaryAsync(string platformAddress, ContainerBinaryExecRequest request, CancellationToken cancellationToken);

    IAsyncEnumerable<ReadOnlyMemory<byte>> StreamLogsAsync(StreamContainerLogsCommand streamContainerLogsCommand, CancellationToken cancellationToken);
    IAsyncEnumerable<DockerContainer> StreamContainerStatsAsync(StreamContainerStatsCommand streamStatsCommand, CancellationToken cancellationToken);
    IAsyncEnumerable<Dictionary<string, DockerContainerStat>> StreamContainersStatsAsync(StreamContainersStatsCommand streamStatsCommand, CancellationToken cancellationToken);

}

public interface IExecSession : IAsyncDisposable
{
    IAsyncEnumerable<ReadOnlyMemory<byte>> Output { get; }
    Task SendAsync(ReadOnlyMemory<byte> input, CancellationToken ct);
    Task ResizeAsync(int cols, int rows, CancellationToken ct);
}
