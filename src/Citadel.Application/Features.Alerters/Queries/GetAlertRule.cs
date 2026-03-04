using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Queries;

public record GetAlertRule(Guid Id) : IQuery<Result<(AlertRule Rule, IEnumerable<AlertChannel> Channels)>>;

internal sealed class GetAlertRuleHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetAlertRule, Result<(AlertRule Rule, IEnumerable<AlertChannel> Channels)>>
{
    public async ValueTask<Result<(AlertRule Rule, IEnumerable<AlertChannel> Channels)>> Handle(GetAlertRule query, CancellationToken cancellationToken)
    {
        var rule = await unitOfWork.AlertRules.GetByIdAsync(query.Id, cancellationToken);
        if (rule is null)
        {
            return Result.Failure<(AlertRule Rule, IEnumerable<AlertChannel> Channels)>(new NotFoundError("Alert rule does not exist"));
        }

        var channels = await unitOfWork.AlertRules.GetAllChannelsAsync(cancellationToken);
        return Result.Success((rule, channels));
    }
}
