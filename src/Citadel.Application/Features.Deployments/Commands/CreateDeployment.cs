using Application.Features.Deployments.Notifications;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Deployments;
using Domain.Entities.Activities;
using Domain.Entities.Deployments;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;

namespace Application.Features.Deployments.Commands;

[RequirePermission(nameof(AppPermission.Deploymen_Create))]
public sealed record CreateDeployment(
    string Name,
    Guid PlatformId,
    string? Description,
    DeploymentSpec Spec) 
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
    INotificationQueue notificationQueue, 
    IActivityStreamManager activityHub, 
    IHttpContextAccessor httpContextAccessor) : ICommandHandler<CreateDeployment, Result<Deployment>>
{
    public async ValueTask<Result<Deployment>> Handle(CreateDeployment command, CancellationToken cancellationToken)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
           ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        var exist = await unitOfWork.Deployments.ExistsAsync(command.Name, command.PlatformId, cancellationToken);
        if (exist)
        {
            return Result.Failure<Deployment>(new ConflictError("Name already exists"));
        }

        if (command.Spec.Image is not ExternalImage)
        {
            if (command.Spec.UpdateBehavior != UpdateBehavior.Disabled)
            {
                return Result.Failure<Deployment>(new BadRequestError("Auto-update requires an external image source."));
            }
        }

        if (command.Spec.Image is ExternalImage extImage && extImage.ImageTag.Contains('@'))
        {
            if (command.Spec.UpdateBehavior != UpdateBehavior.Disabled)
            {
                return Result.Failure<Deployment>(new BadRequestError("Cannot enable Auto-update for an image pinned by digest (contains '@')."));
            }
        }

        // Add deployment
        var deployment = new Deployment(
            name: command.Name,
            description: command.Description,
            createdByActorId: actorId,
            platformId: command.PlatformId,
            spec: command.Spec
            );

        var result = await unitOfWork.Deployments.AddAsync(deployment, cancellationToken);

        // Add activity
        var activity = new ActivityEvent(
            actorId: actorId,
            resourceId: deployment.Id,
            platformId: command.PlatformId,
            resourceName: deployment.Name,
            eventType: ActivityEventType.DeploymentCreated,
            status: ActivityStatus.Information,
            info: new DeploymentCreated(deployment.ToSnapshot())
            );

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);

        // Notify
        var workItem = new DeploymentNotificationWorkItem(deploymentHub, deployment, "create");

        await notificationQueue.EnqueueAsync(workItem, cancellationToken);
        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);
        return deployment;
    }
}
