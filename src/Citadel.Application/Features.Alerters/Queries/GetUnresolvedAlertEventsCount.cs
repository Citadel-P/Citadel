using Domain.Contracts.Interfaces;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Queries;

public sealed record GetUnresolvedAlertEventsCount : IQuery<Result<int>>;

internal sealed class GetUnresolvedAlertEventsCountHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetUnresolvedAlertEventsCount, Result<int>>
{
    public async ValueTask<Result<int>> Handle(GetUnresolvedAlertEventsCount query, CancellationToken cancellationToken)
    {
        var count = await unitOfWork.AlertEvents.CountUnresolvedAsync(cancellationToken);
        return Result.Success(count);
    }
}
