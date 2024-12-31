using Application.Services.Abstractions;
using Infrastructure.EntityFramework;
using LightResults;
using Mediator;
using Infrastructure.Entities;
using Microsoft.EntityFrameworkCore;
using Hosting.Common.ErrorTypes;
using Application.Features.Platforms.Models;

namespace Application.Features.Platforms.Commands;

public sealed record class UpdateSystemInfo(SystemInfoRequest Input) : ICommand<Result>;
internal class PostSystemInfoHandler(
    ApplicationDbContext dbContext,
    IPlatformHubDispatcher platformHub) : ICommandHandler<UpdateSystemInfo, Result>
{
    public async ValueTask<Result> Handle(UpdateSystemInfo command, CancellationToken cancellationToken)
    {
        // Get the platform from db
        Platform platform = await dbContext.Platforms
                                    .Include(s => s.SystemInfo)
                                    .ThenInclude(s => s.SwarmInfo)
                                    .ThenInclude(s => s.RemoteManagers)
                                    .FirstOrDefaultAsync(s => s.SystemInfo.DaemonId == command.Input.Id, cancellationToken);
        if (platform is null)
        {
            return Result.Failure(new NotFoundError($"No platform has been found for id={command.Input.Id}, please reconnect this platform ")); ;
        }

        platform.SystemInfo.PartialUpdate(
            networksCount: command.Input.NetworksCount,
            volumesCount: command.Input.VolumesCount,
            containers: command.Input.Containers,
            containersRunning: command.Input.ContainersRunning,
            containersPaused: command.Input.ContainersPaused,
            containersStopped: command.Input.ContainersStopped,
            images: command.Input.Images,
            ncpu: command.Input.Ncpu,
            memTotal: command.Input.MemTotal,
            serverVersion: command.Input.ServerVersion,
            agentVersion: command.Input.AgentVersion,
            osType: command.Input.OsType,
            osVersion: command.Input.OsVersion,
            operatingSystem: command.Input.OperatingSystem,
            driver: command.Input.Driver
            );

        // Insert the platform stats
        var stat = PlatformStat.Create(
            memoryUsage: command.Input.MemoryUsage,
            cpuUsage: command.Input.CpuUsage,
            created: command.Input.Created,
            rxBytes: command.Input.RxBytes.Value,
            txBytes: command.Input.TxBytes.Value,
            platformId: platform.Id);

        dbContext.PlatformStats.Add(stat);
        await dbContext.SaveChangesAsync(cancellationToken);

        // Notify client(s)
        await platformHub.SendPlatformUpdated(platform);

        return Result.Success();
    }
}