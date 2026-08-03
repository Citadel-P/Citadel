using Application.Features.Alerters.Notifications;
using Application.Features.Containers.Queries;
using Application.Features.Deployments.Commands;
using Application.Features.Deployments.Notifications;
using Application.Services;
using Application.Services.Builds;
using Application.Services.Licensing;
using Application.Services.SignalR;
using Application.TaskJobs;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Deployments;
using Domain.Contracts.Resources.Images;
using Domain.Entities;
using Domain.Entities.Activities;
using Domain.Entities.Deployments;
using Domain.Entities.ResourceBindings;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;

namespace Application.Features.Containers.Commands;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Write)]
public sealed record AdoptContainer(
    Guid ContainerId,
    string Name,
    string? Description,
    DeploymentSpec Spec,
    string PreviewFingerprint,
    IReadOnlyCollection<Guid>? TagIds = null,
    bool ImportSensitiveEnvironmentAsSecrets = false)
    : ICommand<Result<Deployment>>
{
    internal sealed class Validator : AbstractValidator<AdoptContainer>
    {
        public Validator()
        {
            RuleFor(x => x.Name).NotEmpty().ValidNameIdentifier();
            RuleFor(x => x.Description).MaximumLength(600);
            RuleFor(x => x.PreviewFingerprint).NotEmpty().Length(64);
        }
    }
}

