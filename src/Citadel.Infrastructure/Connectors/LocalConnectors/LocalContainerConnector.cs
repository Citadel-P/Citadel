using System.Runtime.CompilerServices;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Hosting.DockerClient.Models.Containers;
using Hosting.DockerClient.Services;
using Hosting.Extensions;
using Infrastructure.Connectors.Mappers;
using LightResults;

namespace Infrastructure.Connectors.LocalConnectors;

internal class LocalContainerConnector(IContainerService containerService) : IContainerConnector
{
    public Task<Result<string>> CreateAsync(CreateContainerCommand createContainerCommand, CancellationToken cancellationToken)
    {
        Dictionary<string, Hosting.DockerClient.EndpointSettings>? networks = [];
        foreach (var (k, v) in createContainerCommand.Networks ?? [])
        {
            networks[k] = v.Map();
        }
        var command = new CreateContainersCommand
        (
            ImageId: createContainerCommand.ImageId,
            Name: createContainerCommand.Name,
            WorkingDir: createContainerCommand.WorkingDir,
            User: createContainerCommand.User,
            MemoryLimit: createContainerCommand.MemoryLimit,
            CpuQuota: createContainerCommand.CpuQuota,
            MemoryReservation: createContainerCommand.MemoryReservation,
            MemorySwap: createContainerCommand.MemorySwap,
            PidsLimit: createContainerCommand.PidsLimit,
            AutoRemove: createContainerCommand.AutoRemove,
            Privileged: createContainerCommand.Privileged,
            ReadonlyRootfs: createContainerCommand.ReadonlyRootfs,
            RestartPolicy: createContainerCommand.RestartPolicy?.Map(),
            Labels: createContainerCommand.Labels,
            Networks: networks,
            EntryPoint: createContainerCommand.EntryPoint,
            Command: createContainerCommand.Command,
            EnvVars: createContainerCommand.EnvVars,
            Ports: createContainerCommand.Ports,
            Volumes: createContainerCommand.Volumes,
            Mounts: createContainerCommand.Mounts?.Select(MapMount).ToList(),
            CapAdd: createContainerCommand.CapAdd,
            CapDrop: createContainerCommand.CapDrop,
            SecurityOpt: createContainerCommand.SecurityOpt,
            NetworkMode: createContainerCommand.NetworkMode
        );
        return containerService.CreateAsync(command, cancellationToken);
    }

    public Task<Result> DeleteAsync(DeleteContainerCommand deleteContainerCommand, CancellationToken cancellationToken)
        => containerService.DeleteAsync( new DeleteContainersCommand
            (
                [.. deleteContainerCommand.ContainerIds],
                deleteContainerCommand.Volume,
                deleteContainerCommand.Force,
                deleteContainerCommand.Link
            ),
            cancellationToken);

    public async Task<Domain.Contracts.Interfaces.IExecSession> ExecAsync(string platformAddress, string containerId, string cmd, CancellationToken cancellationToken)
    {
        var result = await containerService.ExecAsync(containerId, [cmd], cancellationToken);
        if (result.IsFailure(out var error, out var resp))
        {
            throw new InvalidOperationException($"Failed to create exec session: {error.Message}");
        }

        return new LocalExecSessionAdapter(resp);
    }

    public async Task<Result<ContainerBinaryExecResult>> ExecBinaryAsync(string platformAddress, ContainerBinaryExecRequest request, CancellationToken cancellationToken)
    {
        var result = await containerService.ExecBinaryAsync(
            new BinaryExecCommand(
                request.ContainerId,
                request.Command,
                request.Environment,
                request.AttachStdout,
                request.AttachStderr,
                request.Tty),
            cancellationToken);

        return ServiceResultHandlers.HandleResult(result, MapBinaryExecResult);
    }

    public async Task<Result<ContainerInspectionInfo>> InspectAsync(InspectContainerCommand inspectContainerCommand, CancellationToken cancellationToken)
    {
        var result = await containerService.InspectAsync(inspectContainerCommand.ContainerId,cancellationToken);
        return ServiceResultHandlers.HandleResult(result, ContainerMappers.Map);  
    }

    public async Task<Result<IReadOnlyDictionary<string, DockerContainer>>> ListContainersAsync(ContainerFilterCommand containerFilterCommand, CancellationToken cancellationToken)
    {
        var command = new ListContainersCommand
        (
            containerFilterCommand.All,
            containerFilterCommand.Limit,
            containerFilterCommand.Size,
            containerFilterCommand.Filters
        );
        var result = await containerService.ListContainersAsync(command, cancellationToken);
        return ServiceResultHandlers.HandleResult(result, ContainerMappers.Map);
    }

    public Task<Result> PatchAsync(PatchContainerCommand patchContainerCommand, CancellationToken cancellationToken)
    {
        var command = new PatchContainersCommand
        (
            ContainerIds: [.. patchContainerCommand.ContainerIds],
            Action: patchContainerCommand.Action.Map()
        );
        return containerService.PatchAsync(command, cancellationToken);
    }

    public async IAsyncEnumerable<Dictionary<string, DockerContainerStat>> StreamContainersStatsAsync(StreamContainersStatsCommand streamStatsCommand, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var result in containerService.StreamContainersStatsAsync(streamStatsCommand.FetchIntervalMs, cancellationToken))
        {
            yield return result.Map();
        }
    }

    public async IAsyncEnumerable<DockerContainer> StreamContainerStatsAsync(StreamContainerStatsCommand streamStatsCommand, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var result in containerService.StreamContainerStatsAsync(streamStatsCommand.ContainerId, streamStatsCommand.FetchIntervalMs, cancellationToken))
        {
            yield return result.Map();
        }
    }

    public async IAsyncEnumerable<ReadOnlyMemory<byte>> StreamLogsAsync(StreamContainerLogsCommand streamContainerLogsCommand, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var data in containerService.StreamLogsAsync(streamContainerLogsCommand.ContainerId, cancellationToken))
        {
            yield return data;
        }
    }

    private static Hosting.DockerClient.Models.Containers.ContainerMount MapMount(HostMount mount)
        => new(
            Type: mount.Type?.Equals("bind", StringComparison.OrdinalIgnoreCase) == true
                ? Hosting.DockerClient.Models.Containers.ContainerMountType.Bind
                : mount.Type?.Equals("tmpfs", StringComparison.OrdinalIgnoreCase) == true
                    ? Hosting.DockerClient.Models.Containers.ContainerMountType.Tmpfs
                    : Hosting.DockerClient.Models.Containers.ContainerMountType.Volume,
            Source: mount.Source,
            Target: mount.Target ?? "/data",
            ReadOnly: mount.ReadOnly ?? false);

    private static ContainerBinaryExecResult MapBinaryExecResult(BinaryExecSession session)
        => new()
        {
            Output = MapOutput(session.Output),
            GetExitCodeAsync = session.GetExitCodeAsync,
            CleanupAsync = session.DisposeAsync
        };

    private static async IAsyncEnumerable<ContainerBinaryExecChunk> MapOutput(
        IAsyncEnumerable<BinaryExecChunk> output,
        [EnumeratorCancellation] CancellationToken cancellationToken = default)
    {
        await foreach (var chunk in output.WithCancellation(cancellationToken))
        {
            yield return new ContainerBinaryExecChunk(
                chunk.Stream == BinaryExecStream.Stderr
                    ? ContainerExecStream.Stderr
                    : ContainerExecStream.Stdout,
                chunk.Data);
        }
    }
}
