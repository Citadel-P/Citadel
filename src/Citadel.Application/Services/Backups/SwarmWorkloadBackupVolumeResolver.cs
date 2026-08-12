using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Backups;
using Domain.Contracts.Resources.Containers;
using Domain.Entities.Platforms;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services.Backups;

internal interface ISwarmWorkloadBackupVolumeResolver
{
    Task<Result<IReadOnlyList<ResolvedStackBackupVolume>>> ResolveAsync(
        Platform platform,
        IReadOnlyCollection<SwarmServiceProjection> services,
        CancellationToken cancellationToken);
}

internal sealed class SwarmWorkloadBackupVolumeResolver(
    IServiceScopeFactory scopeFactory,
    ISwarmNodeRuntimeConnector nodeRuntimeConnector)
    : ISwarmWorkloadBackupVolumeResolver
{
    private const int MaximumTasksPerService = 5_000;

    public async Task<Result<IReadOnlyList<ResolvedStackBackupVolume>>> ResolveAsync(
        Platform platform,
        IReadOnlyCollection<SwarmServiceProjection> services,
        CancellationToken cancellationToken)
    {
        if (platform.PlatformDescriptor is not DockerSwarmPlatformDescriptor)
            return Result.Failure<IReadOnlyList<ResolvedStackBackupVolume>>(
                new BadRequestError("The selected workload is not on a Docker Swarm Platform."));

        if (platform.Status != PlatformStatus.Online)
            return Result.Failure<IReadOnlyList<ResolvedStackBackupVolume>>(
                new ServiceUnavailableError("The Docker Swarm Platform is offline."));

        if (services.Count == 0)
            return Result.Failure<IReadOnlyList<ResolvedStackBackupVolume>>(
                new BadRequestError("No current Docker Swarm Services were found for this workload."));

        var volumes = new Dictionary<SwarmVolumeKey, ResolvedStackBackupVolume>();
        var mountCounts = new Dictionary<SwarmVolumeKey, int>();

        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var nodes = (await unitOfWork.Swarm.GetNodesAsync(platform.Id, cancellationToken))
            .ToDictionary(static node => node.DockerNodeId, StringComparer.Ordinal);

        foreach (var service in services.OrderBy(static value => value.Name, StringComparer.Ordinal))
        {
            if (service.IsStale)
                return Failure($"Service '{service.Name}' has stale runtime data.");
            if (service.DesiredTaskCount <= 0)
                return Failure($"Service '{service.Name}' has no current Tasks, so local Volume ownership cannot be resolved.");
            if (service.RunningTaskCount < service.DesiredTaskCount)
                return Failure($"Service '{service.Name}' does not have all desired Tasks running.");

            var tasks = await unitOfWork.Swarm.GetTasksAsync(
                platform.Id,
                MaximumTasksPerService,
                cancellationToken,
                service.DockerServiceId);
            var desiredTasks = tasks
                .Where(static task => string.Equals(task.DesiredState, "running", StringComparison.OrdinalIgnoreCase))
                .OrderBy(static task => task.DockerTaskId, StringComparer.Ordinal)
                .ToArray();

            if (desiredTasks.Length != service.DesiredTaskCount)
                return Failure($"Service '{service.Name}' Task placement is not stable.");

            foreach (var task in desiredTasks)
            {
                if (task.IsStale)
                    return Failure($"Task '{task.Name}' has stale runtime data.");
                if (!string.Equals(task.State, "running", StringComparison.OrdinalIgnoreCase))
                    return Failure($"Task '{task.Name}' is not running.");
                if (string.IsNullOrWhiteSpace(task.DockerNodeId)
                    || string.IsNullOrWhiteSpace(task.DockerContainerId))
                {
                    return Failure($"Task '{task.Name}' has no current Node and container placement.");
                }

                if (!nodes.TryGetValue(task.DockerNodeId, out var node)
                    || node.IsStale
                    || !string.Equals(node.Status, "ready", StringComparison.OrdinalIgnoreCase))
                {
                    return Failure($"Task '{task.Name}' is placed on a Node that is unavailable or stale.");
                }

                var inspected = await nodeRuntimeConnector.InspectContainerAsync(
                    platform,
                    task.DockerNodeId,
                    task.DockerContainerId,
                    cancellationToken);
                if (!inspected.IsSuccess(out var container, out var inspectError))
                {
                    return Result.Failure<IReadOnlyList<ResolvedStackBackupVolume>>(
                        new ServiceUnavailableError(
                            $"Could not inspect Task '{task.Name}' on Node '{task.NodeHostname}': {inspectError.Message}"));
                }

                foreach (var mount in container.Mounts)
                {
                    if (!string.Equals(mount.Type, "volume", StringComparison.OrdinalIgnoreCase)
                        || string.IsNullOrWhiteSpace(mount.Name))
                    {
                        continue;
                    }

                    var key = new SwarmVolumeKey(platform.Id, task.DockerNodeId, mount.Name);
                    mountCounts[key] = mountCounts.TryGetValue(key, out var count) ? count + 1 : 1;
                    if (volumes.ContainsKey(key))
                        continue;

                    var volumeResult = await nodeRuntimeConnector.InspectVolumeAsync(
                        platform,
                        task.DockerNodeId,
                        mount.Name,
                        cancellationToken);
                    if (!volumeResult.IsSuccess(out var volume, out var volumeError))
                    {
                        return Result.Failure<IReadOnlyList<ResolvedStackBackupVolume>>(
                            new ServiceUnavailableError(
                                $"Could not inspect Volume '{mount.Name}' on Node '{task.NodeHostname}': {volumeError.Message}"));
                    }

                    if (!IsSupported(volume))
                    {
                        return Failure(
                            $"Volume '{mount.Name}' on Node '{task.NodeHostname}' is not a Docker-managed local Volume without driver options.");
                    }

                    volumes[key] = new ResolvedStackBackupVolume(
                        platform.Id,
                        mount.Name,
                        StackVolumeKind.DeclaredNamed,
                        IsExternal: false,
                        IsShared: false,
                        task.DockerNodeId,
                        task.NodeHostname);
                }
            }
        }

        return Result.Success<IReadOnlyList<ResolvedStackBackupVolume>>(
            [.. volumes
                .Select(pair => pair.Value with { IsShared = mountCounts[pair.Key] > 1 })
                .OrderBy(static value => value.NodeHostname, StringComparer.Ordinal)
                .ThenBy(static value => value.VolumeName, StringComparer.Ordinal)]);
    }

    private static bool IsSupported(Domain.Contracts.Resources.Volumes.DockerVolumeResult volume)
        => string.Equals(volume.Driver, "local", StringComparison.OrdinalIgnoreCase)
           && volume.ClusterVolume is null
           && volume.Options.Count == 0;

    private static Result<IReadOnlyList<ResolvedStackBackupVolume>> Failure(string message)
        => Result.Failure<IReadOnlyList<ResolvedStackBackupVolume>>(new BadRequestError(message));

    private readonly record struct SwarmVolumeKey(Guid PlatformId, string DockerNodeId, string VolumeName);
}
