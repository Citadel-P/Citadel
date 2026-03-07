using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Queries;

public sealed record GetAlertRules : IQuery<Result<(IEnumerable<AlertRule> Rules, IEnumerable<AlertChannel> Channels)>>;

internal sealed class GetAlertRulesHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetAlertRules, Result<(IEnumerable<AlertRule> Rules, IEnumerable<AlertChannel> Channels)>>
{
    public async ValueTask<Result<(IEnumerable<AlertRule> Rules, IEnumerable<AlertChannel> Channels)>> Handle(GetAlertRules query, CancellationToken cancellationToken)
    {
        var rules = await unitOfWork.AlertRules.GetAllAsync(cancellationToken);
        var channels = await unitOfWork.AlertRules.GetAllChannelsAsync(cancellationToken);
        return Result.Success((rules, channels));
    }
}
