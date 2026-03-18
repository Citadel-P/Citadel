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
    IHttpContextAccessor httpContextAccessor) : ICommandHandler<ResolveAlertEvents, Result>
{
    public async ValueTask<Result> Handle(ResolveAlertEvents command, CancellationToken cancellationToken)
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
        if (alertEvents.Count != idCount)
            return Result.Failure(new NotFoundError("One or more alert events do not exist"));

        var utcNow = DateTime.UtcNow;
        foreach (var alertEvent in alertEvents)
        {
            var resolveResult = alertEvent.Resolve(actorId, utcNow, command.ResolutionNote);
            if (!resolveResult.IsSuccess())
                return resolveResult;
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
