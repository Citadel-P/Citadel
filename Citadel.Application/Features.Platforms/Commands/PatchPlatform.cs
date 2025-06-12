using FluentValidation;
using Grpc.Core;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using Domain;
using Domain.Entities;
using Domain.Entities.Platforms;
using Infrastructure.EntityFramework;
using Infrastructure.EntityFramework.Configurations;
using Infrastructure.Services.Abstractions;
using Infrastructure.TaskJobs;
using LightResults;
using Mediator;
using Microsoft.EntityFrameworkCore;
using Microsoft.Extensions.Logging;

namespace Application.Features.Platforms.Commands;

[RequirePermission(nameof(AppPermission.Platform_Update))]
public sealed record PatchPlatform(Guid? Id, JsonMergePatchDocument<Platform> Patch) : ICommand<Result<Platform>>
{
    internal sealed class Validator : PatchCommandValidator<PatchPlatform, Platform>
    {
        public Validator()
            : base(
                  patchSelector: x => x.Patch,
                  jsonTypeInfo: PlatformJsonContext.Default.Platform,
                  modelValidator: new PlatformValidator()
                  )
        { }
    }

    internal class PlatformValidator : AbstractValidator<Platform>
    {
        public PlatformValidator()
        {
            When(x => x.Name is not null, () => RuleFor(x => x.Name).ValidNameIdentifier());
            When(x => x.Address is not null, () => RuleFor(x => x.Address).ValidHostOrIp());
            RuleFor(x => x.Type)
                .Must(x => Enum.IsDefined(x))
                .WithMessage("'{PropertyName}' must be a valid type");
        }
    }
}

internal class PatchPlatformHandler(
    ApplicationDbContext dbContext,
    IGrpcClientFactory clientFactory,
    IPlatformHealthMonitorJob platformHealthMonitorJob,
    ILogger<PatchPlatformHandler> logger)
    : ICommandHandler<PatchPlatform, Result<Platform>>
{
    public async ValueTask<Result<Platform>> Handle(PatchPlatform command, CancellationToken cancellationToken)
    {
        try
        {
            var platform = await dbContext.Platforms.FirstOrDefaultAsync(s => s.Id == command.Id, cancellationToken);
            if (platform == null)
            {
                return Result.Failure<Platform>(new NotFoundError("The provided Id does not exist"));
            }

            var patchedPlatform = command.Patch.ApplyTo(platform, PlatformJsonContext.Default.Platform);

            if (patchedPlatform.Name != null)
            {
                var conflict = await dbContext.Platforms.AsNoTracking().FirstOrDefaultAsync(s => s.Name == patchedPlatform.Name && s.Id != command.Id, cancellationToken);
                if (conflict != null)
                {
                    return Result.Failure<Platform>(new ConflictError("A platform with the same name already exist"));
                }
            }

            if (patchedPlatform.Type == PlatformType.Docker)
            {
                var platformClient = clientFactory.GetPlatformClient(patchedPlatform.Address);
                var platformInfo = await platformClient.ListPlatformInfoAsync(new Google.Protobuf.WellKnownTypes.Empty(), cancellationToken: cancellationToken);

                var oldPlatformAddress = platform.Address;

                PlatformDescriptor? descriptor = null;
                if (platform.PlatformDescriptor is DockerPlatformDescriptor dockerDescriptor)
                {
                    descriptor = dockerDescriptor.Create(
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
                else if (platform.PlatformDescriptor is DockerSwarmPlatformDescriptor swarmDescriptor)
                {
                    // Todo
                }
                else if (platform.PlatformDescriptor is KubernetesPlatformDescriptor k8sDescriptor)
                {
                    // Todo
                }

                platform.PartialUpdate(
                    name: patchedPlatform.Name,
                    address: patchedPlatform.Address,
                    networkCount: platformInfo.NetworkCount,
                    volumeCount: platformInfo.VolumeCount,
                    imageCount: platformInfo.ImageCount,
                    cpuCount: platformInfo.CpuCount,
                    memTotal: platformInfo.MemTotal,
                    serverVersion: platformInfo.ServerVersion,
                    agentVersion: platformInfo.AgentVersion,
                    descriptor: descriptor);

                await dbContext.SaveChangesAsync(cancellationToken);
                await UpdatePlatformTracking(platform.Id, platform.Address, oldPlatformAddress, cancellationToken);
                logger.LogInformation("The platform with id = {PlatformId} has been updated", platform.Id);
            }

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

    private async Task UpdatePlatformTracking(
        Guid id,
        string address,
        string oldAddress,
        CancellationToken cancellationToken = default)
    {
        var removed = await platformHealthMonitorJob.UntrackPlatform(oldAddress, cancellationToken);
        if (removed)
        {
            platformHealthMonitorJob.TrackPlatform(address, id);
        }
    }
}