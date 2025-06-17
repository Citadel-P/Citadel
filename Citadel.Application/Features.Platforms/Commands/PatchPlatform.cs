using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Platforms;
using Domain.Entities;
using Domain.Entities.Platforms;
using FluentValidation;
using Grpc.Core;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.MergePatch;
using Application.TaskJobs;
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
            RuleFor(x => x.ConnectorType)
                .Must(x => Enum.IsDefined(x))
                .WithMessage("'{PropertyName}' must be a valid type");
        }
    }
}

internal class PatchPlatformHandler(
    IUnitOfWork unitOfWork,
    IPlatformHealthMonitorJob platformHealthMonitorJob,
    IConnectorFactory<IPlatformConnector> platformConnectorFactory,
    ILogger<PatchPlatformHandler> logger): ICommandHandler<PatchPlatform, Result<Platform>>
{
    public async ValueTask<Result<Platform>> Handle(PatchPlatform command, CancellationToken cancellationToken)
    {
        try
        {
            var platform = await unitOfWork.Platforms.Query().FirstOrDefaultAsync(s => s.Id == command.Id, cancellationToken);
            if (platform == null)
            {
                return Result.Failure<Platform>(new NotFoundError("The provided Id does not exist"));
            }

            var patchedPlatform = command.Patch.ApplyTo(platform, PlatformJsonContext.Default.Platform);

            if (patchedPlatform.Name != null)
            {
                var conflict = await unitOfWork.Platforms.Query().AsNoTracking().FirstOrDefaultAsync(s => s.Name == patchedPlatform.Name && s.Id != command.Id, cancellationToken);
                if (conflict != null)
                {
                    return Result.Failure<Platform>(new ConflictError("A platform with the same name already exist"));
                }
            }

            if (patchedPlatform.PlatformDescriptor is DockerPlatformDescriptor)
            {
                var oldPlatformAddress = platform.Address;
                var param = new GetPlatformCommand
                (
                    PlatformAddress: patchedPlatform.Address,
                    PlatformName: patchedPlatform.Name ?? platform.Name
                );

                var platformConnector = platformConnectorFactory.GetConnector(patchedPlatform.ConnectorType);
                var platformResult = await platformConnector.GetPlatformAsync(param, cancellationToken);
                if (!platformResult.IsSuccess(out var platformInfo, out var error))
                {
                    logger.LogError("Failed to get platform info for {Address}: {Error}", patchedPlatform.Address, error?.Message);
                    return platformResult;
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
                    descriptor: platformInfo.PlatformDescriptor);

                await unitOfWork.SaveChangesAsync(cancellationToken);
                await UpdatePlatformTracking(platform.Id, platform.Address, platform.ConnectorType, oldPlatformAddress, cancellationToken);
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

    private async Task UpdatePlatformTracking(Guid id, string address, PlatformConnectorType type, string oldAddress, CancellationToken cancellationToken = default)
    {
        var removed = await platformHealthMonitorJob.UntrackPlatform(oldAddress, cancellationToken);
        if (removed)
        {
            platformHealthMonitorJob.TrackPlatform(address, id, type);
        }
    }
}