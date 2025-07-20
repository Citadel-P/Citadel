using System.Runtime.CompilerServices;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Hosting.DockerClient.Services;
using Hosting.Extensions;
using Infrastructure.Connectors.Mappers;
using LightResults;

namespace Infrastructure.Connectors.LocalConnectors;

internal class LocalContainerConnector(IContainerService containerService) : IContainerConnector
{
    public Task<Result> DeleteAsync(DeleteContainerCommand deleteContainerCommand, CancellationToken cancellationToken)
        => containerService.DeleteAsync( new Hosting.DockerClient.Models.Containers.DeleteContainersCommand
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
        var command = new Hosting.DockerClient.Models.Containers.ListContainersCommand
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
        var command = new Hosting.DockerClient.Models.Containers.PatchContainersCommand
        (
            ContainerIds: [.. patchContainerCommand.ContainerIds],
            Action: patchContainerCommand.Action.Map()
        );
        return containerService.PatchAsync(command, cancellationToken);
    }

    public async IAsyncEnumerable<DockerContainerStats> StreamContainerStatsAsync(StreamContainerStatsCommand streamStatsCommand, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var result in containerService.StreamContainerStatsAsync(streamStatsCommand.FetchIntervalMs, cancellationToken))
        {
            yield return result.Map();
        }
    }

    public async IAsyncEnumerable<ContainerLogInfo> StreamLogsAsync(StreamContainerLogsCommand streamContainerLogsCommand, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var log in containerService.StreamLogsAsync(streamContainerLogsCommand.ContainerId, cancellationToken))
        {
            yield return new ContainerLogInfo(log);
        }
    }
}
