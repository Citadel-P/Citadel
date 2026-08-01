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
using Domain.Entities.Activities;
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
    IReadOnlyCollection<Guid>? TagIds = null)
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
    IAdoptionFingerprintService fingerprintService)
    : ICommandHandler<ImportComposeProject, Result<Stack>>
{
    public async ValueTask<Result<Stack>> Handle(
        ImportComposeProject command,
        CancellationToken cancellationToken)
    {
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
            driftPolicy: driftPolicy);
        stack.PartialUpdate(Stack.ToStackStatus(context.ManagedContainers.Select(x => x.Container.State)));

        var added = await unitOfWork.Stacks.AddAsync(
            stack,
            cancellationToken,
            command.TagIds,
            actorId);
        if (added == 0)
            return Result.Failure<Stack>(new BadRequestError("One or more tags do not exist."));

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
}
