using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Queries;

public record GetAlertChannels : IQuery<Result<IEnumerable<AlertChannel>>>;

internal sealed class GetAlertChannelsHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetAlertChannels, Result<IEnumerable<AlertChannel>>>
{
    public async ValueTask<Result<IEnumerable<AlertChannel>>> Handle(GetAlertChannels query, CancellationToken cancellationToken)
    {
        var channels = await unitOfWork.AlertRules.GetAllChannelsAsync(cancellationToken);
        return Result.Success(channels);
    }
}
