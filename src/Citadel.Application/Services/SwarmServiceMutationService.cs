using Application.Services.Builds;
using Application.TaskJobs;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities.Platforms;
using Domain.Entities.SwarmServices;
using Domain.Entities.Activities;
using Hosting.Common.ErrorTypes;
using Hosting.Common;
using Grpc.Core;
using LightResults;
using Microsoft.Extensions.Hosting;
using Application.Services.SignalR;
using Hosting.Common.Abstraction;

namespace Application.Services;

internal interface ISwarmServiceMutationService
{
    Task<Result<SwarmService>> ApplyAsync(
        Guid serviceId,
        Guid actorId,
        CancellationToken cancellationToken,
        Action<string>? reportProgress = null);
    Task<Result<SwarmService>> ScaleAsync(Guid serviceId, int replicas, Guid actorId, CancellationToken cancellationToken);
    Task<Result<SwarmService>> ForceUpdateAsync(Guid serviceId, Guid actorId, CancellationToken cancellationToken);
    Task<Result> DeleteAsync(Guid serviceId, Guid actorId, CancellationToken cancellationToken);
}

internal sealed class SwarmServiceMutationService(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory,
    IBuildImageResolver buildImageResolver,
    IImageCheckBuilder imageCheckBuilder,
    IImageDigestScanner imageDigestScanner,
    IResourceBindingResolver resourceBindingResolver,
    ISwarmReconciliationCoordinator reconciliationCoordinator,
    IHostApplicationLifetime applicationLifetime,
    ISwarmServiceStreamManager streamManager,
    IUserContextAccessor userContext) : ISwarmServiceMutationService
{
    private static readonly TimeSpan MutationTimeout = TimeSpan.FromMinutes(2);

    public Task<Result<SwarmService>> ApplyAsync(
        Guid serviceId,
        Guid actorId,
        CancellationToken cancellationToken,
        Action<string>? reportProgress = null) =>
        MutateAsync(serviceId, SwarmServiceOperationKind.Apply, actorId, null, cancellationToken, reportProgress);

    public Task<Result<SwarmService>> ScaleAsync(
        Guid serviceId,
        int replicas,
        Guid actorId,
        CancellationToken cancellationToken) =>
        MutateAsync(serviceId, SwarmServiceOperationKind.Scale, actorId, replicas, cancellationToken);

    public Task<Result<SwarmService>> ForceUpdateAsync(
        Guid serviceId,
        Guid actorId,
        CancellationToken cancellationToken) =>
        MutateAsync(serviceId, SwarmServiceOperationKind.ForceUpdate, actorId, null, cancellationToken);

    public async Task<Result> DeleteAsync(
        Guid serviceId,
        Guid actorId,
        CancellationToken cancellationToken)
    {
        var service = await unitOfWork.SwarmServices.GetAsync(serviceId, cancellationToken);
        if (service is null)
            return Result.Failure(new NotFoundError("The managed Swarm Service does not exist."));

        if (!await CanMutateAsync(
                service.Id,
                PermissionLevel.Execute,
                SpecificPermission.None,
                actorId,
                cancellationToken))
            return Result.Failure(new NotFoundError("The managed Swarm Service does not exist."));

        var platformResult = await LoadMutationPlatformAsync(service.PlatformId, actorId, cancellationToken);
        if (!platformResult.IsSuccess(out var platform, out var error))
            return Result.Failure(error!);
        if (service.ControlState == ResourceControlState.Processing)
            return Result.Failure(new ConflictError("The Service already has an operation in progress."));

        if (service.DockerServiceId is null)
        {
            await unitOfWork.ActivityEventRepository.AddAsync(new ActivityEvent(
                service.PlatformId, service.Id, actorId, service.Name,
                ActivityEventType.SwarmServiceDeleted, ActivityStatus.Information,
                new SwarmServiceDeleted(service.ToActivitySnapshot())), cancellationToken);
            if (await unitOfWork.SwarmServices.RemoveAsync(service.Id, service.RowVersion, cancellationToken) == 0)
                return Result.Failure(new ConflictError("The Service was changed by another operation."));
            await unitOfWork.CommitAsync(cancellationToken);
            await streamManager.SendSwarmServiceInfo(service, "delete");
            return Result.Success();
        }

        var operationId = Guid.CreateVersion7();
        if (!service.TryPrepareOperation(
                SwarmServiceOperationKind.Delete,
                operationId,
                actorId,
                baseDockerVersion: service.DockerVersionIndex,
                clusterId: platform.ClusterId))
            return Result.Failure(new ConflictError("The Service already has an operation in progress."));

        var prepared = await PersistAndReloadAsync(service, cancellationToken);
        if (!prepared.IsSuccess(out service, out error))
            return Result.Failure(error!);

        // Once Prepared is durable the operation must no longer depend on the HTTP request.
        // Reconciliation can recover an undispatched Prepared operation, but it must not be
        // left there merely because the caller disconnected between the two durable states.
        service.MarkOperationAttempted();
        var attempted = await PersistAndReloadAsync(service, CancellationToken.None);
        if (!attempted.IsSuccess(out service, out error))
            return Result.Failure(error!);

        using var dispatch = CreateDispatchToken();
        Result result;
        try
        {
            result = await connectorFactory.GetConnector(platform.ConnectorType).DeleteServiceAsync(
                new DeleteManagedSwarmServiceCommand(platform.Address, operationId, service.DockerServiceId!),
                dispatch.Token);
        }
        catch (OperationCanceledException)
        {
            await MarkUnknownAsync(service, "The Docker delete outcome is unknown.");
            return Result.Failure(new ConflictError("The Docker delete outcome is unknown. Reconciliation will verify it."));
        }
        catch (Exception ex)
        {
            await MarkUnknownAsync(service, Sanitize(ex.Message));
            return Result.Failure(new ConflictError("The Docker delete outcome is unknown. Reconciliation will verify it."));
        }

        if (result.IsFailure(out error) && error is not NotFoundError)
        {
            if (!IsDefiniteDockerRejection(error))
            {
                await MarkUnknownAsync(service, Sanitize(error.Message));
                return Result.Failure(new ConflictError("The Docker delete outcome is unknown. Reconciliation will verify it."));
            }

            service.CompleteOperation(SwarmServiceOperationState.Rejected, resultCode: "DockerRejected", resultMessage: Sanitize(error.Message));
            await AddFailureActivityAsync(service, actorId, error.Message, CancellationToken.None);
            if (!await PersistAsync(service, CancellationToken.None))
                return Result.Failure(new ConflictError("The Service changed while the operation result was being saved."));
            return Result.Failure(error);
        }

        service.MarkOperationAccepted(service.DockerServiceId, service.DockerVersionIndex);
        var accepted = await PersistAndReloadAsync(service, CancellationToken.None);
        if (!accepted.IsSuccess(out service, out error))
            return Result.Failure(error!);

        await reconciliationCoordinator.RefreshAsync(service.PlatformId, CancellationToken.None);
        var reconciled = await unitOfWork.SwarmServices.GetAsync(service.Id, CancellationToken.None);
        if (reconciled is null)
            return Result.Success();

        // Reconciliation owns projection-derived completion. A continued runtime remains
        // pending/unknown until a later complete observation can prove the outcome.
        return Result.Success();
    }

    private async Task<Result<SwarmService>> MutateAsync(
        Guid serviceId,
        SwarmServiceOperationKind kind,
        Guid actorId,
        int? replicas,
        CancellationToken cancellationToken,
        Action<string>? reportProgress = null)
    {
        var service = await unitOfWork.SwarmServices.GetAsync(serviceId, cancellationToken);
        if (service is null)
            return Result.Failure<SwarmService>(new NotFoundError("The managed Swarm Service does not exist."));

        var requiredLevel = kind == SwarmServiceOperationKind.Scale
            ? PermissionLevel.Write
            : PermissionLevel.Read;
        if (!await CanMutateAsync(service.Id, requiredLevel, SpecificPermission.Apply, actorId, cancellationToken))
            return Result.Failure<SwarmService>(new NotFoundError("The managed Swarm Service does not exist."));
        if (service.ControlState == ResourceControlState.Processing)
            return Result.Failure<SwarmService>(new ConflictError("The Service already has an operation in progress."));
        if (kind == SwarmServiceOperationKind.Scale)
        {
            if (service.Spec.SchedulingMode != SwarmServiceSchedulingMode.Replicated)
                return Result.Failure<SwarmService>(new BadRequestError("Global Services cannot be scaled by replica count."));
            if (replicas is null or < 0)
                return Result.Failure<SwarmService>(new BadRequestError("Replica count cannot be negative."));
            service.UpdateSpec(service.Spec with { Replicas = replicas });
        }

        var platformResult = await LoadMutationPlatformAsync(service.PlatformId, actorId, cancellationToken);
        if (!platformResult.IsSuccess(out var platform, out var error))
            return Result.Failure<SwarmService>(error!);

        var bindingsResult = await resourceBindingResolver.ResolveAsync(
            ResourceBindingScope.SwarmService, service.Id, cancellationToken);
        if (!bindingsResult.IsSuccess(out var bindings, out error))
            return Result.Failure<SwarmService>(error!);
        var referencedBindings = bindings.SelectEntries(
            EnvironmentVariableResolver.GetReferencedNames(service.Spec.Environment));
        var environmentResult = EnvironmentVariableResolver.Build(
            service.Spec.Environment,
            referencedBindings,
            "Service");
        if (!environmentResult.IsSuccess(out var environment, out error))
            return Result.Failure<SwarmService>(new BadRequestError(error!.Message));
        reportProgress?.Invoke(ResourceBindingApplyMessageBuilder.BuildServiceEnvironmentMessage(referencedBindings));
        var effectiveSpec = service.Spec with { Environment = environment };

        var projection = service.DockerServiceId is null
            ? null
            : await unitOfWork.Swarm.GetServiceAsync(service.PlatformId, service.DockerServiceId, cancellationToken);
        var create = service.DockerServiceId is null || projection is null;

        var resolvedResult = await ResolveImageAsync(
            service.Spec.Image,
            service.PlatformId,
            resolveCurrentDigest: kind == SwarmServiceOperationKind.Apply,
            cancellationToken);
        if (!resolvedResult.IsSuccess(out var image, out error))
            return Result.Failure<SwarmService>(error!);

        // Scale and force-update are runtime operations. They must keep using the image that
        // is already applied even when the configured tag has moved since the last Apply.
        if (!create
            && kind is SwarmServiceOperationKind.Scale or SwarmServiceOperationKind.ForceUpdate
            && !string.IsNullOrWhiteSpace(projection!.Image))
        {
            image = image with
            {
                Reference = projection.Image,
                Digest = service.AppliedImageDigest
            };
        }

        if (create)
        {
            var collision = (await unitOfWork.Swarm.GetServicesAsync(service.PlatformId, cancellationToken))
                .Any(item => string.Equals(item.Name, service.DockerName, StringComparison.Ordinal)
                    && item.SwarmServiceId != service.Id);
            if (collision)
                return Result.Failure<SwarmService>(new ConflictError("A Docker Service already uses the generated runtime name."));
        }

        var operationId = Guid.CreateVersion7();
        var targetRuntimeHash = SwarmServiceRuntimeHasher.Hash(effectiveSpec, image.Reference);
        long? expectedForceUpdate = kind == SwarmServiceOperationKind.ForceUpdate
            ? (projection?.ForceUpdate ?? 0) + 1
            : null;
        if (!service.TryPrepareOperation(
                kind,
                operationId,
                actorId,
                targetRuntimeHash,
                baseDockerVersion: projection?.VersionIndex ?? service.DockerVersionIndex,
                expectedForceUpdate: expectedForceUpdate,
                clusterId: platform.ClusterId))
            return Result.Failure<SwarmService>(new ConflictError("The Service already has an operation in progress."));

        var prepared = await PersistAndReloadAsync(service, cancellationToken);
        if (!prepared.IsSuccess(out service, out error))
            return Result.Failure<SwarmService>(error!);

        service.MarkOperationAttempted();
        var attempted = await PersistAndReloadAsync(service, CancellationToken.None);
        if (!attempted.IsSuccess(out service, out error))
            return Result.Failure<SwarmService>(error!);

        var labels = CreateLabels(effectiveSpec.Labels, service.Id, operationId);
        using var dispatch = CreateDispatchToken();
        Result<ManagedSwarmServiceMutationResult> mutation;
        try
        {
            var connector = connectorFactory.GetConnector(platform.ConnectorType);
            mutation = create
                ? await connector.CreateServiceAsync(new CreateManagedSwarmServiceCommand(
                    platform.Address, operationId, service.DockerName, effectiveSpec,
                    image.Reference, labels, image.RegistryAuth), dispatch.Token)
                : await connector.UpdateServiceAsync(new UpdateManagedSwarmServiceCommand(
                    platform.Address, operationId, service.DockerServiceId!,
                    projection!.VersionIndex, effectiveSpec, image.Reference, labels,
                    image.RegistryAuth, kind == SwarmServiceOperationKind.ForceUpdate ? 1 : 0), dispatch.Token);

            if (!create
                && kind is SwarmServiceOperationKind.Scale or SwarmServiceOperationKind.ForceUpdate
                && mutation.IsFailure(out var mutationError)
                && mutationError is ConflictError)
            {
                var inspected = await connector.InspectServiceAsync(
                    new InspectSwarmServiceCommand(platform.Address, service.DockerServiceId!),
                    dispatch.Token);
                if (inspected.IsSuccess(out var current))
                {
                    long? retryExpectedForceUpdate = kind == SwarmServiceOperationKind.ForceUpdate
                        ? current.ForceUpdate + 1
                        : null;
                    if (service.TryPrepareVersionConflictRetry(current.VersionIndex, retryExpectedForceUpdate))
                    {
                        var retryPrepared = await PersistAndReloadAsync(service, CancellationToken.None);
                        if (!retryPrepared.IsSuccess(out service, out error))
                            return Result.Failure<SwarmService>(error!);

                        mutation = await connector.UpdateServiceAsync(new UpdateManagedSwarmServiceCommand(
                            platform.Address,
                            operationId,
                            service.DockerServiceId!,
                            current.VersionIndex,
                            effectiveSpec,
                            image.Reference,
                            labels,
                            image.RegistryAuth,
                            kind == SwarmServiceOperationKind.ForceUpdate ? 1 : 0), dispatch.Token);
                    }
                }
            }
        }
        catch (OperationCanceledException)
        {
            await MarkUnknownAsync(service!, "The Docker mutation outcome is unknown.");
            return Result.Failure<SwarmService>(new ConflictError("The Docker mutation outcome is unknown. Reconciliation will verify it."));
        }
        catch (Exception ex)
        {
            await MarkUnknownAsync(service!, Sanitize(ex.Message));
            return Result.Failure<SwarmService>(new ConflictError("The Docker mutation outcome is unknown. Reconciliation will verify it."));
        }

        if (!mutation.IsSuccess(out var mutationResult, out error))
        {
            if (!IsDefiniteDockerRejection(error!))
            {
                await MarkUnknownAsync(service, Sanitize(error.Message));
                return Result.Failure<SwarmService>(new ConflictError("The Docker mutation outcome is unknown. Reconciliation will verify it."));
            }

            service.CompleteOperation(SwarmServiceOperationState.Rejected, resultCode: "DockerRejected", resultMessage: Sanitize(error!.Message));
            await AddFailureActivityAsync(service, actorId, error.Message, CancellationToken.None);
            if (!await PersistAsync(service, CancellationToken.None))
                return Result.Failure<SwarmService>(new ConflictError("The Service changed while the operation result was being saved."));
            return Result.Failure<SwarmService>(error);
        }

        service.MarkOperationAccepted(
            mutationResult.ServiceId ?? service.DockerServiceId,
            projection?.VersionIndex,
            mutationResult.Warnings);
        var accepted = await PersistAndReloadAsync(service, CancellationToken.None);
        if (!accepted.IsSuccess(out service, out error))
            return Result.Failure<SwarmService>(error!);

        await reconciliationCoordinator.RefreshAsync(service.PlatformId, CancellationToken.None);
        // The reconciliation transaction is the sole writer of projection-derived state.
        // Reload instead of completing with this request's now-stale aggregate.
        return await unitOfWork.SwarmServices.GetAsync(service.Id, CancellationToken.None) is { } reconciled
            ? Result.Success(reconciled)
            : Result.Failure<SwarmService>(new NotFoundError("The managed Swarm Service no longer exists."));
    }

    private async Task<Result<Platform>> LoadMutationPlatformAsync(
        Guid platformId,
        Guid actorId,
        CancellationToken cancellationToken)
    {
        var platform = await unitOfWork.Platforms.GetByIdAsync(platformId, cancellationToken);
        if (platform is null
            || (actorId != Constants.SystemId
                && !userContext.Current.IsAdmin
                && !await unitOfWork.Platforms.CanAccessAsync(
                    userContext.Current.UserId, platformId, cancellationToken)))
            return Result.Failure<Platform>(new NotFoundError("The Docker Swarm platform does not exist."));
        if (platform.PlatformDescriptor is not DockerSwarmPlatformDescriptor descriptor)
            return Result.Failure<Platform>(new BadRequestError("Managed Services require a Docker Swarm platform."));
        if (platform.Status != PlatformStatus.Online || !descriptor.ControlAvailable)
            return Result.Failure<Platform>(new ConflictError("The Docker Swarm manager is not available."));
        return Result.Success(platform);
    }

    private Task<bool> CanMutateAsync(
        Guid serviceId,
        PermissionLevel permissionLevel,
        SpecificPermission specificPermission,
        Guid actorId,
        CancellationToken cancellationToken) =>
        actorId == Constants.SystemId || userContext.Current.IsAdmin
            ? Task.FromResult(true)
            : unitOfWork.SwarmServices.CanAccessAsync(
                userContext.Current.UserId,
                serviceId,
                permissionLevel,
                specificPermission,
                cancellationToken);

    private async Task<Result<ResolvedServiceImage>> ResolveImageAsync(
        SwarmServiceImageInfo image,
        Guid platformId,
        bool resolveCurrentDigest,
        CancellationToken cancellationToken)
    {
        Guid registryId;
        string reference;
        string? digest;
        if (image is SwarmExternalImage external)
        {
            registryId = external.RegistryId;
            reference = external.ImageTag;
            digest = external.ResolvedDigest ?? ExtractDigest(external.ImageTag);
        }
        else if (image is SwarmBuildImage build)
        {
            var resolved = await buildImageResolver.ResolveAsync(
                build.BuildProjectId,
                build.ResolvedImageReference,
                build.ResolvedDigest,
                build.ResolvedBuildRunId,
                cancellationToken);
            if (!resolved.IsSuccess(out var buildImage, out var error))
                return Result.Failure<ResolvedServiceImage>(error!);
            registryId = buildImage.RegistryId;
            reference = buildImage.ImageReference;
            digest = buildImage.Digest;
        }
        else
        {
            return Result.Failure<ResolvedServiceImage>(new BadRequestError("The Service image source is not supported."));
        }

        var registry = await unitOfWork.Registries.GetAsync(registryId, cancellationToken);
        if (registry is null)
            return Result.Failure<ResolvedServiceImage>(new NotFoundError("The selected Registry does not exist."));

        if (resolveCurrentDigest && image is SwarmExternalImage externalImage)
        {
            if (!Helpers.TrySplitImageTag(externalImage.ImageTag, out var repository, out var tag))
            {
                if (digest is null)
                    return Result.Failure<ResolvedServiceImage>(new BadRequestError(
                        "The Service image must use a tagged or digest-pinned reference."));
            }
            else
            {
                var task = imageCheckBuilder.BuildScanTask(
                    new ImageKey(registryId, repository, tag),
                    platformId,
                    registry);
                if (!task.IsSuccess(out var scanTask, out var scanTaskError))
                    return Result.Failure<ResolvedServiceImage>(scanTaskError!);

                var scanned = await imageDigestScanner.ScanAsync(scanTask, cancellationToken);
                if (!scanned.IsSuccess(out digest, out var scanError))
                    return Result.Failure<ResolvedServiceImage>(scanError!);
            }

            reference = BuildImageReference.PinToDigest(externalImage.ImageTag, digest);
        }
        try
        {
            return Result.Success(new ResolvedServiceImage(
                reference,
                digest,
                registry.Configuration.GetRegistryAuth(registry.RegistryHost)));
        }
        catch (NotImplementedException)
        {
            return Result.Failure<ResolvedServiceImage>(new BadRequestError("The selected Registry type cannot authenticate Swarm Service image pulls yet."));
        }
    }

    private async Task<Result<SwarmService>> PersistAndReloadAsync(
        SwarmService service,
        CancellationToken cancellationToken)
    {
        if (await unitOfWork.SwarmServices.UpdateAsync(service, cancellationToken) == 0)
            return Result.Failure<SwarmService>(new ConflictError("The Service was changed by another operation."));
        await unitOfWork.CommitAsync(cancellationToken);
        var updated = await unitOfWork.SwarmServices.GetAsync(service.Id, cancellationToken);
        if (updated is null)
            return Result.Failure<SwarmService>(new NotFoundError("The managed Swarm Service no longer exists."));
        await streamManager.SendSwarmServiceInfo(updated);
        return Result.Success(updated);
    }

    private async Task<bool> PersistAsync(SwarmService service, CancellationToken cancellationToken)
    {
        if (await unitOfWork.SwarmServices.UpdateAsync(service, cancellationToken) == 0)
            return false;
        await unitOfWork.CommitAsync(cancellationToken);
        await streamManager.SendSwarmServiceInfo(service);
        return true;
    }

    private async Task MarkUnknownAsync(SwarmService service, string message)
    {
        var actorId = service.ControlTriggeredBy;
        service.CompleteOperation(
            SwarmServiceOperationState.OutcomeUnknown,
            resultCode: "OutcomeUnknown",
            resultMessage: message);
        if (actorId is Guid triggeringActorId)
            await AddFailureActivityAsync(service, triggeringActorId, message, CancellationToken.None);
        await PersistAsync(service, CancellationToken.None);
        await reconciliationCoordinator.RefreshAsync(service.PlatformId, CancellationToken.None);
    }

    private static bool IsDefiniteDockerRejection(IError error) => error switch
    {
        BadRequestError or ConflictError or ForbiddenError or UnauthorizedError or NotFoundError => true,
        ClientRpcException rpcError => rpcError.StatusCode is
            StatusCode.InvalidArgument
            or StatusCode.FailedPrecondition
            or StatusCode.AlreadyExists
            or StatusCode.NotFound
            or StatusCode.PermissionDenied
            or StatusCode.Unauthenticated
            or StatusCode.OutOfRange
            or StatusCode.Unimplemented,
        _ => false
    };

    private CancellationTokenSource CreateDispatchToken()
    {
        var source = CancellationTokenSource.CreateLinkedTokenSource(applicationLifetime.ApplicationStopping);
        source.CancelAfter(MutationTimeout);
        return source;
    }

    private static Dictionary<string, string> CreateLabels(
        IReadOnlyDictionary<string, string> configuredLabels,
        Guid serviceId,
        Guid operationId)
    {
        var labels = configuredLabels.ToDictionary(StringComparer.Ordinal);
        labels["com.citadel.managed"] = "true";
        labels["com.citadel.service-id"] = serviceId.ToString();
        labels["com.citadel.operation-id"] = operationId.ToString();
        return labels;
    }

    private static string Sanitize(string message) => message.Length <= 1000 ? message : message[..1000];

    private static string? ExtractDigest(string imageReference)
    {
        var separator = imageReference.LastIndexOf('@');
        return separator >= 0 && separator < imageReference.Length - 1
            ? imageReference[(separator + 1)..]
            : null;
    }

    private async Task AddFailureActivityAsync(
        SwarmService service,
        Guid actorId,
        string reason,
        CancellationToken cancellationToken)
    {
        var operation = service.CurrentOperation!;
        await unitOfWork.ActivityEventRepository.AddAsync(new ActivityEvent(
            service.PlatformId, service.Id, actorId, service.Name,
            ActivityEventType.SwarmServiceOperationFailed, ActivityStatus.Failure,
            new SwarmServiceOperationFailed(operation.Id, operation.Kind, Sanitize(reason))), cancellationToken);
    }

    private sealed record ResolvedServiceImage(string Reference, string? Digest, string? RegistryAuth);
}
