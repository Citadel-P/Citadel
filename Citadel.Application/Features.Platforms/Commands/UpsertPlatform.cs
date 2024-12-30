using Infrastructure.Entities;
using Hosting.Common.ErrorTypes;
using FluentValidation;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;
using Hosting.Common;
using Infrastructure.EntityFramework;
using Infrastructure.Services;

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
    IAgentService agentService,
    ApplicationDbContext dbContext,
    ILogger<UpsertPlatformHandler> logger)
    : ICommandHandler<UpsertPlatform, Result<Platform>>
{
    public async ValueTask<Result<Platform>> Handle(UpsertPlatform command, CancellationToken cancellationToken)
    {
        return (command.Id is null)
                ? await CreatePlatform(command, cancellationToken)
                : await UpdatePlatform(command, cancellationToken);
    }

    private async Task<Result<Platform>> CreatePlatform(UpsertPlatform command, CancellationToken cancellationToken)
    {
        // Check if the platform already exists
        if (await dbContext.Platforms.AsNoTracking()
                                    .FirstOrDefaultAsync(s => s.Address == command.Address || s.Name == command.Name, cancellationToken: cancellationToken) != null)
        {
            return Result.Failure<Platform>(new ConflictError("A platform with the same [Name] or [Address] already exists!"));
        }

        // Try to get platform system info
        var result = await agentService.GetSystemInfo(command.Address, cancellationToken);
        if (!result.IsSuccess(out var systemInfoView))
        {
            result.IsFailure(out var error);
            return Result.Failure<Platform>(error);
        }

        var platform = Platform.Create(command.Name, command.Address, systemInfoView.Map(), [systemInfoView.MapStat()]);
        dbContext.Platforms.Add(platform);

        // Save to db
        await dbContext.SaveChangesAsync(cancellationToken);

        logger.LogInformation("A new platform has been added, id = {PlatformId}", platform.Id);
        return Result.Success(platform);
    }

    private async Task<Result<Platform>> UpdatePlatform(UpsertPlatform command, CancellationToken cancellationToken)
    {
        var platform = await dbContext.Platforms.FirstOrDefaultAsync(s => s.Id == command.Id, cancellationToken);
        if (platform == null)
        {
            Result.Failure<Platform>(new NotFoundError("The provided platform Id does not exists"));
        }

        platform.PartialUpdate(command.Name, command.Address);

        // Save to db
        await dbContext.SaveChangesAsync(cancellationToken);

        logger.LogInformation("The platform with id = {PlatformId} has been updated", platform.Id);
        return Result.Success(platform);
    }
}

internal static class PlatformMapper 
{
    internal static SystemInfo Map(this Infrastructure.SystemInfoView systemInfoView)
    {
        

        var swarmInfo = SwarmInfo.Create(
                nodeID: systemInfoView.Swarm.NodeID,
                nodeAddr: systemInfoView.Swarm.NodeAddr,
                localNodeState: systemInfoView.Swarm.LocalNodeState,
                controlAvailable: systemInfoView.Swarm.ControlAvailable,
                error: systemInfoView.Swarm.Error,
                nodes: systemInfoView.Swarm.Nodes,
                managers: systemInfoView.Swarm.Managers,
                remoteManagers: systemInfoView.Swarm?.RemoteManagers?.Select(s => SwarmPeer.Create(nodeID: s.NodeID, addr: s.Addr))
                );

        return SystemInfo.Create(
            daemonId: systemInfoView.DaemonId,
            networksCount: systemInfoView.NetworksCount,
            volumesCount: systemInfoView.VolumesCount,
            containers: systemInfoView.Containers,
            containersRunning: systemInfoView.ContainersRunning,
            containersStopped: systemInfoView.ContainersStopped,
            containersPaused: systemInfoView.ContainersPaused,
            images: systemInfoView.Images,
            driver: systemInfoView.Driver,
            operatingSystem: systemInfoView.OperatingSystem,
            osVersion: systemInfoView.OsVersion,
            osType: systemInfoView.OsType,
            architecture: systemInfoView.Architecture,
            ncpu: systemInfoView.Ncpu,
            serverVersion: systemInfoView.ServerVersion,
            memTotal: systemInfoView.MemTotal,
            agentVersion: systemInfoView.AgentVersion,
            swarmInfo: swarmInfo
            );
    }

    internal static PlatformStat MapStat(this Infrastructure.SystemInfoView systemInfoView)
    {
        return PlatformStat.Create(
            memoryUsage: systemInfoView.MemoryUsage,
            cpuUsage: systemInfoView.CpuUsage,
            created: systemInfoView.Created,
            rxBytes: systemInfoView.RxBytes.Value,
            txBytes: systemInfoView.TxBytes.Value
            );
    }
}