using Agent.Server.Containers;
using Citadel.Common;
using FluentValidation;
using Grpc.Core;
using Hosting.Common;
using Hosting.Common.ErrorTypes;
using Infrastructure.Entities;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Infrastructure.TaskJobs;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;

namespace Application.Features.Platforms.Commands;

/// <summary>
/// Create or update a platform
/// </summary>
public sealed record UpsertPlatform(Guid? Id, string Name, string Address) : ICommand<Result<Platform>>
{
    internal class Validator : AbstractValidator<UpsertPlatform>
    {
        public Validator()
        {
            RuleFor(x => x.Name).NotEmpty().ValidNameIdentifier();
            RuleFor(x => x.Address).ValidHostOrIp();
        }
    }
}

internal class UpsertPlatformHandler(
    IGrpcClientFactory clientFactory,
    IDaemonEventJob daemonEventJob,
    IPlatformsStatsReaderJob platformInfoJob,
    IContainersStatsReaderJob containersStatsJob,
    ApplicationDbContext dbContext,
    IGrpcHealthMonitorJob grpcHealthMonitorJob,
    ILogger<UpsertPlatformHandler> logger)
    : ICommandHandler<UpsertPlatform, Result<Platform>>
{
    public async ValueTask<Result<Platform>> Handle(UpsertPlatform command, CancellationToken cancellationToken)
    {
        try
        {
            // Get platform and containers info
            var platformClient = clientFactory.GetPlatformClient(command.Address);
            var containersClient = clientFactory.GetContainerClient(command.Address);

            var platformInfoTsk = platformClient.GetPlatformInfoAsync(new Google.Protobuf.WellKnownTypes.Empty(), cancellationToken: cancellationToken);
            var containersTsk = containersClient.ListContainersAsync(new ContainersListMessage { All = true }, cancellationToken: cancellationToken);
            var platformInfo = await platformInfoTsk;
            var containers = await containersTsk;

            var platform = (command.Id is null)
                    ? await CreatePlatform(command, platformInfo, containers, cancellationToken)
                    : await UpdatePlatform(command, platformInfo, containers, cancellationToken);

            return platform;
        }
        catch (RpcException ex)
        {
            return Result.Failure<Platform>(new ClientRpcException($"An RPC exception occurred: {ex.Message}", ex.StatusCode));
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "An error occurred while processing the request");
            return Result.Failure<Platform>(new InternalServerError("An error occurred while processing the request"));
        }
    }

    private async Task<Result<Platform>> CreatePlatform(UpsertPlatform command, PlatformInfoMessage platformInfo, ContainersListReply containers, CancellationToken cancellationToken)
    {
        // Check if the platform already exists
        if (await dbContext.Platforms.AsNoTracking()
                                    .FirstOrDefaultAsync(s => s.Address == command.Address || s.Name == command.Name, cancellationToken: cancellationToken) != null)
        {
            return Result.Failure<Platform>(new ConflictError("A platform with the same [Name] or [Address] already exists!"));
        }

        DockerPlatformDescriptor configuration = new (
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
            descriptor: configuration,
            stats: [platformInfo.MapStat()]
            );
        dbContext.Platforms.Add(platform);

        // Add containers
        dbContext.ContainersInfo.AddRange(containers.Containers.Select(s => s.Value.Map(platform.Id, DateTimeOffset.UtcNow.ToUnixTimeSeconds())));


        // Save to db
        await dbContext.SaveChangesAsync(cancellationToken);

        // Enqueue job
        SyncTaskJobs(new PlatformData (platform.Id, platform.Address, platform.Status), cancellationToken: cancellationToken);

        logger.LogInformation("A new platform has been added, id = {PlatformId}", platform.Id);
        return Result.Success(platform);
    }

    private async Task<Result<Platform>> UpdatePlatform(UpsertPlatform command, PlatformInfoMessage platformInfo, ContainersListReply containers, CancellationToken cancellationToken)
    {
        var platform = await dbContext.Platforms.FirstOrDefaultAsync(s => s.Id == command.Id, cancellationToken);
        if (platform == null)
        {
            return Result.Failure<Platform>(new NotFoundError("The provided platform Id does not exists"));
        }

        var oldPlatformAddress = platform.Address;
        platform.PartialUpdate(
            name: command.Name,
            address: command.Address,
            
            networkCount: platformInfo.NetworkCount,
            volumeCount: platformInfo.VolumeCount,
            containersRunning: platformInfo.ContainersRunning,
            containersPaused: platformInfo.ContainersPaused,
            containersStopped: platformInfo.ContainersStopped,
            imageCount: platformInfo.ImageCount,
            cpuCount: platformInfo.CpuCount,
            memTotal: platformInfo.MemTotal,
            serverVersion: platformInfo.ServerVersion,
            agentVersion: platformInfo.AgentVersion);

        if (platform.PlatformDescriptor is DockerPlatformDescriptor dockerConfig)
        {
            dockerConfig.PartialUpdate(
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
        }
        else if (platform.PlatformDescriptor is DockerSwarmPlatformDescriptor dockerSwarmConfig)
        {
        }

        // Save to db
        await dbContext.SaveChangesAsync(cancellationToken);

        // Re-queue job
        SyncTaskJobs(new PlatformData(platform.Id, platform.Address, platform.Status), oldPlatformAddress, cancellationToken);

        logger.LogInformation("The platform with id = {PlatformId} has been updated", platform.Id);
        return platform;
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

internal static class Mapper 
{
    internal static PlatformStat MapStat(this PlatformInfoMessage systemInfo)
        => PlatformStat.Create(
            created: systemInfo.Created,
            memoryUsage: systemInfo.PlatformStat.MemoryUsage,
            cpuUsage: systemInfo.PlatformStat.CpuUsage,
            rxBytes: systemInfo.PlatformStat.RxBytes,
            txBytes: systemInfo.PlatformStat.TxBytes
            );
}