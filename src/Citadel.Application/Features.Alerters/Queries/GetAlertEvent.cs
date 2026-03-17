using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Queries;

public sealed record GetAlertEvent(Guid Id) : IQuery<Result<AlertEvent>>;

internal sealed class GetAlertEventHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetAlertEvent, Result<AlertEvent>>
{
    public async ValueTask<Result<AlertEvent>> Handle(GetAlertEvent query, CancellationToken cancellationToken)
    {
        var alertEvent = await unitOfWork.AlertEvents.GetByIdAsync(query.Id, cancellationToken);
        return alertEvent ?? Result.Failure<AlertEvent>(new NotFoundError("Alert event does not exist"));
    }
}
