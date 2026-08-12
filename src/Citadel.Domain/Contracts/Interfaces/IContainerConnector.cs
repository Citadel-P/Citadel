using Domain.Contracts.Resources.Containers;
using LightResults;

using Domain.Entities.Platforms;
using Domain.Contracts.Resources.Swarm;
using Domain.Contracts.Resources.Images;
using Domain.Contracts.Resources.Networks;
using Domain.Contracts.Resources.Volumes;

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

public interface ISwarmNodeRuntimeConnector
{
    Task<Result<IReadOnlyList<ImageResult>>> ListImagesAsync(
        Platform platform,
        string dockerNodeId,
        CancellationToken cancellationToken);

    Task<Result<IReadOnlyList<DockerVolumeResult>>> ListVolumesAsync(
        Platform platform,
        string dockerNodeId,
        CancellationToken cancellationToken);

    Task<Result<IReadOnlyList<DockerNetworkResult>>> ListNetworksAsync(
        Platform platform,
        string dockerNodeId,
        CancellationToken cancellationToken);

    Task<Result<InspectImageResult>> InspectImageAsync(
        Platform platform,
        string dockerNodeId,
        string dockerImageId,
        CancellationToken cancellationToken);

    Task<Result<DockerVolumeResult>> InspectVolumeAsync(
        Platform platform,
        string dockerNodeId,
        string volumeName,
        CancellationToken cancellationToken);

    Task<Result<DockerVolumeResult>> CreateVolumeAsync(
        Platform platform,
        string dockerNodeId,
        CreateDockerVolumeCommand command,
        CancellationToken cancellationToken);

    Task<Result> DeleteVolumeAsync(
        Platform platform,
        string dockerNodeId,
        DeleteDockerVolumeCommand command,
        CancellationToken cancellationToken);

    Task<Result<DockerNetworkDetails>> InspectNetworkAsync(
        Platform platform,
        string dockerNodeId,
        string dockerNetworkId,
        CancellationToken cancellationToken);

    Task<Result<string>> CreateContainerAsync(
        Platform platform,
        string dockerNodeId,
        CreateContainerCommand command,
        CancellationToken cancellationToken);

    Task<Result<ContainerBinaryExecResult>> ExecBinaryAsync(
        Platform platform,
        string dockerNodeId,
        ContainerBinaryExecRequest request,
        CancellationToken cancellationToken);

    Task<Result<IReadOnlyDictionary<string, DockerContainer>>> ListContainersAsync(
        Platform platform,
        string dockerNodeId,
        CancellationToken cancellationToken);

    Task<Result<ContainerInspectionInfo>> InspectContainerAsync(
        Platform platform,
        string dockerNodeId,
        string dockerContainerId,
        CancellationToken cancellationToken);

    IAsyncEnumerable<ReadOnlyMemory<byte>> StreamContainerLogsAsync(
        Platform platform,
        string dockerNodeId,
        string dockerContainerId,
        CancellationToken cancellationToken);

    Task<Result<SwarmLogsResult>> GetContainerLogsAsync(
        Platform platform,
        string dockerNodeId,
        string dockerContainerId,
        int tail,
        CancellationToken cancellationToken);

    IAsyncEnumerable<DockerContainer> StreamContainerStatsAsync(
        Platform platform,
        string dockerNodeId,
        string dockerContainerId,
        int fetchIntervalMs,
        CancellationToken cancellationToken);

    IAsyncEnumerable<Dictionary<string, DockerContainerStat>> StreamContainersStatsAsync(
        Platform platform,
        string dockerNodeId,
        int fetchIntervalMs,
        CancellationToken cancellationToken);

    IAsyncEnumerable<DaemonEventInfo> StreamDaemonEventsAsync(
        Platform platform,
        string dockerNodeId,
        CancellationToken cancellationToken);

    Task<Result> PatchContainersAsync(
        Platform platform,
        string dockerNodeId,
        ContainerAction action,
        IReadOnlyCollection<string> dockerContainerIds,
        CancellationToken cancellationToken);

    Task<Result> DeleteContainersAsync(
        Platform platform,
        string dockerNodeId,
        IReadOnlyCollection<string> dockerContainerIds,
        bool volumes,
        bool force,
        bool link,
        CancellationToken cancellationToken);

    Task<IExecSession> ExecAsync(
        Platform platform,
        string dockerNodeId,
        string dockerContainerId,
        string command,
        CancellationToken cancellationToken);
}
