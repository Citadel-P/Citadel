using Application.Features.Deployments.Notifications;
using Application.Services.Builds;
using Application.Services.SignalR;
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
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;
using ActivityEvent = Domain.Entities.Activities.ActivityEvent;

namespace Application.Features.Deployments.Commands;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Write)]
public sealed record PatchDeployment(Guid Id, JsonMergePatchDocument<Deployment> Patch) : ICommand<Result<Deployment>>
{
    internal sealed class Validator : PatchCommandValidator<PatchDeployment, Deployment>
    {
        public Validator()
            : base(
                  patchSelector: x => x.Patch,
                  jsonTypeInfo: DeploymentJsonContext.Default.Deployment,
                  modelValidator: new DeploymentValidator()
                  )
        { }
    }

    internal sealed class DeploymentValidator : AbstractValidator<Deployment>
    {
        public DeploymentValidator()
        {

            RuleFor(x => x.Id).NotEmpty().NotNull();
        }
    }
}

internal sealed class PatchDeploymentHandler(IUnitOfWork unitOfWork, IDeploymentStreamManager deploymentHub, IPlatformStreamManager platformHub, INotificationQueue notificationQueue,
    IActivityStreamManager activityHub, IUserContextAccessor userContext) : ICommandHandler<PatchDeployment, Result<Deployment>>
{
    public async ValueTask<Result<Deployment>> Handle(PatchDeployment command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        var deployment = await unitOfWork.Deployments.GetAsync(command.Id, cancellationToken);
        if (deployment == null)
        {
            return Result.Failure<Deployment>(new NotFoundError("The provided deployment does not exist"));
        }
        var previousPlatformId = deployment.PlatformId;

        var patchedDeployment = command.Patch.ApplyTo(deployment, DeploymentJsonContext.Default.Deployment);
        
        if (patchedDeployment.Spec is not null)
        {
            patchedDeployment.PartialUpdate(
                spec: BuildImageProvenance.Preserve(patchedDeployment.Spec, deployment.Spec));

            var imageValidation = await DeploymentImageValidation.ValidateAsync(patchedDeployment.Spec, unitOfWork, cancellationToken);
            if (imageValidation.IsFailure(out var imageError))
            {
                return Result.Failure<Deployment>(imageError);
            }
        }

        // Add activity
        var activity = new ActivityEvent(
                actorId: actorId,
                resourceId: deployment.Id,
                resourceName: deployment.Name,
                status: ActivityStatus.Success,
                platformId: deployment.PlatformId,
                eventType: ActivityEventType.DeploymentUpdated,
                info: new DeploymentUpdated(deployment.ToSnapshot(), patchedDeployment.ToSnapshot(command.Id))
            );

        // Update deployment
        deployment.PartialUpdate(
            platformId: patchedDeployment.PlatformId,
            spec: patchedDeployment.Spec);

        await unitOfWork.Deployments.UpdateAsync(deployment, cancellationToken);

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        Guid[] platformIds = previousPlatformId == deployment.PlatformId
            ? [deployment.PlatformId]
            : new[] { previousPlatformId, deployment.PlatformId };
        var platforms = await unitOfWork.Platforms.GetPlatformsWithLatestStatByIdsAsync(platformIds, cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);

        var workItem = new DeploymentNotificationWorkItem(deploymentHub, deployment);
        
        // Notify
        await notificationQueue.EnqueueAsync(workItem, cancellationToken);
        foreach (var platform in platforms)
        {
            await platformHub.PushPlatformUpdate(platform);
        }

        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);
        return deployment;
    }
}
