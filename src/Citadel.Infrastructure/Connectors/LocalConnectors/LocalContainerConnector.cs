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
            AutoRemove: createContainerCommand.AutoRemove,
            RestartPolicy: createContainerCommand.RestartPolicy?.Map(),
            Labels: createContainerCommand.Labels,
            Networks: networks,
            EntryPoint: createContainerCommand.EntryPoint,
            Command: createContainerCommand.Command,
            EnvVars: createContainerCommand.EnvVars,
            Ports: createContainerCommand.Ports,
            Volumes: createContainerCommand.Volumes
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
}
