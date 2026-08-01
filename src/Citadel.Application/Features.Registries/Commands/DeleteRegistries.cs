using Application.Features.Deployments.Notifications;
using Application.Services.SignalR;
using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Registries;
using Domain.Entities.Activities;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Microsoft.Extensions.Logging;

namespace Application.Features.Registries.Commands;

[RequirePermission(ResourceType.Registry, PermissionLevel.Execute)]
public sealed record DeleteRegistries(IEnumerable<Guid> Ids) : ICommand<Result>;

internal class DeleteRegistriesHandler(
    IUnitOfWork unitOfWork,
    IActivityStreamManager activityHub,
    INotificationQueue notificationQueue,
    IUserContextAccessor userContext,
    ILogger<DeleteRegistriesHandler> logger) : ICommandHandler<DeleteRegistries, Result>
{
    public async ValueTask<Result> Handle(DeleteRegistries command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        var requestedIds = command.Ids.Distinct().ToArray();
        var toDelete = (await unitOfWork.Registries.GetAllAsync(requestedIds, cancellationToken) ?? [])
            .ToArray();

        if (requestedIds.Length == 0 || toDelete.Length != requestedIds.Length)
        {
            return Result.Failure(new NotFoundError("One or more registries were not found."));
        }

        var ids = toDelete.Select(s => s.Id).ToArray();
        var notifications = new List<INotificationWorkItem>(ids.Length);
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
            notifications.Add(new ActivityNotificationWorkItem(
                activityHub,
                await activity.AssignActor(unitOfWork, cancellationToken)));
        }

        var result = await unitOfWork.Registries.RemoveRangeAsync(ids, cancellationToken);
        if (result != ids.Length)
        {
            await unitOfWork.RollbackAsync();
            return Result.Failure(new ConflictError(
                "The registry set changed while deletion was in progress."));
        }

        await unitOfWork.CommitAsync(cancellationToken);

        using var notificationCancellation = new CancellationTokenSource(TimeSpan.FromSeconds(5));
        foreach (var notification in notifications)
        {
            try
            {
                await notificationQueue.EnqueueAsync(notification, notificationCancellation.Token);
            }
            catch (Exception ex)
            {
                logger.LogWarning(ex, "Failed to publish deleted registry activity");
            }
        }

        return Result.Success();
    }
}
