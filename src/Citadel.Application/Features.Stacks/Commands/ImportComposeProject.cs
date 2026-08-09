using Application.Features.Alerters.Notifications;
using Application.Features.Containers.Queries;
using Application.Features.Deployments.Notifications;
using Application.Features.Stacks.Queries;
using Application.Services;
using Application.Services.Builds;
using Application.Services.Licensing;
using Application.Services.SignalR;
using Application.TaskJobs;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Stacks;
using Domain.Contracts.Resources.Swarm;
using Domain.Entities.Activities;
using Domain.Entities.ResourceBindings;
using Domain.Entities.Stacks;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using Hosting.Common.Pipelines.Interfaces;
using LightResults;
using Mediator;

namespace Application.Features.Stacks.Commands;

[RequirePermission(ResourceType.Stack, PermissionLevel.Write)]
public sealed record ImportComposeProject(
    Guid PlatformId,
    string ProjectName,
    string Name,
    string? Description,
    StackSource StackSource,
    StackSpec Spec,
    string PreviewFingerprint,
    IReadOnlyCollection<Guid>? TagIds = null,
    StackImportKind? ImportKind = null,
    bool ImportSensitiveEnvironmentAsSecrets = false)
    : ICommand<Result<Stack>>
{
    internal sealed class Validator : AbstractValidator<ImportComposeProject>
    {
        public Validator()
        {
            RuleFor(x => x.Name).NotEmpty().ValidNameIdentifier();
            RuleFor(x => x.ProjectName).NotEmpty().MaximumLength(128);
            RuleFor(x => x.Description).MaximumLength(600);
            RuleFor(x => x.Spec).NotNull();
            RuleFor(x => x.PreviewFingerprint).NotEmpty().Length(64);
        }
    }
}

