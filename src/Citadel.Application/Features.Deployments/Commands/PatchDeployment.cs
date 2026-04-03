using Application.Features.Deployments.Notifications;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Deployments;
using Domain.Entities.Activities;
using Domain.Entities.Deployments;
using FluentValidation;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using Hosting.Common.MergePatch;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;
using ActivityEvent = Domain.Entities.Activities.ActivityEvent;

namespace Application.Features.Deployments.Commands;

[RequirePermission(nameof(AppPermission.Deploymen_Update))]
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

internal sealed class PatchDeploymentHandler(IUnitOfWork unitOfWork, IDeploymentStreamManager deploymentHub, INotificationQueue notificationQueue, 
    IActivityStreamManager activityHub, IHttpContextAccessor httpContextAccessor) : ICommandHandler<PatchDeployment, Result<Deployment>>
{
    public async ValueTask<Result<Deployment>> Handle(PatchDeployment command, CancellationToken cancellationToken)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
           ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        var deployment = await unitOfWork.Deployments.GetAsync(command.Id, cancellationToken);
        if (deployment == null)
        {
            return Result.Failure<Deployment>(new NotFoundError("The provided deployment does not exist"));
        }

        var patchedDeployment = command.Patch.ApplyTo(deployment, DeploymentJsonContext.Default.Deployment);
        
        if (patchedDeployment.Spec?.Image is not ExternalImage)
        {
            if (patchedDeployment.Spec?.UpdateBehavior != UpdateBehavior.Disabled)
            {
                return Result.Failure<Deployment>(new BadRequestError("Auto-update requires an external image source."));
            }
        }

        if (patchedDeployment.Spec?.Image is ExternalImage extImage && extImage.ImageTag.Contains('@'))
        {
            if (patchedDeployment.Spec?.UpdateBehavior != UpdateBehavior.Disabled)
            {
                return Result.Failure<Deployment>(new BadRequestError("Cannot enable Auto-update for an image pinned by digest (contains '@')."));
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

        deployment.PartialUpdate(
            platformId: patchedDeployment.PlatformId,
            spec: patchedDeployment.Spec);

        await unitOfWork.Deployments.UpdateAsync(deployment, cancellationToken);

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);

        var workItem = new DeploymentNotificationWorkItem(deploymentHub, deployment);
        
        // Notify
        await notificationQueue.EnqueueAsync(workItem, cancellationToken);
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);
        return deployment;
    }
}
