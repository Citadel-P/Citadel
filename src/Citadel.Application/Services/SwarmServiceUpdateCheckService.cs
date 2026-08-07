using Application.Services.Licensing;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Domain.Entities.SwarmServices;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.ErrorTypes;
using LightResults;

namespace Application.Services;

internal sealed record SwarmServiceUpdateCheckResult(
    SwarmService Service,
    bool UpdateAvailable,
    bool ApplyStarted,
    string Reason);

internal interface ISwarmServiceUpdateCheckService
{
    Task<Result<SwarmServiceUpdateCheckResult>> CheckAsync(
        Guid serviceId,
        Guid actorId,
        bool applyWhenAvailable,
        CancellationToken cancellationToken);
}

internal sealed class SwarmServiceUpdateCheckService(
    IUnitOfWork unitOfWork,
    IImageCheckBuilder imageCheckBuilder,
    IImageDigestScanner imageDigestScanner,
    DeploymentUpdateEvaluator updateEvaluator,
    IUpdateCheckLeaseManager leaseManager,
    ISwarmServiceMutationService mutationService,
    ILicenseEntitlementService entitlementService,
    ISwarmServiceStreamManager streamManager,
    TimeProvider timeProvider) : ISwarmServiceUpdateCheckService
{
    public async Task<Result<SwarmServiceUpdateCheckResult>> CheckAsync(
        Guid serviceId,
        Guid actorId,
        bool applyWhenAvailable,
        CancellationToken cancellationToken)
    {
        if (!leaseManager.TryAcquire(ResourceType.SwarmService, serviceId, out var lease))
            return Result.Failure<SwarmServiceUpdateCheckResult>(new ConflictError(
                "An update check is already running for this Service."));

        using (lease)
        {
            var service = await unitOfWork.SwarmServices.GetAsync(serviceId, cancellationToken);
            if (service is null)
                return Result.Failure<SwarmServiceUpdateCheckResult>(new NotFoundError(
                    "The managed Swarm Service does not exist."));
            if (service.ControlState == ResourceControlState.Processing)
                return Result.Failure<SwarmServiceUpdateCheckResult>(new ConflictError(
                    "The Service is currently processing another operation."));
            if (service.Spec.Image is not SwarmExternalImage image)
                return Result.Failure<SwarmServiceUpdateCheckResult>(new BadRequestError(
                    "Only external tagged images support update checks."));
            if (!Helpers.TrySplitImageTag(image.ImageTag, out var repository, out var tag))
                return Result.Failure<SwarmServiceUpdateCheckResult>(new BadRequestError(
                    "The Service image must use a supported tagged reference."));
            if (string.IsNullOrWhiteSpace(service.AppliedImageDigest))
                return Result.Failure<SwarmServiceUpdateCheckResult>(new ConflictError(
                    "The Service has no applied image digest to compare."));

            var registry = await unitOfWork.Registries.GetAsync(image.RegistryId, cancellationToken);
            if (registry is null)
                return Result.Failure<SwarmServiceUpdateCheckResult>(new BadRequestError(
                    "The Service's configured Registry does not exist."));
            var task = imageCheckBuilder.BuildScanTask(
                new ImageKey(image.RegistryId, repository, tag), service.PlatformId, registry);
            if (!task.IsSuccess(out var scanTask, out var error))
                return Result.Failure<SwarmServiceUpdateCheckResult>(error!);

            var startedAt = timeProvider.GetUtcNow();
            service.TryBeginUpdateCheck(actorId, startedAt);
            if (!await SaveAsync(service, cancellationToken))
                return Result.Failure<SwarmServiceUpdateCheckResult>(new ConflictError(
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

                    return Result.Failure<SwarmServiceUpdateCheckResult>(error!);
                }

                var evaluation = updateEvaluator.Evaluate(
                    service.AppliedImageDigest!, remoteDigest, timeProvider.GetUtcNow().UtcDateTime);
                service.CompleteUpdateCheck(evaluation.State);
                completed = await SaveAsync(service, cancellationToken);
                if (!completed)
                    return Result.Failure<SwarmServiceUpdateCheckResult>(new ConflictError(
                        "The Service changed while the update check was running."));

                service = await unitOfWork.SwarmServices.GetAsync(service.Id, cancellationToken) ?? service;
                await streamManager.SendSwarmServiceInfo(service);
                if (!evaluation.UpdateAvailable)
                    return Result.Success(new SwarmServiceUpdateCheckResult(
                        service, false, false, "The Service image is up to date."));

                if (!applyWhenAvailable || service.Spec.UpdateBehavior != UpdateBehavior.AutoDeploy)
                    return Result.Success(new SwarmServiceUpdateCheckResult(
                        service, true, false, "A newer image digest is available."));

                var automatedOperations = await entitlementService.EnsureEnabledAsync(
                    LicenseCapability.AutomatedOperations, cancellationToken);
                if (automatedOperations.IsFailure(out error))
                    return Result.Success(new SwarmServiceUpdateCheckResult(
                        service, true, false, $"Paused by license: {error!.Message}"));
                var operationalGuardrails = await entitlementService.EnsureEnabledAsync(
                    LicenseCapability.OperationalGuardrails, cancellationToken);
                if (operationalGuardrails.IsFailure(out error))
                    return Result.Success(new SwarmServiceUpdateCheckResult(
                        service, true, false, $"Paused by license: {error!.Message}"));

                var applied = await mutationService.ApplyAsync(
                    service.Id, Constants.SystemId, cancellationToken);
                if (!applied.IsSuccess(out service, out error))
                    return Result.Failure<SwarmServiceUpdateCheckResult>(error!);

                return Result.Success(new SwarmServiceUpdateCheckResult(
                    service, true, true, "A newer image digest was found and Apply started."));
            }
            finally
            {
                if (!completed)
                    await ReleaseUpdateCheckAsync(service!.Id, actorId, startedAt);
            }
        }
    }

    private async Task ReleaseUpdateCheckAsync(Guid serviceId, Guid actorId, DateTimeOffset startedAt)
    {
        var current = await unitOfWork.SwarmServices.GetAsync(serviceId, CancellationToken.None);
        if (current is null
            || current.ControlState != ResourceControlState.Processing
            || current.ControlTriggeredBy != actorId
            || current.ControlStartedAt != startedAt.ToUnixTimeSeconds())
            return;

        current.CompleteUpdateCheck();
        if (await SaveAsync(current, CancellationToken.None))
            await streamManager.SendSwarmServiceInfo(current);
    }

    private async Task<bool> SaveAsync(SwarmService service, CancellationToken cancellationToken)
    {
        if (await unitOfWork.SwarmServices.UpdateAsync(service, cancellationToken) == 0)
            return false;
        await unitOfWork.CommitAsync(cancellationToken);
        return true;
    }
}
