using Hosting.Common;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Queries;

[RequirePermission(ResourceType.Alert, PermissionLevel.Read)]
public sealed record GetAlertEvent(Guid Id) : IQuery<Result<AlertEvent>>;

internal sealed class GetAlertEventHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor) : IQueryHandler<GetAlertEvent, Result<AlertEvent>>
{
    public async ValueTask<Result<AlertEvent>> Handle(GetAlertEvent query, CancellationToken cancellationToken)
    {
        var user = userContextAccessor.Current;
        var alertEvent = user is not null && !user.IsAdmin
            ? await unitOfWork.AlertEvents.GetAuthorizedByIdAsync(
                user.ActorId,
                ResourceType.Alert,
                PermissionLevel.Read,
                SpecificPermission.None,
                query.Id,
                cancellationToken)
            : await unitOfWork.AlertEvents.GetByIdAsync(query.Id, cancellationToken);

        if (alertEvent is null && user is { IsAuthenticated: true, IsAdmin: false })
        {
            var exists = await unitOfWork.AlertEvents.GetByIdAsync(query.Id, cancellationToken);
            if (exists is not null)
                return Result.Failure<AlertEvent>(new ForbiddenError("Missing permission [Read] on [Alert]"));
        }

        return alertEvent ?? Result.Failure<AlertEvent>(new NotFoundError("Alert event does not exist"));
    }
}
