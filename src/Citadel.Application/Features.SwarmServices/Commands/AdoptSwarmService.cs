using Application.Features.SwarmServices.Queries;
using Application.Services;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.SwarmServices;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;

namespace Application.Features.SwarmServices.Commands;

[RequirePermission(ResourceType.SwarmService, PermissionLevel.Write)]
[RequirePermission(ResourceType.Platform, PermissionLevel.Read, SpecificPermission.Inspect,
    ResourceIdProperty = nameof(PlatformId))]
public sealed record AdoptSwarmService(
    Guid PlatformId,
    string DockerServiceId,
    string Name,
    string? Description,
    SwarmServiceSpec Spec,
    string PreviewFingerprint,
    IReadOnlyCollection<Guid>? TagIds = null)
    : ICommand<Result<SwarmService>>
{
    internal sealed class Validator : AbstractValidator<AdoptSwarmService>
    {
        public Validator()
        {
            RuleFor(command => command.PlatformId).NotEmpty();
            RuleFor(command => command.DockerServiceId).NotEmpty().MaximumLength(255);
            RuleFor(command => command.Name).NotEmpty().ValidNameIdentifier();
            RuleFor(command => command.Description).MaximumLength(600);
            RuleFor(command => command.Spec).NotNull();
            RuleFor(command => command.PreviewFingerprint).NotEmpty().Length(64);
        }
    }
}

internal sealed class AdoptSwarmServiceHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<ISwarmConnector> connectorFactory,
    IUserContextAccessor userContext,
    IAdoptionFingerprintService fingerprintService,
    ISwarmServiceStreamManager streamManager)
    : ICommandHandler<AdoptSwarmService, Result<SwarmService>>
{
    public async ValueTask<Result<SwarmService>> Handle(
        AdoptSwarmService command,
        CancellationToken cancellationToken)
    {
        var loaded = await SwarmServiceAdoptionDraftFactory.LoadAsync(
            command.PlatformId,
            command.DockerServiceId,
            unitOfWork,
            connectorFactory,
            cancellationToken);
        if (!loaded.IsSuccess(out var context))
            return Result.Failure<SwarmService>(loaded.Errors);

        var currentFingerprint = SwarmServiceAdoptionDraftFactory.ComputeFingerprint(context, fingerprintService);
        if (!fingerprintService.Matches(currentFingerprint, command.PreviewFingerprint))
            return Result.Failure<SwarmService>(new ConflictError("Docker Service configuration changed. Reload the adoption draft and review it again."));

        if (!ReferencesSameRepository(context.Service.Image, command.Spec.Image))
            return Result.Failure<SwarmService>(new BadRequestError("Select an external image from the Docker Service's current image repository."));

        var spec = command.Spec with
        {
            UpdateBehavior = UpdateBehavior.Disabled,
            Webhook = null
        };
        if (SwarmServiceAdoptionDraftFactory.GetRedactedSensitiveEnvironmentName(spec.Environment) is { } sensitiveName)
        {
            return Result.Failure<SwarmService>(new BadRequestError(
                $"Enter a new value or Citadel binding for sensitive environment variable '{sensitiveName}'."));
        }
        var validation = await SwarmServiceValidation.ValidateAsync(
            spec,
            command.PlatformId,
            unitOfWork,
            userContext,
            cancellationToken);
        if (validation.IsFailure(out var validationError))
            return Result.Failure<SwarmService>(validationError);

        if (await unitOfWork.SwarmServices.ExistsAsync(command.PlatformId, command.Name, cancellationToken))
            return Result.Failure<SwarmService>(new ConflictError("Name already exists on this platform."));
        if (await unitOfWork.SwarmServices.GetByDockerServiceIdAsync(
                command.PlatformId,
                command.DockerServiceId,
                cancellationToken) is not null)
            return Result.Failure<SwarmService>(new ConflictError("Swarm Service is already managed by Citadel."));

        var service = SwarmService.AdoptExisting(
            command.Name,
            command.Description,
            command.PlatformId,
            userContext.Current.ActorId,
            context.Service.Name,
            context.Service.Id,
            context.Service.VersionIndex,
            context.Service.RuntimeHash,
            spec);
        service.ApplyObservation(context.Projection);

        var added = await unitOfWork.SwarmServices.AddAsync(
            service,
            cancellationToken,
            command.TagIds,
            userContext.Current.ActorId);
        if (added == 0)
        {
            if (await unitOfWork.SwarmServices.GetByDockerServiceIdAsync(
                    command.PlatformId,
                    command.DockerServiceId,
                    cancellationToken) is not null)
                return Result.Failure<SwarmService>(new ConflictError("Swarm Service is already managed by Citadel."));
            if (await unitOfWork.SwarmServices.ExistsAsync(command.PlatformId, command.Name, cancellationToken))
                return Result.Failure<SwarmService>(new ConflictError("Name already exists on this platform."));
            return Result.Failure<SwarmService>(new BadRequestError("One or more tags do not exist."));
        }

        await unitOfWork.ActivityEventRepository.AddAsync(new ActivityEvent(
            platformId: service.PlatformId,
            resourceId: service.Id,
            actorId: userContext.Current.ActorId,
            resourceName: service.Name,
            eventType: ActivityEventType.SwarmServiceAdopted,
            status: ActivityStatus.Information,
            info: new SwarmServiceAdopted(service.ToActivitySnapshot(), context.Service.Id)), cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);
        await streamManager.SendSwarmServiceInfo(service, "create");
        return Result.Success(service);
    }

    private static bool ReferencesSameRepository(string sourceReference, SwarmServiceImageInfo selectedImage)
    {
        if (selectedImage is not SwarmExternalImage external)
            return false;
        return string.Equals(GetRepository(sourceReference), GetRepository(external.ImageTag), StringComparison.OrdinalIgnoreCase);
    }

    private static string? GetRepository(string imageReference)
    {
        var value = imageReference.Trim().TrimStart('/').ToLowerInvariant();
        if (value.Length == 0 || value.StartsWith("sha256:", StringComparison.Ordinal))
            return null;
        var digest = value.LastIndexOf('@');
        if (digest > 0) value = value[..digest];
        var slash = value.LastIndexOf('/');
        var tag = value.LastIndexOf(':');
        if (tag > slash) value = value[..tag];
        var firstSlash = value.IndexOf('/');
        if (firstSlash < 0) return $"docker.io/library/{value}";
        var first = value[..firstSlash];
        if (string.Equals(first, "index.docker.io", StringComparison.Ordinal))
            return "docker.io" + value[firstSlash..];
        if (!first.Contains('.') && !first.Contains(':') && !string.Equals(first, "localhost", StringComparison.Ordinal))
            return $"docker.io/{value}";
        return value;
    }
}
