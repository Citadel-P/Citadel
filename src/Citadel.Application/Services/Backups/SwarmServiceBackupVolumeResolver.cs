using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Backups;
using Domain.Entities.Platforms;
using Hosting.Common.ErrorTypes;
using LightResults;
using Microsoft.Extensions.DependencyInjection;

namespace Application.Services.Backups;

internal sealed class SwarmServiceBackupVolumeResolver(
    IServiceScopeFactory scopeFactory,
    ISwarmWorkloadBackupVolumeResolver workloadResolver)
    : ISwarmServiceBackupVolumeResolver
{
    public async Task<Result<SwarmServiceBackupVolumeResolution>> ResolveAsync(
        Guid swarmServiceId,
        CancellationToken cancellationToken)
    {
        await using var scope = scopeFactory.CreateAsyncScope();
        var unitOfWork = scope.ServiceProvider.GetRequiredService<IUnitOfWork>();
        var service = await unitOfWork.SwarmServices.GetAsync(swarmServiceId, cancellationToken);
        if (service is null)
            return Result.Failure<SwarmServiceBackupVolumeResolution>(new NotFoundError("Swarm Service not found."));
        if (string.IsNullOrWhiteSpace(service.DockerServiceId))
        {
            return Result.Failure<SwarmServiceBackupVolumeResolution>(
                new BadRequestError("The Swarm Service has not been applied and has no current Tasks."));
        }

        var platform = await unitOfWork.Platforms.GetByIdAsync(service.PlatformId, cancellationToken);
        if (platform?.PlatformDescriptor is not DockerSwarmPlatformDescriptor)
        {
            return Result.Failure<SwarmServiceBackupVolumeResolution>(
                new BadRequestError("The selected Service is not on a Docker Swarm Platform."));
        }

        var projection = await unitOfWork.Swarm.GetServiceAsync(
            platform.Id,
            service.DockerServiceId,
            cancellationToken);
        if (projection is null)
        {
            return Result.Failure<SwarmServiceBackupVolumeResolution>(
                new BadRequestError("The Swarm Service has no current runtime projection."));
        }

        var resolved = await workloadResolver.ResolveAsync(platform, [projection], cancellationToken);
        if (!resolved.IsSuccess(out var volumes, out var error))
            return Result.Failure<SwarmServiceBackupVolumeResolution>(error);

        var warnings = volumes
            .Where(static volume => volume.IsShared)
            .Select(static volume =>
                $"Volume {volume.VolumeName} on Node {volume.NodeHostname ?? volume.DockerNodeId} is mounted by more than one current Task.")
            .Distinct(StringComparer.Ordinal)
            .ToArray();

        return Result.Success(new SwarmServiceBackupVolumeResolution(
            service.Id,
            service.Name,
            platform.Id,
            platform.Name,
            platform.Status,
            volumes,
            warnings));
    }
}
