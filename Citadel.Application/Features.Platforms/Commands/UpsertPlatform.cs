using Infrastructure.Entities;
using Hosting.Common.ErrorTypes;
using FluentValidation;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;
using Hosting.Common;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Citadel.Common;
using Infrastructure.Services;
using Quartz;
using Infrastructure.TaskJobs;
using System.Net;
using Grpc.Core;

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
    ISchedulerFactory schedulerFactory,
    ApplicationDbContext dbContext,
    ICacheService cacheService,
    ILogger<UpsertPlatformHandler> logger)
    : ICommandHandler<UpsertPlatform, Result<Platform>>
{
    public async ValueTask<Result<Platform>> Handle(UpsertPlatform command, CancellationToken cancellationToken)
    {
        try
        {
            // Get platform system info
            var client = clientFactory.GetPlatformClient(command.Address);
            var platformInfo = await client.GetPlatformInfoAsync(new Google.Protobuf.WellKnownTypes.Empty(), cancellationToken: cancellationToken);

            var platform = (command.Id is null)
                    ? await CreatePlatform(command, platformInfo, cancellationToken)
                    : await UpdatePlatform(command, platformInfo, cancellationToken);

            // rebuild cache
            cacheService.DeleteClientsAddresses();

            return platform;
        }
        catch (RpcException ex)
        {
            return Result.Failure<Platform>(new ClientRpcException($"An RPC exception occurred: {ex.Message}"));
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "An error occurred while processing the request");
            return Result.Failure<Platform>(new InternalServerError("An error occurred while processing the request"));
        }
    }

    private async Task<Result<Platform>> CreatePlatform(UpsertPlatform command, PlatformInfoMessage platformInfo, CancellationToken cancellationToken)
    {
        // Check if the platform already exists
        if (await dbContext.Platforms.AsNoTracking()
                                    .FirstOrDefaultAsync(s => s.Address == command.Address || s.Name == command.Name, cancellationToken: cancellationToken) != null)
        {
            return Result.Failure<Platform>(new ConflictError("A platform with the same [Name] or [Address] already exists!"));
        }

        var platform = Platform.Create(
            name: command.Name, 
            address: command.Address,
            daemonId: platformInfo.Id,
            networksCount: platformInfo.NetworksCount ,
            volumesCount: platformInfo.VolumesCount,
            containers: platformInfo.Containers,
            containersRunning: platformInfo.ContainersRunning,
            containersPaused: platformInfo.ContainersPaused,
            containersStopped: platformInfo.ContainersStopped,
            images: platformInfo.Images,
            driver: platformInfo.Driver,
            operatingSystem: platformInfo.OperatingSystem,
            osVersion: platformInfo.OsVersion,
            osType: platformInfo.OsType,
            architecture: platformInfo.Architecture,
            ncpu: platformInfo.Ncpu,
            memTotal: platformInfo.MemTotal,
            serverVersion: platformInfo.ServerVersion,
            agentVersion: platformInfo.AgentVersion,
            swarmInfo: platformInfo.SwarmInfo.Map(),
            stats: [platformInfo.MapStat()]
            );
        dbContext.Platforms.Add(platform);

        // Save to db
        await dbContext.SaveChangesAsync(cancellationToken);

        // Re-queue job
        await EnqueueTaskJob(platform.Address, cancellationToken: cancellationToken);

        logger.LogInformation("A new platform has been added, id = {PlatformId}", platform.Id);
        return Result.Success(platform);
    }

    private async Task<Result<Platform>> UpdatePlatform(UpsertPlatform command, PlatformInfoMessage platformInfo, CancellationToken cancellationToken)
    {
        var platform = await dbContext.Platforms.FirstOrDefaultAsync(s => s.Id == command.Id, cancellationToken);
        if (platform == null)
        {
            Result.Failure<Platform>(new NotFoundError("The provided platform Id does not exists"));
        }

        var oldPlatformAddress = platform.Address;
        platform.PartialUpdate(
            name: command.Name,
            address: command.Address,
            daemonId: platformInfo.Id,
            networksCount: platformInfo.NetworksCount,
            volumesCount: platformInfo.VolumesCount,
            containers: platformInfo.Containers,
            containersRunning: platformInfo.ContainersRunning,
            containersPaused: platformInfo.ContainersPaused,
            containersStopped: platformInfo.ContainersStopped,
            images: platformInfo.Images,
            driver: platformInfo.Driver,
            operatingSystem: platformInfo.OperatingSystem,
            osVersion: platformInfo.OsVersion,
            osType: platformInfo.OsType,
            architecture: platformInfo.Architecture,
            ncpu: platformInfo.Ncpu,
            memTotal: platformInfo.MemTotal,
            serverVersion: platformInfo.ServerVersion,
            agentVersion: platformInfo.AgentVersion);

        // Save to db
        await dbContext.SaveChangesAsync(cancellationToken);

        // Re-queue job
        await EnqueueTaskJob(platform.Address, oldPlatformAddress, cancellationToken);

        logger.LogInformation("The platform with id = {PlatformId} has been updated", platform.Id);
        return Result.Success(platform);
    }

    private async Task EnqueueTaskJob(string newAddress, string oldAddress = null, CancellationToken cancellationToken = default)
    {
        var scheduler = await schedulerFactory.GetScheduler(cancellationToken);
        if (!string.IsNullOrEmpty(oldAddress))
        {
            await scheduler.AbortStreamDaemonEventJob(oldAddress, logger, cancellationToken);
        }
        await scheduler.EnqueueStreamDaemonEventJob(newAddress, logger, cancellationToken);
    }

}

internal static class Mapper 
{
    internal static SwarmInfo Map(this SwarmInfoMessage swarmInfo)
        => SwarmInfo.Create(
                nodeID: swarmInfo?.NodeID,
                nodeAddr: swarmInfo?.NodeAddr,
                localNodeState: swarmInfo?.LocalNodeState,
                controlAvailable: swarmInfo?.ControlAvailable ?? false,
                error: swarmInfo?.Error,
                nodes: swarmInfo?.Nodes ?? 0,
                managers: swarmInfo?.Managers ?? 0,
                remoteManagers: swarmInfo?.RemoteManagers?.Select(s => SwarmPeer.Create(nodeID: s.NodeID, addr: s.Addr))
                );

    internal static PlatformStat MapStat(this PlatformInfoMessage systemInfo)
        => PlatformStat.Create(
            memoryUsage: systemInfo.MemoryUsage,
            cpuUsage: systemInfo.CpuUsage,
            created: systemInfo.Created,
            rxBytes: systemInfo.RxBytes,
            txBytes: systemInfo.TxBytes
            );
}