internal sealed class AdoptContainerHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<IContainerConnector> connectorFactory,
    IConnectorFactory<IImageConnector> imageConnectorFactory,
    IContainerAuthorizationService containerAuthorizationService,
    IDeploymentStreamManager deploymentStreamManager,
    IPlatformStreamManager platformStreamManager,
    IContainerStreamManager containerStreamManager,
    IActivityStreamManager activityStreamManager,
    IAlertEventStreamManager alertEventStreamManager,
    INotificationQueue notificationQueue,
    IUserContextAccessor userContext,
    ILicenseEntitlementService entitlementService,
    IAdoptionFingerprintService fingerprintService,
    ISecretValueProtector secretValueProtector)
    : ICommandHandler<AdoptContainer, Result<Deployment>>
{
    public async ValueTask<Result<Deployment>> Handle(
        AdoptContainer command,
        CancellationToken cancellationToken)
    {
        var contextResult = await ContainerAdoptionDraftFactory.LoadContextAsync(
            command.ContainerId,
            unitOfWork,
            connectorFactory,
            imageConnectorFactory,
            containerAuthorizationService,
            cancellationToken);
        if (!contextResult.IsSuccess(out var context))
            return Result.Failure<Deployment>(contextResult.Errors);

        var currentFingerprint = ContainerAdoptionDraftFactory.ComputeFingerprint(context, fingerprintService);
        if (!fingerprintService.Matches(currentFingerprint, command.PreviewFingerprint))
        {
            return Result.Failure<Deployment>(
                new ConflictError("Container configuration changed. Reload the adoption draft and review it again."));
        }

        var requestedSpec = BuildImageProvenance.Clear(command.Spec);
        var validationContextResult = await ContainerAdoptionImageValidation.ResolveAsync(
            context,
            requestedSpec,
            unitOfWork,
            imageConnectorFactory,
            cancellationToken);
        if (!validationContextResult.IsSuccess(out var validationContext))
            return Result.Failure<Deployment>(validationContextResult.Errors);

        var currentDraft = ContainerAdoptionDraftFactory.Create(validationContext, command.Name, fingerprintService);
        var blocker = currentDraft.Issues.FirstOrDefault(x => x.Severity == AdoptionIssueSeverity.Blocker);
        if (blocker is not null)
            return Result.Failure<Deployment>(new BadRequestError(blocker.Message));

        var sensitiveNames = ContainerAdoptionDraftFactory.GetSensitiveEnvironmentNames(context.Inspection);
        IReadOnlyDictionary<string, string> importedSensitiveValues =
            new Dictionary<string, string>(StringComparer.Ordinal);
        if (command.ImportSensitiveEnvironmentAsSecrets)
        {
            var invalidName = sensitiveNames.FirstOrDefault(
                name => !ContainerAdoptionDraftFactory.IsValidEnvironmentBindingName(name));
            if (invalidName is not null)
            {
                return Result.Failure<Deployment>(
                    new BadRequestError(
                        $"Sensitive environment variable '{invalidName}' is not a valid Citadel binding name."));
            }

            importedSensitiveValues =
                ContainerAdoptionDraftFactory.GetImportableSensitiveEnvironmentValues(context.Inspection);
            var missingValue = sensitiveNames.FirstOrDefault(name => !importedSensitiveValues.ContainsKey(name));
            if (missingValue is not null)
            {
                return Result.Failure<Deployment>(
                    new BadRequestError(
                        $"Sensitive environment variable '{missingValue}' could not be imported from the container."));
            }

            requestedSpec = requestedSpec with
            {
                EnvironmentVariables = ContainerAdoptionDraftFactory.UseSensitiveEnvironmentBindingReferences(
                    requestedSpec.EnvironmentVariables,
                    sensitiveNames)
            };
        }
        else
        {
            var availableBindingNames = sensitiveNames.Count == 0
                ? new HashSet<string>(StringComparer.Ordinal)
                : (await unitOfWork.ResourceBindings.GetEntriesAsync(
                        ResourceBindingScope.Global,
                        null,
                        cancellationToken))
                    .Select(entry => entry.Name)
                    .ToHashSet(StringComparer.Ordinal);

            foreach (var sensitiveName in sensitiveNames)
            {
                if (!ContainerAdoptionDraftFactory.HasResolvedSensitiveEnvironment(
                        requestedSpec.EnvironmentVariables,
                        sensitiveName,
                        availableBindingNames))
                {
                    return Result.Failure<Deployment>(
                        new BadRequestError(
                            $"Enter a value or binding for sensitive environment variable '{sensitiveName}'."));
                }
            }
        }

        if (requestedSpec.EnvironmentVariables?.Any(
                x => x.EndsWith($"={ContainerInspectionRedactor.RedactedValue}", StringComparison.Ordinal)) == true)
        {
            return Result.Failure<Deployment>(
                new BadRequestError("Redacted environment placeholders cannot be saved."));
        }

        if (await unitOfWork.Deployments.ExistsAsync(command.Name, context.Platform.Id, cancellationToken))
            return Result.Failure<Deployment>(new ConflictError("Name already exists."));

        var spec = requestedSpec;
        var licenseResult = await ValidateLicenseAsync(spec, cancellationToken);
        if (licenseResult.IsFailure(out var licenseError))
            return Result.Failure<Deployment>(licenseError);

        var imageValidation = await DeploymentImageValidation.ValidateAsync(
            spec,
            PlatformType.Docker,
            unitOfWork,
            cancellationToken);
        if (imageValidation.IsFailure(out var imageError))
            return Result.Failure<Deployment>(imageError);

        var actorId = userContext.Current.ActorId;
        var deployment = new Deployment(
            name: command.Name,
            description: command.Description,
            createdByActorId: actorId,
            platformId: context.Platform.Id,
            spec: spec);
        deployment.PartialUpdate(status: Deployment.ToDeploymentStatus(context.Container.State));

        var added = await unitOfWork.Deployments.AddAsync(
            deployment,
            cancellationToken,
            command.TagIds,
            actorId);
        if (added == 0)
            return Result.Failure<Deployment>(new BadRequestError("One or more tags do not exist."));

        foreach (var importedSecret in importedSensitiveValues.OrderBy(entry => entry.Key, StringComparer.Ordinal))
        {
            var secret = new SecretDefinition(
                ContainerAdoptionDraftFactory.BuildImportedSecretName(
                    deployment.Id,
                    deployment.Name,
                    importedSecret.Key),
                SecretProviderType.InternalEncrypted);
            var value = new InternalSecretValue(
                secret.Id,
                secretValueProtector.Protect(importedSecret.Value));
            var binding = new ResourceBinding(
                Name: importedSecret.Key,
                Kind: ResourceBindingKind.Secret,
                Scope: ResourceBindingScope.Deployment,
                ResourceId: deployment.Id,
                Value: null,
                SecretId: secret.Id,
                SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable);

            await unitOfWork.SecretDefinitions.AddAsync(secret, value, cancellationToken);
            await unitOfWork.ResourceBindings.AddAsync(binding, cancellationToken);
        }

        var assigned = await unitOfWork.Containers.TryAssignToDeploymentAsync(
            context.Container.Id,
            context.Container.PlatformId,
            context.Container.DockerContainerId,
            deployment.Id,
            cancellationToken);
        if (assigned != 1)
        {
            return Result.Failure<Deployment>(
                new ConflictError("Container is no longer eligible for adoption. Refresh the container list."));
        }

        context.Container.PartialUpdate(deploymentId: deployment.Id);
        deployment.PartialUpdate(container: context.Container);

        var activity = new ActivityEvent(
            actorId: actorId,
            resourceId: deployment.Id,
            platformId: deployment.PlatformId,
            resourceName: deployment.Name,
            eventType: ActivityEventType.DeploymentAdopted,
            status: ActivityStatus.Information,
            info: new DeploymentAdopted(
                deployment.ToSnapshot(),
                context.Container.DockerContainerId,
                context.Container.Name));
        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);

        var resolvedAlerts = (await unitOfWork.AlertEvents.GetUnresolvedUnmanagedContainerAlertsAsync(
            [context.Container.DockerContainerId],
            cancellationToken)).ToArray();
        var actor = resolvedAlerts.Length == 0
            ? null
            : await unitOfWork.Actors.GetById(actorId, cancellationToken);
        var utcNow = DateTime.UtcNow;
        foreach (var alert in resolvedAlerts)
        {
            var resolveResult = alert.Resolve(actorId, utcNow, "Container adopted as a deployment.");
            if (!resolveResult.IsSuccess())
                return Result.Failure<Deployment>(resolveResult.Errors);

            alert.AssignActor(actor);
        }

        if (resolvedAlerts.Length > 0)
            await unitOfWork.AlertEvents.BulkUpdateAsync(resolvedAlerts, cancellationToken);

        var platform = await unitOfWork.Platforms.GetPlatformWithLatestStatAsync(
            context.Platform.Id,
            cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);

        await notificationQueue.EnqueueAsync(
            new DeploymentNotificationWorkItem(deploymentStreamManager, deployment, "create"),
            cancellationToken);
        await notificationQueue.EnqueueAsync(
            new SendContainersInfoNotificationWorkItem(
                containerStreamManager,
                [context.Container],
                context.Platform.Id),
            cancellationToken);
        await notificationQueue.EnqueueAsync(
            new ActivityNotificationWorkItem(
                activityStreamManager,
                await activity.AssignActor(unitOfWork, cancellationToken)),
            cancellationToken);

        if (platform is not null)
            await platformStreamManager.PushPlatformUpdate(platform);

        if (resolvedAlerts.Length > 0)
        {
            var payload = await AlertEventNotificationPayloadBuilder.BuildAsync(
                unitOfWork,
                resolvedAlerts,
                cancellationToken);
            await notificationQueue.EnqueueAsync(
                new UpdatedAlertEventsNotificationWorkItem(
                    payload.AlertEventsByUser,
                    payload.UnresolvedCountsByUser,
                    alertEventStreamManager),
                cancellationToken);
        }

        return deployment;
    }

    private async Task<Result> ValidateLicenseAsync(
        DeploymentSpec spec,
        CancellationToken cancellationToken)
    {
        if (DeploymentLicenseConfigurationPolicy.ExpandsOperationalGuardrails(null, spec))
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.OperationalGuardrails,
                cancellationToken);
            if (entitlement.IsFailure(out var error))
                return Result.Failure(error);
        }

        if (DeploymentLicenseConfigurationPolicy.ExpandsAutomatedOperations(null, spec))
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.AutomatedOperations,
                cancellationToken);
            if (entitlement.IsFailure(out var error))
                return Result.Failure(error);
        }

        return Result.Success();
    }
}