internal sealed class ImportComposeProjectHandler(
    IUnitOfWork unitOfWork,
    IConnectorFactory<IContainerConnector> connectorFactory,
    IContainerAuthorizationService containerAuthorizationService,
    IStackStreamManager stackStreamManager,
    IPlatformStreamManager platformStreamManager,
    IContainerStreamManager containerStreamManager,
    IActivityStreamManager activityStreamManager,
    IAlertEventStreamManager alertEventStreamManager,
    INotificationQueue notificationQueue,
    IUserContextAccessor userContext,
    ILicenseEntitlementService entitlementService,
    IGitStackMaterializer gitStackMaterializer,
    IPermissionService permissionService,
    IAdoptionFingerprintService fingerprintService,
    IConnectorFactory<ISwarmConnector> swarmConnectorFactory,
    ISwarmReconciliationCoordinator swarmReconciliationCoordinator,
    ISecretValueProtector secretValueProtector)
    : ICommandHandler<ImportComposeProject, Result<Stack>>
{
    public async ValueTask<Result<Stack>> Handle(
        ImportComposeProject command,
        CancellationToken cancellationToken)
    {
        var targetPlatform = await unitOfWork.Platforms.GetByIdAsync(command.PlatformId, cancellationToken);
        var importKind = command.ImportKind
            ?? (targetPlatform?.PlatformDescriptor.Type == PlatformType.DockerSwarm
                ? StackImportKind.SwarmStack
                : StackImportKind.ComposeProject);
        if (importKind == StackImportKind.SwarmStack)
        {
            if (targetPlatform?.PlatformDescriptor.Type != PlatformType.DockerSwarm)
                return Result.Failure<Stack>(new BadRequestError("Docker Stack import requires a Docker Swarm platform."));

            return await ImportSwarmStackAsync(command, cancellationToken);
        }

        var contextResult = await ComposeProjectImportDraftFactory.LoadContextAsync(
            command.PlatformId,
            command.ProjectName,
            unitOfWork,
            connectorFactory,
            containerAuthorizationService,
            cancellationToken);
        if (!contextResult.IsSuccess(out var context))
            return Result.Failure<Stack>(contextResult.Errors);

        var sourceResult = await ComposeProjectImportDraftFactory.AnalyzeSourceAsync(
            context,
            command.Name,
            command.StackSource,
            command.Spec,
            unitOfWork,
            gitStackMaterializer,
            permissionService,
            userContext,
            cancellationToken);
        if (!sourceResult.IsSuccess(out var source))
            return Result.Failure<Stack>(sourceResult.Errors);

        var validation = ComposeProjectImportDraftFactory.CreateValidation(context, source, fingerprintService);
        if (!fingerprintService.Matches(
                validation.PreviewFingerprint,
                command.PreviewFingerprint))
        {
            return Result.Failure<Stack>(
                new ConflictError("The Compose project or selected source changed. Validate the import again."));
        }

        var blocker = validation.Issues.FirstOrDefault(x => x.Severity == AdoptionIssueSeverity.Blocker);
        if (blocker is not null)
            return Result.Failure<Stack>(new BadRequestError(blocker.Message));

        var sensitiveBindings = ComposeProjectImportDraftFactory.GetSensitiveBindingAnalysis(context, source);
        if (command.ImportSensitiveEnvironmentAsSecrets && !sensitiveBindings.CanImport)
        {
            var message = sensitiveBindings.Issues.FirstOrDefault()?.Message
                          ?? "No sensitive Compose binding values are available to import.";
            return Result.Failure<Stack>(new BadRequestError(message));
        }

        if (await unitOfWork.Stacks.ExistsAsync(command.Name, cancellationToken))
            return Result.Failure<Stack>(new ConflictError("Name already exists."));

        var spec = BuildImageProvenance.Clear(source.SafeSpec);
        var driftPolicy = StackDriftPolicy.Disabled;
        var entitlement = await StackLicenseConfigurationPolicy.EnsureAllowedAsync(
            currentSpec: null,
            currentDriftPolicy: null,
            spec,
            driftPolicy,
            entitlementService,
            cancellationToken);
        if (entitlement.IsFailure(out var entitlementError))
            return Result.Failure<Stack>(entitlementError);

        var actorId = userContext.Current.ActorId;
        var stack = Stack.Create(
            name: command.Name,
            createdByActorId: actorId,
            StackSource: command.StackSource,
            platformId: context.Platform.Id,
            spec: spec,
            description: command.Description,
            driftPolicy: driftPolicy,
            platform: context.Platform);
        stack.PartialUpdate(Stack.ToStackStatus(context.ManagedContainers.Select(x => x.Container.State)));

        var added = await unitOfWork.Stacks.AddAsync(
            stack,
            cancellationToken,
            command.TagIds,
            actorId);
        if (added == 0)
            return Result.Failure<Stack>(new BadRequestError("One or more tags do not exist."));

        if (command.ImportSensitiveEnvironmentAsSecrets)
        {
            foreach (var importedBinding in sensitiveBindings.Bindings)
            {
                var secret = new SecretDefinition(
                    ContainerAdoptionDraftFactory.BuildImportedSecretName(
                        stack.Id,
                        stack.Name,
                        importedBinding.Name),
                    SecretProviderType.InternalEncrypted);
                var value = new InternalSecretValue(
                    secret.Id,
                    secretValueProtector.Protect(importedBinding.Value));
                var binding = new ResourceBinding(
                    Name: importedBinding.Name,
                    Kind: ResourceBindingKind.Secret,
                    Scope: ResourceBindingScope.Stack,
                    ResourceId: stack.Id,
                    Value: null,
                    SecretId: secret.Id,
                    SecretDeliveryMode: SecretDeliveryMode.EnvironmentVariable);

                await unitOfWork.SecretDefinitions.AddAsync(secret, value, cancellationToken);
                await unitOfWork.ResourceBindings.AddAsync(binding, cancellationToken);
            }
        }

        if (context.Platform.PlatformDescriptor.Type == PlatformType.DockerSwarm
            && !await unitOfWork.Stacks.TryReserveSwarmNamespaceAsync(
                stack.Id,
                context.Platform.Id,
                context.ProjectName,
                cancellationToken))
        {
            return Result.Failure<Stack>(new ConflictError(
                $"Docker Stack namespace '{context.ProjectName}' is already reserved by another Stack."));
        }

        var containerIds = context.ManagedContainers.Select(x => x.Container.Id).ToArray();
        var dockerContainerIds = context.ManagedContainers.Select(x => x.Container.DockerContainerId).ToArray();
        var assigned = await unitOfWork.Containers.TryAssignComposeProjectToStackAsync(
            context.Platform.Id,
            context.ProjectName,
            containerIds,
            dockerContainerIds,
            stack.Id,
            context.OrphanedOwnerStackId,
            cancellationToken);
        if (assigned != containerIds.Length)
        {
            return Result.Failure<Stack>(
                new ConflictError("Compose project is no longer eligible for import. Refresh the container list."));
        }

        foreach (var container in context.ManagedContainers)
            container.Container.PartialUpdate(stackId: stack.Id);

        var activity = new ActivityEvent(
            actorId: actorId,
            resourceId: stack.Id,
            platformId: context.Platform.Id,
            resourceName: stack.Name,
            eventType: ActivityEventType.StackImported,
            status: ActivityStatus.Information,
            info: new StackImported(
                stack.ToSnapshot(),
                context.ProjectName,
                [.. context.ManagedContainers
                    .Select(x => x.ServiceName)
                    .Where(name => !string.IsNullOrWhiteSpace(name))
                    .Distinct(StringComparer.OrdinalIgnoreCase)]));
        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);

        var resolvedAlerts = (await unitOfWork.AlertEvents.GetUnresolvedUnmanagedContainerAlertsAsync(
            dockerContainerIds,
            cancellationToken)).ToArray();
        var actor = resolvedAlerts.Length == 0
            ? null
            : await unitOfWork.Actors.GetById(actorId, cancellationToken);
        var utcNow = DateTime.UtcNow;
        foreach (var alert in resolvedAlerts)
        {
            var resolveResult = alert.Resolve(actorId, utcNow, "Compose project imported as a stack.");
            if (!resolveResult.IsSuccess())
                return Result.Failure<Stack>(resolveResult.Errors);

            alert.AssignActor(actor);
        }

        if (resolvedAlerts.Length > 0)
            await unitOfWork.AlertEvents.BulkUpdateAsync(resolvedAlerts, cancellationToken);

        var platform = await unitOfWork.Platforms.GetPlatformWithLatestStatAsync(
            context.Platform.Id,
            cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        await notificationQueue.EnqueueAsync(
            new StackNotificationWorkItem(stackStreamManager, stack, "create"),
            cancellationToken);
        await notificationQueue.EnqueueAsync(
            new SendContainersInfoNotificationWorkItem(
                containerStreamManager,
                context.ManagedContainers.Select(x => x.Container),
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

        return stack;
    }

    private async Task<Result<Stack>> ImportSwarmStackAsync(
        ImportComposeProject command,
        CancellationToken cancellationToken)
    {
        if (command.ImportSensitiveEnvironmentAsSecrets)
        {
            return Result.Failure<Stack>(new BadRequestError(
                "Docker Swarm does not expose existing Secret values for import."));
        }

        var contextResult = await SwarmStackImportDraftFactory.LoadContextAsync(
            command.PlatformId,
            command.ProjectName,
            unitOfWork,
            swarmConnectorFactory,
            permissionService,
            userContext,
            cancellationToken);
        if (!contextResult.IsSuccess(out var context))
            return Result.Failure<Stack>(contextResult.Errors);

        var sourceResult = await ComposeProjectImportDraftFactory.AnalyzeSourceAsync(
            context.Platform,
            context.Namespace,
            command.Name,
            command.StackSource,
            command.Spec,
            unitOfWork,
            gitStackMaterializer,
            permissionService,
            userContext,
            cancellationToken);
        if (!sourceResult.IsSuccess(out var source))
            return Result.Failure<Stack>(sourceResult.Errors);

        var validation = SwarmStackImportDraftFactory.CreateValidation(context, source, fingerprintService);
        if (!fingerprintService.Matches(validation.PreviewFingerprint, command.PreviewFingerprint))
        {
            return Result.Failure<Stack>(
                new ConflictError("The Docker Stack or selected source changed. Validate the import again."));
        }

        var blocker = validation.Issues.FirstOrDefault(static issue => issue.Severity == AdoptionIssueSeverity.Blocker);
        if (blocker is not null)
            return Result.Failure<Stack>(new BadRequestError(blocker.Message));
        if (await unitOfWork.Stacks.ExistsAsync(command.Name, cancellationToken))
            return Result.Failure<Stack>(new ConflictError("Name already exists."));

        var spec = BuildImageProvenance.Clear(source.SafeSpec);
        var driftPolicy = StackDriftPolicy.Disabled;
        var entitlement = await StackLicenseConfigurationPolicy.EnsureAllowedAsync(
            currentSpec: null,
            currentDriftPolicy: null,
            spec,
            driftPolicy,
            entitlementService,
            cancellationToken);
        if (entitlement.IsFailure(out var entitlementError))
            return Result.Failure<Stack>(entitlementError);

        var actorId = userContext.Current.ActorId;
        var stack = Stack.Create(
            command.Name,
            actorId,
            command.StackSource,
            context.Platform.Id,
            spec,
            command.Description,
            driftPolicy,
            context.Platform);
        var status = context.Services.All(static service => service.DesiredTaskCount == 0)
            ? StackReleaseStatus.Stopped
            : context.Services.All(static service => service.RunningTaskCount >= service.DesiredTaskCount)
                ? StackReleaseStatus.Healthy
                : StackReleaseStatus.Degraded;
        stack.PartialUpdate(status);

        var added = await unitOfWork.Stacks.AddAsync(stack, cancellationToken, command.TagIds, actorId);
        if (added == 0)
            return Result.Failure<Stack>(new BadRequestError("One or more tags do not exist."));

        if (!await unitOfWork.Stacks.TryReserveSwarmNamespaceAsync(
                stack.Id,
                context.Platform.Id,
                context.Namespace,
                cancellationToken))
        {
            return Result.Failure<Stack>(new ConflictError(
                $"Docker Stack namespace '{context.Namespace}' is already reserved by another Stack."));
        }

        var serviceIds = context.Services.Select(static service => service.Id).ToArray();
        var assigned = await unitOfWork.Swarm.TryAssignStackNamespaceAsync(
            context.Platform.Id,
            context.Namespace,
            serviceIds,
            stack.Id,
            cancellationToken);
        if (assigned != serviceIds.Length)
        {
            return Result.Failure<Stack>(new ConflictError(
                "Docker Stack Services are no longer eligible for import. Refresh the Services list."));
        }

        var stackTaskContainers = (await unitOfWork.Containers.GetByPlatformIdAsync(
                context.Platform.Id,
                cancellationToken))
            .Where(container => container.IsSwarmTask
                                && string.Equals(container.DockerStack, context.Namespace, StringComparison.Ordinal))
            .ToArray();
        if (stackTaskContainers.Any(container => container.DeploymentId is not null || container.StackId is not null))
        {
            return Result.Failure<Stack>(new ConflictError(
                "One or more Docker Stack task containers are already linked to another Citadel resource."));
        }

        if (stackTaskContainers.Length > 0)
        {
            var assignedContainers = await unitOfWork.Containers.TryAssignComposeProjectToStackAsync(
                context.Platform.Id,
                context.Namespace,
                stackTaskContainers.Select(static container => container.Id).ToArray(),
                stackTaskContainers.Select(static container => container.DockerContainerId).ToArray(),
                stack.Id,
                context.OrphanedOwnerStackId,
                cancellationToken);
            if (assignedContainers != stackTaskContainers.Length)
            {
                return Result.Failure<Stack>(new ConflictError(
                    "Docker Stack task containers changed during import. Refresh the Containers list."));
            }

            foreach (var container in stackTaskContainers)
                container.PartialUpdate(stackId: stack.Id);
        }

        var activity = new ActivityEvent(
            platformId: context.Platform.Id,
            resourceId: stack.Id,
            actorId: actorId,
            resourceName: stack.Name,
            eventType: ActivityEventType.StackImported,
            status: ActivityStatus.Information,
            info: new StackImported(
                stack.ToSnapshot(),
                context.Namespace,
                context.Services.Select(service =>
                        service.Name.StartsWith($"{context.Namespace}_", StringComparison.Ordinal)
                            ? service.Name[(context.Namespace.Length + 1)..]
                            : service.Name)
                    .ToArray()));
        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        var platform = await unitOfWork.Platforms.GetPlatformWithLatestStatAsync(context.Platform.Id, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        await notificationQueue.EnqueueAsync(new StackNotificationWorkItem(stackStreamManager, stack, "create"), cancellationToken);
        if (stackTaskContainers.Length > 0)
        {
            await notificationQueue.EnqueueAsync(
                new SendContainersInfoNotificationWorkItem(
                    containerStreamManager,
                    stackTaskContainers,
                    context.Platform.Id),
                cancellationToken);
        }
        await notificationQueue.EnqueueAsync(
            new ActivityNotificationWorkItem(activityStreamManager, await activity.AssignActor(unitOfWork, cancellationToken)),
            cancellationToken);
        if (platform is not null)
            await platformStreamManager.PushPlatformUpdate(platform);

        await swarmReconciliationCoordinator.RefreshAsync(context.Platform.Id, CancellationToken.None);
        return stack;
    }
}
