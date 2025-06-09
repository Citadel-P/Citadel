using Citadel.Agent.Containers.V1;
using Citadel.Agent.Common.V1;
using EFCore.BulkExtensions;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Infrastructure;
using Infrastructure.Entities;
using Infrastructure.Entities.Platforms;
using Infrastructure.EntityFramework;
using Infrastructure.Services.Abstractions;
using Infrastructure.TaskJobs;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;

namespace Application.Features.Platforms.Commands;

[RequirePermission(nameof(AppPermission.Platform_Create))]
public sealed record CreatePlatform(string Name, string Address, PlatformType Type) : ICommand<Result<Platform>>
{
    internal class Validator : AbstractValidator<CreatePlatform>
    {
        public Validator()
        {
            RuleFor(x => x.Name).ValidNameIdentifier();
            RuleFor(x => x.Address).ValidHostOrIp();
        }
    }
}

internal sealed class CreatePlatformHandler(
    ApplicationDbContext dbContext,
    IGrpcClientFactory clientFactory,
    IPlatformHealthMonitorJob platformHealthMonitorJob,
    ILogger<PatchPlatformHandler> logger) : ICommandHandler<CreatePlatform, Result<Platform>>
{
    public async ValueTask<Result<Platform>> Handle(CreatePlatform command, CancellationToken cancellationToken)
    {
        // Check if the platform already exists
        if (await dbContext.Platforms.AsNoTracking()
                                    .FirstOrDefaultAsync(s => s.Address == command.Address || s.Name == command.Name, cancellationToken: cancellationToken) != null)
        {
            return Result.Failure<Platform>(new ConflictError("A platform with the same [Name] or [Address] already exists!"));
        }

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

        var platformInfoTsk = platformClient.ListPlatformInfoAsync(new Google.Protobuf.WellKnownTypes.Empty(), cancellationToken: cancellationToken);
        var containersTsk = containersClient.ListAsync(new ListContainersMessage { All = true }, cancellationToken: cancellationToken);
        var platformInfo = await platformInfoTsk;
        var containers = await containersTsk;

        DockerPlatformDescriptor descriptor = new (
            DaemonId: platformInfo.Id,
            ContainerCount: platformInfo.ContainerCount,
            ContainersRunning: platformInfo.ContainersRunning,
            ContainersPaused: platformInfo.ContainersPaused,
            ContainersStopped: platformInfo.ContainersStopped,
            Driver: platformInfo.Driver,
            OperatingSystem: platformInfo.OperatingSystem,
            OsVersion: platformInfo.OsVersion,
            OsType: platformInfo.OsType,
            Architecture: platformInfo.Architecture);

        var platform = new Platform (
            name: command.Name,
            type: command.Type,
            address: command.Address,
            status: PlatformStatus.Online,
            networkCount: platformInfo.NetworkCount,
            volumeCount: platformInfo.VolumeCount,
            imageCount: platformInfo.ImageCount,
            cpuCount: platformInfo.CpuCount,
            memTotal: platformInfo.MemTotal,
            serverVersion: platformInfo.ServerVersion,
            agentVersion: platformInfo.AgentVersion,
            platformDescriptor: descriptor
            )
            .AppendStat(platformInfo.MapStat());

        // Add platform
        dbContext.Platforms.Add(platform);
        await dbContext.SaveChangesAsync(cancellationToken);

        // Add containers
        var at = DateTimeOffset.UtcNow.ToUnixTimeSeconds();
        var mappedContainers = containers.Containers.Select(s => s.Value.Map(platform.Id, at)).ToArray();
        await dbContext.BulkInsertAsync(mappedContainers, cancellationToken: cancellationToken);

        // Start tracking the platform
        platformHealthMonitorJob.TrackPlatform(platform.Address, platform.Id);

        logger.LogInformation("A new platform has been added, id = {PlatformId}", platform.Id);
        return Result.Success(platform);
    }
}

internal static class Mapper
{
    internal static PlatformStat MapStat(this PlatformInfoMessage systemInfo)
        => new (
            created: systemInfo.Created,
            memoryUsage: systemInfo.PlatformStat?.MemoryUsage ?? 0,
            cpuUsage: systemInfo.PlatformStat?.CpuUsage ?? 0,
            rxBytes: systemInfo.PlatformStat?.RxBytes ?? 0,
            txBytes: systemInfo.PlatformStat?.TxBytes ?? 0
            );
}