internal static class ContainerAdoptionImageValidation
{
    internal static async Task<Result<ContainerAdoptionContext>> ResolveAsync(
        ContainerAdoptionContext context,
        DeploymentSpec spec,
        IUnitOfWork unitOfWork,
        IConnectorFactory<IImageConnector> imageConnectorFactory,
        CancellationToken cancellationToken)
    {
        if (spec.Image is LocalImage localImage)
        {
            return await ResolveLocalAsync(
                context,
                localImage,
                unitOfWork,
                imageConnectorFactory,
                cancellationToken);
        }

        if (spec.Image is ExternalImage externalImage)
        {
            return await ResolveExternalAsync(
                context,
                externalImage,
                unitOfWork,
                cancellationToken);
        }

        if (context.Image is not null)
            return context;

        return Result.Failure<ContainerAdoptionContext>(
            new BadRequestError(
                "Select a local replacement or an external image from the original repository before adopting this container."));
    }

    private static async Task<Result<ContainerAdoptionContext>> ResolveLocalAsync(
        ContainerAdoptionContext context,
        LocalImage localImage,
        IUnitOfWork unitOfWork,
        IConnectorFactory<IImageConnector> imageConnectorFactory,
        CancellationToken cancellationToken)
    {
        if (!Guid.TryParse(localImage.ImageId, out var imageId) || imageId == Guid.Empty)
        {
            return Result.Failure<ContainerAdoptionContext>(
                new BadRequestError("Select a valid local image before adopting this container."));
        }

        var selectedImage = await unitOfWork.Images.GetByIdAsync(
            imageId,
            context.Platform.Id,
            cancellationToken);
        if (selectedImage is null)
        {
            return Result.Failure<ContainerAdoptionContext>(
                new NotFoundError("The selected local image is not available on this platform."));
        }

        if (context.Image?.Id != selectedImage.Id
            && !ReferencesSameRepository(context, selectedImage.Tags))
        {
            return Result.Failure<ContainerAdoptionContext>(
                new BadRequestError("Select a replacement from the container's original image repository."));
        }

        if (context.Image?.Id == selectedImage.Id
            && (!ContainerAdoptionDraftFactory.RequiresImageDefaultComparison(context.Inspection.Config)
                || context.ImageInspection is not null))
        {
            return context;
        }

        return await InspectAsync(
            context,
            selectedImage,
            selectedImage.DockerImageId,
            imageConnectorFactory,
            cancellationToken);
    }

