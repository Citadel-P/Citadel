using Application.Services;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Commands;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Write)]
public sealed record CheckDeploymentUpdates(Guid DeploymentId) : ICommand<Result<Deployment>>;

internal sealed class CheckDeploymentUpdatesHandler(
    IUnitOfWork unitOfWork,
    IImageCheckBuilder imageCheckBuilder,
    IImageDigestScanner imageDigestScanner,
    DeploymentUpdateEvaluator updateEvaluator,
    IUpdateCheckLeaseManager leaseManager,
    IUserContextAccessor userContext,
    IDeploymentStreamManager deploymentStreamManager,
    INotificationQueue notificationQueue,
    TimeProvider timeProvider)
    : ICommandHandler<CheckDeploymentUpdates, Result<Deployment>>
{
    public async ValueTask<Result<Deployment>> Handle(
        CheckDeploymentUpdates command,
        CancellationToken cancellationToken)
    {
        if (!leaseManager.TryAcquire(
                ResourceType.Deployment,
                command.DeploymentId,
                out var lease))
        {
            return Result.Failure<Deployment>(
                new ConflictError("An update check is already running for this resource."));
        }

        using (lease)
        {
            var deployment = await unitOfWork.Deployments.GetAsync(
                command.DeploymentId,
                cancellationToken);
            if (deployment is null)
            {
                return Result.Failure<Deployment>(
                    new NotFoundError("The provided deployment does not exist."));
            }

            var checkResult = imageCheckBuilder.BuildDeploymentCheck(
                deployment,
                ImageCheckMode.OnDemand);
            if (checkResult.IsFailure(out var checkError, out var check))
            {
                return Result.Failure<Deployment>(checkError);
            }

            var registry = await unitOfWork.Registries.GetAsync(
                check.Key.RegistryId,
                cancellationToken);
            if (registry is null)
            {
                return Result.Failure<Deployment>(
                    new BadRequestError("The deployment's configured registry does not exist."));
            }

            var taskResult = imageCheckBuilder.BuildScanTask(
                check.Key,
                deployment.PlatformId,
                registry);
            if (taskResult.IsFailure(out var taskError, out var scanTask))
            {
                return Result.Failure<Deployment>(taskError);
            }

            var claimResult = await ClaimUpdateCheckAsync(deployment, cancellationToken);
            if (claimResult.IsFailure(out var claimError, out var claim))
            {
                return Result.Failure<Deployment>(claimError);
            }

            var operationCompleted = false;
            try
            {
                await deploymentStreamManager.SendDeploymentInfo(
                    CreateProcessingSnapshot(deployment, claim));

                var scanResult = await imageDigestScanner.ScanAsync(scanTask, cancellationToken);
                if (scanResult.IsFailure(out var scanError, out var remoteDigest))
                {
                    if (scanError is BadGatewayError)
                    {
                        var failedState = CreateFailedState(deployment, check, scanError);
                        operationCompleted = await CompleteUpdateCheckAsync(
                            deployment,
                            failedState,
                            claim.RowVersion,
                            cancellationToken);
                        if (!operationCompleted)
                        {
                            return ChangedDuringCheck();
                        }
                    }

                    return Result.Failure<Deployment>(scanError);
                }

                var checkedAt = timeProvider.GetUtcNow().UtcDateTime;
                var evaluation = updateEvaluator.Evaluate(
                    check.DeployedImage.ResolvedDigest!,
                    remoteDigest,
                    checkedAt);
                operationCompleted = await CompleteUpdateCheckAsync(
                    deployment,
                    evaluation.State,
                    claim.RowVersion,
                    cancellationToken);
                if (!operationCompleted)
                {
                    return ChangedDuringCheck();
                }

                return Result.Success(deployment);
            }
            finally
            {
                if (!operationCompleted)
                {
                    await ReleaseUpdateCheckAsync(
                        deployment,
                        claim,
                        CancellationToken.None);
                }
            }
        }
    }

    private async Task<Result<UpdateCheckClaim>> ClaimUpdateCheckAsync(
        Deployment deployment,
        CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId == Guid.Empty
            ? Constants.SystemId
            : userContext.Current.ActorId;
        var startedAt = timeProvider.GetUtcNow().ToUnixTimeSeconds();

        var affectedRows = await unitOfWork.Deployments.UpdateProcessingAsync(
            deployment.Id,
            deployment.Status,
            ResourceControlState.Processing,
            startedAt,
            deployment.RowVersion,
            checkRowVersion: true,
            actorId,
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        if (affectedRows == 0)
        {
            return Result.Failure<UpdateCheckClaim>(
                new ConflictError("The deployment changed before the update check could start."));
        }

        return Result.Success(new UpdateCheckClaim(
            deployment.RowVersion + 1,
            startedAt,
            actorId));
    }

    private async Task<bool> CompleteUpdateCheckAsync(
        Deployment deployment,
        AutoUpdateState state,
        long operationRowVersion,
        CancellationToken cancellationToken)
    {
        var affectedRows = await unitOfWork.Deployments.TryCompleteUpdateCheckAsync(
            deployment.Id,
            state,
            operationRowVersion,
            deployment.PlatformId,
            deployment.Status,
            deployment.Spec!,
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        if (affectedRows == 0)
        {
            return false;
        }

        deployment.SetAutoUpdateState(state);
        await NotifyAsync(deployment, cancellationToken);
        return true;
    }

    private async Task<bool> ReleaseUpdateCheckAsync(
        Deployment deployment,
        UpdateCheckClaim claim,
        CancellationToken cancellationToken)
    {
        var affectedRows = await unitOfWork.Deployments.TryReleaseUpdateCheckAsync(
            deployment.Id,
            claim.StartedAt,
            claim.ActorId,
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);
        if (affectedRows == 0)
        {
            return false;
        }

        await NotifyAsync(deployment, cancellationToken);
        return true;
    }

    private static Deployment CreateProcessingSnapshot(
        Deployment deployment,
        UpdateCheckClaim claim)
    {
        var snapshot = Deployment.FromPersistence(
            deployment.Id,
            deployment.Name,
            deployment.PlatformId,
            claim.RowVersion,
            claim.StartedAt,
            ResourceControlState.Processing,
            deployment.Status,
            deployment.CreatedAt,
            deployment.CreatedByActorId,
            claim.ActorId,
            deployment.Description,
            deployment.AutoUpdateState,
            deployment.Spec,
            deployment.Platform,
            deployment.Image,
            deployment.Container,
            deployment.LatestActivityEvent);
        snapshot.AssignTags(deployment.Tags);
        return snapshot;
    }

    private AutoUpdateState CreateFailedState(
        Deployment deployment,
        DeploymentImageCheck check,
        IError scanError)
    {
        var previous = deployment.AutoUpdateState;
        return new AutoUpdateState(
            LastCheckedAt: timeProvider.GetUtcNow().UtcDateTime,
            Status: AutoUpdateStatus.Failed,
            CurrentDigest: previous?.CurrentDigest ?? check.DeployedImage.ResolvedDigest,
            RemoteDigest: previous?.RemoteDigest,
            LastError: scanError.Message);
    }

    private ValueTask NotifyAsync(Deployment deployment, CancellationToken cancellationToken)
        => notificationQueue.EnqueueAsync(
            new DeploymentNotificationWorkItem(deploymentStreamManager, deployment),
            cancellationToken);

    private static Result<Deployment> ChangedDuringCheck()
        => Result.Failure<Deployment>(
            new ConflictError(
                "The deployment changed while the update check was running. Run the check again."));

    private sealed record UpdateCheckClaim(
        long RowVersion,
        long StartedAt,
        Guid ActorId);
}
