using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities;
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
    UpdateBehavior UpdateBehavior,
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

internal class CreateDeploymentHandler(IUnitOfWork unitOfWork, INotificationQueue notificationQueue, IHttpContextAccessor httpContextAccessor) : ICommandHandler<CreateDeployment, Result<Deployment>>
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
            if (command.UpdateBehavior != UpdateBehavior.Disabled)
            {
                return Result.Failure<Deployment>(new BadRequestError("Auto-update requires an external image source."));
            }
        }

        if (command.Spec.Image is ExternalImage extImage && extImage.ImageTag.Contains('@'))
        {
            if (command.UpdateBehavior != UpdateBehavior.Disabled)
            {
                return Result.Failure<Deployment>(new BadRequestError("Cannot enable Auto-update for an image pinned by digest (contains '@')."));
            }
        }

        // Add deployment
        var deployment = new Deployment(
            name : command.Name,
            description : command.Description,
            createdByActorId: actorId,
            platformId : command.PlatformId,
            updateBehavior : command.UpdateBehavior,
            spec : command.Spec
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
            info: new DeploymentCreated(command.Spec)
            );

        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);

        await unitOfWork.CommitAsync(cancellationToken);

        // Notify
        await notificationQueue.EnqueueAsync(new ActivityNotification(activity), cancellationToken);
        return deployment;
    }
}