    private static async Task<Result<ContainerAdoptionContext>> ResolveExternalAsync(
        ContainerAdoptionContext context,
        ExternalImage externalImage,
        IUnitOfWork unitOfWork,
        CancellationToken cancellationToken)
    {
        var registry = await unitOfWork.Registries.GetAsync(
            externalImage.RegistryId,
            cancellationToken);
        if (registry is null)
        {
            return Result.Failure<ContainerAdoptionContext>(
                new NotFoundError("The selected registry does not exist."));
        }

        var imageReference = new PullImageService.PullImageInput(
                context.Platform.Id,
                externalImage.RegistryId,
                externalImage.ImageTag)
            .ToConnectorCommand(context.Platform.Address, registry)
            .FromImage;

        if (!ReferencesSameRepository(context, [imageReference]))
        {
            return Result.Failure<ContainerAdoptionContext>(
                new BadRequestError(
                    "Select an external image from the container's original image repository."));
        }

        return context;
    }

    private static async Task<Result<ContainerAdoptionContext>> InspectAsync(
        ContainerAdoptionContext context,
        Image selectedImage,
        string imageReference,
        IConnectorFactory<IImageConnector> imageConnectorFactory,
        CancellationToken cancellationToken)
    {
        var inspectionResult = await imageConnectorFactory
            .GetConnector(context.Platform.ConnectorType)
            .InspectImageAsync(
                new InspectImageCommand(context.Platform.Address, imageReference),
                cancellationToken);
        if (!inspectionResult.IsSuccess(out var inspection))
            return Result.Failure<ContainerAdoptionContext>(inspectionResult.Errors);

        return ValidateInspection(context, selectedImage, inspection);
    }

    private static Result<ContainerAdoptionContext> ValidateInspection(
        ContainerAdoptionContext context,
        Image selectedImage,
        InspectImageResult inspection)
    {
        if (!string.Equals(inspection.Id, selectedImage.DockerImageId, StringComparison.OrdinalIgnoreCase))
        {
            return Result.Failure<ContainerAdoptionContext>(
                new ConflictError("The selected replacement image changed. Refresh the platform and try again."));
        }

        return context with { Image = selectedImage, ImageInspection = inspection };
    }

    private static bool ReferencesSameRepository(
        ContainerAdoptionContext context,
        IEnumerable<string> selectedReferences)
    {
        var sourceRepositories = (context.Image?.Tags ?? [])
            .Append(context.Inspection.Config?.Image ?? string.Empty)
            .Select(GetRepository)
            .Where(static value => value is not null)
            .ToHashSet(StringComparer.OrdinalIgnoreCase);

        return selectedReferences
            .Select(GetRepository)
            .Any(repository => repository is not null && sourceRepositories.Contains(repository));
    }

    private static string? GetRepository(string imageReference)
    {
        var value = imageReference.Trim().TrimStart('/').ToLowerInvariant();
        if (value.Length == 0
            || value.StartsWith("sha256:", StringComparison.Ordinal)
            || value.Contains("<none>", StringComparison.Ordinal))
        {
            return null;
        }

        var digestSeparator = value.LastIndexOf('@');
        if (digestSeparator > 0)
            value = value[..digestSeparator];

        var slash = value.LastIndexOf('/');
        var tagSeparator = value.LastIndexOf(':');
        if (tagSeparator > slash)
            value = value[..tagSeparator];

        var firstSlash = value.IndexOf('/');
        if (firstSlash < 0)
            return $"docker.io/library/{value}";

        var firstSegment = value[..firstSlash];
        if (string.Equals(firstSegment, "index.docker.io", StringComparison.Ordinal))
            return "docker.io" + value[firstSlash..];

        if (!firstSegment.Contains('.')
            && !firstSegment.Contains(':')
            && !string.Equals(firstSegment, "localhost", StringComparison.Ordinal))
        {
            return $"docker.io/{value}";
        }

        return value;
    }
}
