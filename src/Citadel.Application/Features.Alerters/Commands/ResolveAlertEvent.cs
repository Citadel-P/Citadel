using Application.Features.Alerters.Notifications;
using Application.Services.SignalR;
using Domain.Contracts.Interfaces;
using FluentValidation;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Commands;

[RequirePermission(ResourceType.Alert, PermissionLevel.Write)]
public sealed record ResolveAlertEvents(IEnumerable<Guid> Ids, string? ResolutionNote = null) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<ResolveAlertEvents>
    {
        public Validator()
        {
            RuleFor(x => x.Ids).NotEmpty();
            RuleFor(x => x.ResolutionNote).MaximumLength(1000);
        }
    }
}

internal sealed class ResolveAlertEventsHandler(
    IUnitOfWork unitOfWork,
    INotificationQueue notificationQueue,
    IAlertEventStreamManager alertEventStreamManager,
    IUserContextAccessor userContext) : ICommandHandler<ResolveAlertEvents, Result>
{
    public async ValueTask<Result> Handle(ResolveAlertEvents command, CancellationToken cancellationToken)
    {
        var actorId = userContext.Current.ActorId;
        IEnumerable<Guid> ids = command.Ids;
        if (!ids.TryGetNonEnumeratedCount(out var idCount) || idCount > 1)
        {
            ids = [.. ids.Distinct()];
            idCount = ids.Count();
        }

        var alertEvents = await unitOfWork.AlertEvents.GetByIdAsync(ids, cancellationToken);
        if (alertEvents.Count() != idCount)
            return Result.Failure(new NotFoundError("One or more alert events do not exist"));

        var actor = await unitOfWork.Actors.GetById(actorId, cancellationToken);

        var utcNow = DateTime.UtcNow;
        foreach (var alertEvent in alertEvents)
        {
            var resolveResult = alertEvent.Resolve(actorId, utcNow, command.ResolutionNote);
            if (!resolveResult.IsSuccess())
                return resolveResult;

            alertEvent.AssignActor(actor);
        }

        await unitOfWork.AlertEvents.BulkUpdateAsync(alertEvents, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        var unresolvedCount = await unitOfWork.AlertEvents.CountUnresolvedAsync(cancellationToken);

        await notificationQueue.EnqueueAsync(
            new UpdatedAlertEventsNotificationWorkItem(alertEvents, alertEventStreamManager),
            cancellationToken);

        await notificationQueue.EnqueueAsync(
            new UnresolvedAlertCountNotificationWorkItem(unresolvedCount, alertEventStreamManager),
            cancellationToken);

        return Result.Success();
    }
}
