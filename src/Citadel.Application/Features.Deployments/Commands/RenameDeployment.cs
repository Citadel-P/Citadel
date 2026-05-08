using Application.Features.Deployments.Notifications;
using Application.Services.SignalR;
using Application.TaskJobs.WorkItems;
using Domain;
using Domain.Contracts.Interfaces;
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
using ActivityEvent = Domain.Entities.Activities.ActivityEvent;

namespace Application.Features.Deployments.Commands;

[RequirePermission(ResourceType.Deployment, PermissionLevel.Write)]
public sealed record RenameDeployment(Guid Id, string Name) : ICommand<Result<Deployment>>
{
    internal sealed class Validator : AbstractValidator<RenameDeployment>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty().NotNull();
            RuleFor(x => x.Name).NotEmpty().ValidNameIdentifier();
        }
    }
}

internal sealed class RenameDeploymentHandler(
    IUnitOfWork unitOfWork, 
    IDeploymentStreamManager deploymentHub,
    INotificationQueue notificationQueue, 
    IActivityStreamManager activityHub,
    IHttpContextAccessor httpContextAccessor) : ICommandHandler<RenameDeployment, Result<Deployment>>
{
    public async ValueTask<Result<Deployment>> Handle(RenameDeployment command, CancellationToken cancellationToken)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
           ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        var deployment = await unitOfWork.Deployments.GetAsync(command.Id, cancellationToken);
        if (deployment == null)
        {
            return Result.Failure<Deployment>(new NotFoundError("The provided deployment does not exist"));
        }

        var conflict = await unitOfWork.Deployments.ExistsAsync(command.Id, command.Name, deployment.PlatformId, cancellationToken);
        if (conflict)
        {
            return Result.Failure<Deployment>(new ConflictError("Name already exists"));
        }

        // Activity
        var activity = new ActivityEvent(
               actorId: actorId,
               resourceId: deployment.Id,
               platformId: deployment.PlatformId,
               resourceName: command.Name,
               eventType: ActivityEventType.DeploymentRenamed,
               status: ActivityStatus.Success,
               info: new DeploymentRenamed(deployment.Name, command.Name)
           );

        deployment.PartialUpdate(name: command.Name);

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
