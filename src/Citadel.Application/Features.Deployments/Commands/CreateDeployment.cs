using Application.Features.Deployments.Notifications;
using Application.Services.Builds;
using Application.Services.SignalR;
using Application.Services.Licensing;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Deployments;
using Domain.Entities.Activities;
using Domain.Entities.Deployments;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Commands;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Write)]
public sealed record CreateDeployment(
    string Name,
    Guid PlatformId,
    string? Description,
    DeploymentSpec Spec,
    IReadOnlyCollection<Guid>? TagIds = null,
    ActivitySourceResource? DuplicateSource = null)
    : ICommand<Result<Deployment>>
{
    internal sealed class Validator : AbstractValidator<CreateDeployment>
    {
        public Validator()
        {
            RuleFor(x => x.Name).NotEmpty().ValidNameIdentifier();
            RuleFor(x => x.Description).MaximumLength(600);
        }
    }
}

internal class CreateDeploymentHandler(
    IUnitOfWork unitOfWork,
    IDeploymentStreamManager deploymentHub,
    IPlatformStreamManager platformHub,
    INotificationQueue notificationQueue,
    IActivityStreamManager activityHub,
    IUserContextAccessor userContext,
    ILicenseEntitlementService entitlementService) : ICommandHandler<CreateDeployment, Result<Deployment>>
{
    public async ValueTask<Result<Deployment>> Handle(CreateDeployment command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        var exist = await unitOfWork.Deployments.ExistsAsync(command.Name, command.PlatformId, cancellationToken);
        if (exist)
        {
            return Result.Failure<Deployment>(new ConflictError("Name already exists"));
        }

        var spec = BuildImageProvenance.Clear(command.Spec);
        if (DeploymentLicenseConfigurationPolicy.ExpandsOperationalGuardrails(
                current: null,
                spec))
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.OperationalGuardrails,
                cancellationToken);
            if (entitlement.IsFailure(out var entitlementError))
                return Result.Failure<Deployment>(entitlementError);
        }

        if (DeploymentLicenseConfigurationPolicy.ExpandsAutomatedOperations(
                current: null,
                spec))
        {
            var entitlement = await entitlementService.EnsureEnabledAsync(
                LicenseCapability.AutomatedOperations,
                cancellationToken);
            if (entitlement.IsFailure(out var entitlementError))
                return Result.Failure<Deployment>(entitlementError);
        }

        var imageValidation = await DeploymentImageValidation.ValidateAsync(spec, unitOfWork, cancellationToken);
        if (imageValidation.IsFailure(out var imageError))
        {
            return Result.Failure<Deployment>(imageError);
        }

        var duplicateSourceResult = await GetValidDuplicateSourceAsync(command.DuplicateSource, cancellationToken);
        if (!duplicateSourceResult.IsSuccess(out var duplicateSource))
        {
            return Result.Failure<Deployment>(duplicateSourceResult.Errors);
        }

        var deployment = new Deployment(
            name: command.Name,
            description: command.Description,
            createdByActorId: actorId,
            platformId: command.PlatformId,
            spec: spec);

        var result = await unitOfWork.Deployments.AddAsync(deployment, cancellationToken, command.TagIds, actorId);
        if (result == 0)
            return Result.Failure<Deployment>(new BadRequestError("One or more tags do not exist."));

        var eventType = duplicateSource is null
            ? ActivityEventType.DeploymentCreated
            : ActivityEventType.DeploymentDuplicated;
        ActivityEventInfo info = duplicateSource is null
            ? new DeploymentCreated(deployment.ToSnapshot())
            : new DeploymentDuplicated(deployment.ToSnapshot(), duplicateSource);

        var activity = new ActivityEvent(
            actorId: actorId,
            resourceId: deployment.Id,
            platformId: command.PlatformId,
            resourceName: deployment.Name,
            eventType: eventType,
            status: ActivityStatus.Information,
            info: info);

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        var platform = await unitOfWork.Platforms.GetPlatformWithLatestStatAsync(command.PlatformId, cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);

        var workItem = new DeploymentNotificationWorkItem(deploymentHub, deployment, "create");

        await notificationQueue.EnqueueAsync(workItem, cancellationToken);
        if (platform is not null)
        {
            await platformHub.PushPlatformUpdate(platform);
        }

        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);
        return deployment;
    }

    private async Task<Result<ActivitySourceResource?>> GetValidDuplicateSourceAsync(
        ActivitySourceResource? source,
        CancellationToken cancellationToken)
    {
        if (source is null)
            return Result.Success<ActivitySourceResource?>(null);

        if (source.ResourceType != ActivityResourceType.Deployment)
            return Result.Failure<ActivitySourceResource?>(new BadRequestError("Duplicate source must be a deployment."));

        var sourceDeployment = await unitOfWork.Deployments.GetAsync(source.ResourceId, cancellationToken);
        if (sourceDeployment is null)
            return Result.Failure<ActivitySourceResource?>(new NotFoundError("Duplicate source deployment does not exist."));

        var user = userContext.Current;
        if (!user.IsAdmin && !await unitOfWork.Deployments.CanAccessAsync(user.UserId, source.ResourceId, cancellationToken))
            return Result.Failure<ActivitySourceResource?>(new ForbiddenError("Missing permission [Read] on duplicate source deployment."));

        return Result.Success<ActivitySourceResource?>(new ActivitySourceResource(
            ActivityResourceType.Deployment,
            sourceDeployment.Id,
            sourceDeployment.Name));
    }
}

internal static class DeploymentImageValidation
{
    internal static async Task<Result> ValidateAsync(
        DeploymentSpec spec,
        IUnitOfWork unitOfWork,
        CancellationToken cancellationToken)
    {
        if (spec.Image is BuildImage buildImage)
        {
            if (spec.UpdateBehavior != UpdateBehavior.Disabled)
                return Result.Failure(new BadRequestError("Build images use Redeploy on build instead of registry auto-update."));

            if (buildImage.BuildProjectId == Guid.Empty)
                return Result.Failure(new BadRequestError("Build project is required."));

            if (await unitOfWork.BuildProjects.GetAsync(buildImage.BuildProjectId, cancellationToken, includeArchived: true) is null)
                return Result.Failure(new NotFoundError("Build project not found."));

            return Result.Success();
        }

        if (spec.Image is not ExternalImage)
        {
            return spec.UpdateBehavior == UpdateBehavior.Disabled
                ? Result.Success()
                : Result.Failure(new BadRequestError("Auto-update requires an external image source."));
        }

        if (spec.Image is ExternalImage extImage && extImage.ImageTag.Contains('@') && spec.UpdateBehavior != UpdateBehavior.Disabled)
            return Result.Failure(new BadRequestError("Cannot enable Auto-update for an image pinned by digest (contains '@')."));

        return Result.Success();
    }
}
