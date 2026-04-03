using Application.Features.Deployments.Notifications;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Activities;
using Domain.Entities.Registries;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;

namespace Application.Features.Registries.Commands;

[RequirePermission(nameof(AppPermission.Registry_Update))]
public sealed record RenameRegistry(Guid Id, string Name) : ICommand<Result<Registry>>
{
    internal sealed class Validator : AbstractValidator<RenameRegistry>
    {
        public Validator()
        {
            RuleFor(x => x.Id).NotEmpty().NotNull();
            RuleFor(x => x.Name).NotEmpty().ValidNameIdentifier();
        }
    }
}

internal sealed class RenameRegistryHandler(
    IUnitOfWork unitOfWork,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue,
    IHttpContextAccessor httpContextAccessor) : ICommandHandler<RenameRegistry, Result<Registry>>
{
    public async ValueTask<Result<Registry>> Handle(RenameRegistry command, CancellationToken cancellationToken)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
           ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        var registry = await unitOfWork.Registries.GetAsync(command.Id, cancellationToken);
        if (registry == null)
        {
            return Result.Failure<Registry>(new NotFoundError("The provided registry does not exist"));
        }

        var conflict = await unitOfWork.Registries.ExistsAsync(command.Id, command.Name, cancellationToken);
        if (conflict)
        {
            return Result.Failure<Registry>(new ConflictError("Name already exists"));
        }

        var oldName = registry.Name;
        registry.PartialUpdate(name: command.Name);

        var activity = new ActivityEvent(
               actorId: actorId,
               resourceId: registry.Id,
               platformId: null,
               resourceName: command.Name,
               eventType: ActivityEventType.RegistryRenamed,
               status: ActivityStatus.Success,
               info: new RegistryRenamed(oldName, command.Name)
           );

        await unitOfWork.Registries.UpdateAsync(registry, cancellationToken);
        await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);

        return registry;
    }
}
