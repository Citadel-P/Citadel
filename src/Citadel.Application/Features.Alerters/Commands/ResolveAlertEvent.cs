using Domain.Contracts.Interfaces;
using FluentValidation;
using Hosting.Common.ErrorTypes;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;
using System.Security.Claims;

namespace Application.Features.Alerters.Commands;

public sealed record ResolveAlertEvents(IReadOnlyCollection<Guid> Ids, string? ResolutionNote = null) : ICommand<Result>
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
    IHttpContextAccessor httpContextAccessor) : ICommandHandler<ResolveAlertEvents, Result>
{
    public async ValueTask<Result> Handle(ResolveAlertEvents command, CancellationToken cancellationToken)
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
                alertEvent.Resolve(actorId, utcNow, command.ResolutionNote);
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
