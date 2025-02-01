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
            RuleFor(x => x.Name).NotEmpty().Length(4, 128);
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
            var systemInfo = await client.GetSystemInfoAsync(new Google.Protobuf.WellKnownTypes.Empty(), cancellationToken: cancellationToken);

            var platform = (command.Id is null)
                    ? await CreatePlatform(command, systemInfo, cancellationToken)
                    : await UpdatePlatform(command, systemInfo, cancellationToken);

            // rebuild cache
            cacheService.DeleteClientsAddresses();

            return platform;
        }
        catch (RpcException ex)
        {
            return Result.Failure<Platform>(new ClientRpcException($"An rpc exception occurred while processing the request {ex.Message}"));
        }
        catch (Exception ex)
        {
            logger.LogError(ex, "An error occurred while processing the request");
            return Result.Failure<Platform>(new InternalServerError("An error occurred while processing the request"));
        }
    }

    private async Task<Result<Platform>> CreatePlatform(UpsertPlatform command, SystemInfoMessage systemInfo, CancellationToken cancellationToken)
    {
        // Check if the platform already exists
        if (await dbContext.Platforms.AsNoTracking()
                                    .FirstOrDefaultAsync(s => s.Address == command.Address || s.Name == command.Name, cancellationToken: cancellationToken) != null)
        {
            return Result.Failure<Platform>(new ConflictError("A platform with the same [Name] or [Address] already exists!"));
        }

        var platform = Platform.Create(command.Name, command.Address, systemInfo.Map(), [systemInfo.MapStat()]);
        dbContext.Platforms.Add(platform);

        // Save to db
        await dbContext.SaveChangesAsync(cancellationToken);

        // Re-queue job
        await RequeuTaskJob(cancellationToken, platform.Address);

        logger.LogInformation("A new platform has been added, id = {PlatformId}", platform.Id);
        return Result.Success(platform);
    }

    private async Task<Result<Platform>> UpdatePlatform(UpsertPlatform command, SystemInfoMessage systemInfo, CancellationToken cancellationToken)
    {
        var platform = await dbContext.Platforms.FirstOrDefaultAsync(s => s.Id == command.Id, cancellationToken);
        if (platform == null)
        {
            Result.Failure<Platform>(new NotFoundError("The provided platform Id does not exists"));
        }

        var oldPlatformAddress = platform.Address;
        platform.PartialUpdate(command.Name, command.Address, systemInfo.Map());

        // Save to db
        await dbContext.SaveChangesAsync(cancellationToken);

        // Re-queue job
        await RequeuTaskJob(cancellationToken, platform.Address, oldPlatformAddress);

        logger.LogInformation("The platform with id = {PlatformId} has been updated", platform.Id);
        return Result.Success(platform);
    }

    private async Task RequeuTaskJob(CancellationToken cancellationToken, string newAddress, string? oldAddress = null)
    {
        var scheduler = await schedulerFactory.GetScheduler(cancellationToken);
        if (!string.IsNullOrEmpty(oldAddress))
        {
            await scheduler.AbortStreamDaemonEventJob(oldAddress, logger, cancellationToken);
        }
        await scheduler.EnqueueStreamDaemonEventJob(newAddress, logger, cancellationToken);
    }

}

internal static class PlatformMapper 
{
    internal static SystemInfo Map(this SystemInfoMessage systemInfo)
    {
        if (systemInfo.Id == null) 
        {
            throw new ArgumentNullException("Daemon Id is required");
        }

        var swarmInfo = SwarmInfo.Create(
                nodeID: systemInfo.SwarmInfo?.NodeID,
                nodeAddr: systemInfo.SwarmInfo?.NodeAddr,
                localNodeState: systemInfo.SwarmInfo?.LocalNodeState,
                controlAvailable: systemInfo.SwarmInfo?.ControlAvailable ?? false,
                error: systemInfo.SwarmInfo?.Error,
                nodes: systemInfo.SwarmInfo?.Nodes ?? 0,
                managers: systemInfo.SwarmInfo?.Managers ?? 0,
                remoteManagers: systemInfo.SwarmInfo?.RemoteManagers?.Select(s => SwarmPeer.Create(nodeID: s.NodeID, addr: s.Addr))
                );

        return SystemInfo.Create(
            daemonId: systemInfo.Id,
            networksCount: systemInfo.NetworksCount,
            volumesCount: systemInfo.VolumesCount,
            containers: systemInfo.Containers,
            containersRunning: systemInfo.ContainersRunning,
            containersStopped: systemInfo.ContainersStopped,
            containersPaused: systemInfo.ContainersPaused,
            images: systemInfo.Images,
            driver: systemInfo.Driver,
            operatingSystem: systemInfo.OperatingSystem,
            osVersion: systemInfo.OsVersion,
            osType: systemInfo.OsType,
            architecture: systemInfo.Architecture,
            ncpu: systemInfo.Ncpu,
            serverVersion: systemInfo.ServerVersion,
            memTotal: systemInfo.MemTotal,
            agentVersion: systemInfo.AgentVersion,
            swarmInfo: swarmInfo
            );
    }

    internal static PlatformStat MapStat(this SystemInfoMessage systemInfo)
    {
        return PlatformStat.Create(
            memoryUsage: systemInfo.MemoryUsage,
            cpuUsage: systemInfo.CpuUsage,
            created: systemInfo.Created,
            rxBytes: systemInfo.RxBytes,
            txBytes: systemInfo.TxBytes
            );
    }
}