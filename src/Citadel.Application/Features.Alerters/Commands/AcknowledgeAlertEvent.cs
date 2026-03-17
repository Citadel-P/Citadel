using Domain.Contracts.Interfaces;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;

namespace Application.Features.Alerters.Commands;

public sealed record AcknowledgeAlertEvents(IReadOnlyCollection<Guid> Ids) : ICommand<Result>
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
    IHttpContextAccessor httpContextAccessor) : ICommandHandler<AcknowledgeAlertEvents, Result>
{
    public async ValueTask<Result> Handle(AcknowledgeAlertEvents command, CancellationToken cancellationToken)
    {
        var actorId = httpContextAccessor.HttpContext?.User?.GetActorId()
           ?? throw new ArgumentNullException($"{nameof(ClaimsPrincipal)} is missing");

        var ids = command.Ids.Distinct().ToArray();
        var alertEvents = (await unitOfWork.AlertEvents.GetByIdAsync(ids, cancellationToken)).ToList();
        if (alertEvents.Count != ids.Length)
            return Result.Failure(new NotFoundError("One or more alert events do not exist"));

        var utcNow = DateTime.UtcNow;
        try
        {
            foreach (var alertEvent in alertEvents)
                alertEvent.Acknowledge(actorId, utcNow);
        }
        catch (InvalidOperationException ex)
        {
            return Result.Failure(new BadRequestError(ex.Message));
        }

        await unitOfWork.AlertEvents.BulkUpdateAsync(alertEvents, cancellationToken);
        await unitOfWork.CommitAsync(cancellationToken);

        return Result.Success();
    }
}
