using Citadel.Agent.Common.V1;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Containers;
using Domain.Entities;
using Domain.Entities.Platforms;
using EFCore.BulkExtensions;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
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
    IContainerConnector containerConnector,
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

        var platformInfo = await platformClient.ListPlatformInfoAsync(new Google.Protobuf.WellKnownTypes.Empty(), cancellationToken: cancellationToken);
        

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
        var containers = await GetContainers(platform, cancellationToken);
        if (containers != null && containers.Any()) 
        {
            await dbContext.BulkInsertAsync(containers, cancellationToken: cancellationToken);
        }

        // Start tracking the platform
        platformHealthMonitorJob.TrackPlatform(platform.Address, platform.Id);

        logger.LogInformation("A new platform has been added, id = {PlatformId}", platform.Id);
        return Result.Success(platform);
    }

    private async Task<IEnumerable<Container>> GetContainers(Platform platform, CancellationToken cancellationToken)
    {
        var command = new ContainerFilterCommand
            (
                PlatformId: platform.Id,
                PlatformAddress: platform.Address, 
                All: true
            );

        var containersResult = await containerConnector.ListContainersAsync(command, cancellationToken: cancellationToken);

        if (!containersResult.IsSuccess(out var containers))
        {
            containersResult.IsFailure(out var error);
            logger.LogError("Failed to list containers for platform {Address}: {Error}", platform.Address, error?.Message);
            return [];
        }

        else return containers.Values;
    }
}

internal static class Mapper
{
    internal static PlatformStat MapStat(this PlatformInfoResponse systemInfo)
        => new (
            created: systemInfo.Created,
            memoryUsage: systemInfo.PlatformStat?.MemoryUsage ?? 0,
            cpuUsage: systemInfo.PlatformStat?.CpuUsage ?? 0,
            rxBytes: systemInfo.PlatformStat?.RxBytes ?? 0,
            txBytes: systemInfo.PlatformStat?.TxBytes ?? 0
            );
}