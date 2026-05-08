using Application.Features.Deployments.Notifications;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Registries;
using Domain.Entities.Activities;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Hosting.Common;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;

namespace Application.Features.Registries.Commands;

[RequirePermission(ResourceType.Registry, PermissionLevel.Execute)]
public sealed record DeleteRegistries(IEnumerable<Guid> Ids) : ICommand<Result>;

internal class DeleteRegistriesHandler(
    IUnitOfWork unitOfWork,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue,
    IHttpContextAccessor httpContextAccessor) : ICommandHandler<DeleteRegistries, Result>
{
    public async ValueTask<Result> Handle(DeleteRegistries command, CancellationToken cancellationToken)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
           ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        var toDelete = await unitOfWork.Registries.GetAllAsync(command.Ids, cancellationToken);

        if (toDelete == null || toDelete.Any() == false)
        {
            return Result.Failure(new NotFoundError("No registries found matching the provided IDs for deletion."));
        }

        var ids = toDelete.Select(s => s.Id).ToList();
        foreach (var registry in toDelete)
        {
            var activity = new ActivityEvent(
                actorId: actorId,
                resourceId: registry.Id,
                platformId: null,
                resourceName: registry.Name,
                status: ActivityStatus.Success,
                eventType: ActivityEventType.RegistryDeleted,
                info: new RegistryDeleted(registry.ToSnapshot())
            );

            await unitOfWork.ActivityEventRepository.AddAsync(activity, cancellationToken);
            await notificationQueue.EnqueueAsync(new ActivityNotificationWorkItem(activityHub, await activity.AssignActor(unitOfWork, cancellationToken)), cancellationToken);
        }

        var result = await unitOfWork.Registries.RemoveRangeAsync(command.Ids, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return result > 0
            ? Result.Success()
            : Result.Failure(new NotFoundError("No registries found matching the provided IDs for deletion."));
    }
} 