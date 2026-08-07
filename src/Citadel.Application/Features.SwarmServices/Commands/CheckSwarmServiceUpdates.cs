using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Domain.Entities.SwarmServices;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.SwarmServices.Commands;

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Write, ResourceIdProperty = nameof(Id))]
public sealed record CheckSwarmServiceUpdates(Guid Id) : ICommand<Result<SwarmService>>;

internal sealed class CheckSwarmServiceUpdatesHandler(
    IUnitOfWork unitOfWork,
    IImageCheckBuilder imageCheckBuilder,
    IImageDigestScanner imageDigestScanner,
    DeploymentUpdateEvaluator updateEvaluator,
    IUpdateCheckLeaseManager leaseManager,
    IUserContextAccessor userContext,
    ISwarmServiceStreamManager streamManager,
    TimeProvider timeProvider)
    : ICommandHandler<CheckSwarmServiceUpdates, Result<SwarmService>>
{
    public async ValueTask<Result<SwarmService>> Handle(
        CheckSwarmServiceUpdates command,
        CancellationToken cancellationToken)
    {
        if (!leaseManager.TryAcquire(ResourceType.SwarmService, command.Id, out var lease))
            return Result.Failure<SwarmService>(new ConflictError(
                "An update check is already running for this Service."));

        using (lease)
        {
            var service = await unitOfWork.SwarmServices.GetAsync(command.Id, cancellationToken);
            if (service is null)
                return Result.Failure<SwarmService>(new NotFoundError(
                    "The managed Swarm Service does not exist."));
            if (!userContext.Current.IsAdmin
                && userContext.Current.ActorId != Constants.SystemId
                && !await unitOfWork.Platforms.CanAccessAsync(
                    userContext.Current.UserId, service.PlatformId, cancellationToken))
                return Result.Failure<SwarmService>(new NotFoundError(
                    "The managed Swarm Service does not exist."));
            if (service.ControlState == ResourceControlState.Processing)
                return Result.Failure<SwarmService>(new ConflictError(
                    "The Service is currently processing another operation."));
            if (service.Spec.Image is not SwarmExternalImage image)
                return Result.Failure<SwarmService>(new BadRequestError(
                    "Only external tagged images support update checks."));
            if (!Helpers.TrySplitImageTag(image.ImageTag, out var repository, out var tag))
                return Result.Failure<SwarmService>(new BadRequestError(
                    "The Service image must use a supported tagged reference."));
            if (string.IsNullOrWhiteSpace(service.AppliedImageDigest))
                return Result.Failure<SwarmService>(new ConflictError(
                    "The Service has no applied image digest to compare."));

            var registry = await unitOfWork.Registries.GetAsync(image.RegistryId, cancellationToken);
            if (registry is null)
                return Result.Failure<SwarmService>(new BadRequestError(
                    "The Service's configured Registry does not exist."));
            var task = imageCheckBuilder.BuildScanTask(
                new ImageKey(image.RegistryId, repository, tag), service.PlatformId, registry);
            if (!task.IsSuccess(out var scanTask, out var error))
                return Result.Failure<SwarmService>(error!);

            var actorId = userContext.Current.ActorId == Guid.Empty
                ? Constants.SystemId
                : userContext.Current.ActorId;
            var startedAt = timeProvider.GetUtcNow();
            service.TryBeginUpdateCheck(actorId, startedAt);
            if (!await SaveAsync(service, cancellationToken))
                return Result.Failure<SwarmService>(new ConflictError(
                    "The Service changed before the update check could start."));

            var completed = false;
            try
            {
                service = await unitOfWork.SwarmServices.GetAsync(service.Id, cancellationToken) ?? service;
                await streamManager.SendSwarmServiceInfo(service);
                var scanned = await imageDigestScanner.ScanAsync(scanTask, cancellationToken);
                if (!scanned.IsSuccess(out var remoteDigest, out error))
                {
                    if (error is BadGatewayError)
                    {
                        service.CompleteUpdateCheck(new AutoUpdateState(
                            timeProvider.GetUtcNow().UtcDateTime,
                            AutoUpdateStatus.Failed,
                            service.AppliedImageDigest,
                            service.AutoUpdateState.RemoteDigest,
                            error.Message));
                        completed = await SaveAsync(service, cancellationToken);
                    }
                    return Result.Failure<SwarmService>(error!);
                }

                var evaluation = updateEvaluator.Evaluate(
                    service.AppliedImageDigest!, remoteDigest, timeProvider.GetUtcNow().UtcDateTime);
                service.CompleteUpdateCheck(evaluation.State);
                completed = await SaveAsync(service, cancellationToken);
                if (!completed)
                    return Result.Failure<SwarmService>(new ConflictError(
                        "The Service changed while the update check was running."));
                service = await unitOfWork.SwarmServices.GetAsync(service.Id, cancellationToken) ?? service;
                await streamManager.SendSwarmServiceInfo(service);
                return Result.Success(service);
            }
            finally
            {
                if (!completed)
                {
                    var current = await unitOfWork.SwarmServices.GetAsync(service.Id, CancellationToken.None);
                    if (current is not null
                        && current.ControlState == ResourceControlState.Processing
                        && current.ControlTriggeredBy == actorId
                        && current.ControlStartedAt == startedAt.ToUnixTimeSeconds()
                        && current.CurrentOperation is null)
                    {
                        current.CompleteUpdateCheck();
                        if (await SaveAsync(current, CancellationToken.None))
                            await streamManager.SendSwarmServiceInfo(current);
                    }
                }
            }
        }
    }

    private async Task<bool> SaveAsync(SwarmService service, CancellationToken cancellationToken)
    {
        if (await unitOfWork.SwarmServices.UpdateAsync(service, cancellationToken) == 0)
            return false;
        await unitOfWork.CommitAsync(cancellationToken);
        return true;
    }
}
