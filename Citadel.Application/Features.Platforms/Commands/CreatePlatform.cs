using Agent.Server.Containers;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Infrastructure.TaskJobs;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;

namespace Application.Features.Platforms.Commands;

public sealed record CreatePlatform(string Name, string Address, PlatformType Type) : ICommand<Result<Platform>>
{
    internal class Validator : AbstractValidator<CreatePlatform>
    {
        public Validator()
        {
            RuleFor(x => x.Name).NotEmpty().ValidNameIdentifier();
            RuleFor(x => x.Address).ValidHostOrIp();
        }
    }
}

internal sealed class CreatePlatformHandler(
    IGrpcClientFactory clientFactory,
    IDaemonEventJob daemonEventJob,
    IPlatformsStatsReaderJob platformInfoJob,
    IContainersStatsReaderJob containersStatsJob,
    ApplicationDbContext dbContext,
    IGrpcHealthMonitorJob grpcHealthMonitorJob,
    ILogger<UpsertPlatformHandler> logger) : ICommandHandler<CreatePlatform, Result<Platform>>
{
    public async ValueTask<Result<Platform>> Handle(CreatePlatform command, CancellationToken cancellationToken)
    {
        if (command.Type == PlatformType.Docker)
        {
            return await HandleDockerPlatform(command, cancellationToken);
        }
        else
        {
            return Result.Failure<Platform>(new BadRequestError("Only Docker platform type is supported at the moment."));
        }
    }

    private async Task<Result<Platform>> HandleDockerPlatform(CreatePlatform command, CancellationToken cancellationToken)
    {
        // Get platform and containers info
        var platformClient = clientFactory.GetPlatformClient(command.Address);
        var containersClient = clientFactory.GetContainerClient(command.Address);

        var platformInfoTsk = platformClient.GetPlatformInfoAsync(new Google.Protobuf.WellKnownTypes.Empty(), cancellationToken: cancellationToken);
        var containersTsk = containersClient.ListContainersAsync(new ContainersListMessage { All = true }, cancellationToken: cancellationToken);
        var platformInfo = await platformInfoTsk;
        var containers = await containersTsk;

        // Check if the platform already exists
        if (await dbContext.Platforms.AsNoTracking()
                                    .FirstOrDefaultAsync(s => s.Address == command.Address || s.Name == command.Name, cancellationToken: cancellationToken) != null)
        {
            return Result.Failure<Platform>(new ConflictError("A platform with the same [Name] or [Address] already exists!"));
        }

        DockerPlatformDescriptor descriptor = new (
            daemonId: platformInfo.Id,
            containerCount: platformInfo.ContainerCount,
            containersRunning: platformInfo.ContainersRunning,
            containersPaused: platformInfo.ContainersPaused,
            containersStopped: platformInfo.ContainersStopped,
            driver: platformInfo.Driver,
            operatingSystem: platformInfo.OperatingSystem,
            osVersion: platformInfo.OsVersion,
            osType: platformInfo.OsType,
            architecture: platformInfo.Architecture);

        var platform = Platform.Create(
            name: command.Name,
            address: command.Address,
            networkCount: platformInfo.NetworkCount,
            volumeCount: platformInfo.VolumeCount,
            imageCount: platformInfo.ImageCount,
            cpuCount: platformInfo.CpuCount,
            memTotal: platformInfo.MemTotal,
            serverVersion: platformInfo.ServerVersion,
            agentVersion: platformInfo.AgentVersion,
            descriptor: descriptor,
            stats: [platformInfo.MapStat()]
            );
        dbContext.Platforms.Add(platform);

        // Add containers
        dbContext.ContainersInfo.AddRange(containers.Containers.Select(s => s.Value.Map(platform.Id, DateTimeOffset.UtcNow.ToUnixTimeSeconds())));


        // Save to db
        await dbContext.SaveChangesAsync(cancellationToken);

        // Sync jobs
        SyncTaskJobs(new PlatformData(platform.Id, platform.Address, platform.Status), cancellationToken: cancellationToken);

        logger.LogInformation("A new platform has been added, id = {PlatformId}", platform.Id);
        return Result.Success(platform);

    }

    private void SyncTaskJobs(PlatformData platformData, string? oldAddress = null, CancellationToken cancellationToken = default)
    {
        if (!string.IsNullOrEmpty(oldAddress))
        {
            grpcHealthMonitorJob.UntrackAddress(oldAddress);
            daemonEventJob.StopMonitoringPlatform(oldAddress);
            platformInfoJob.StopStreamStatsForPlatform(oldAddress);
            containersStatsJob.StopStreamStatsForPlatform(oldAddress);
        }

        grpcHealthMonitorJob.TrackAddress(platformData.Address);
        daemonEventJob.StartMonitoringPlatform(platformData, cancellationToken);
        platformInfoJob.StartStreamStatsForPlatform(platformData, cancellationToken);
        containersStatsJob.StartStreamStatsForPlatform(platformData, cancellationToken);
    }
}
