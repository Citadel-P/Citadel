using Domain.Contracts.Interfaces;
using FluentValidation;
using Application.Features.Alerters.Notifications;
using Application.Services.SignalR;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;

namespace Application.Features.Alerters.Commands;

public sealed record AcknowledgeAlertEvents(IEnumerable<Guid> Ids) : ICommand<Result>
{
    internal sealed class Validator : AbstractValidator<AcknowledgeAlertEvents>
    {
        public Validator()
        {
            RuleFor(x => x.Ids).NotEmpty();
        }
    }
}

internal sealed class AcknowledgeAlertEventsHandler(
    IUnitOfWork unitOfWork,
    INotificationQueue notificationQueue,
    IAlertEventStreamManager alertEventStreamManager,
    IHttpContextAccessor httpContextAccessor) : ICommandHandler<AcknowledgeAlertEvents, Result>
{
    public async ValueTask<Result> Handle(AcknowledgeAlertEvents command, CancellationToken cancellationToken)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
           ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

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
            var acknowledgeResult = alertEvent.Acknowledge(actorId, utcNow);
            if (!acknowledgeResult.IsSuccess())
                return acknowledgeResult;

            alertEvent.AssignActor(actor);
        }

        await unitOfWork.AlertEvents.BulkUpdateAsync(alertEvents, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        await notificationQueue.EnqueueAsync(
            new UpdatedAlertEventsNotificationWorkItem(alertEvents, alertEventStreamManager),
            cancellationToken);

        return Result.Success();
    }
}
