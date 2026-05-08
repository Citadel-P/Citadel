using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;
using Hosting.Common;

namespace Application.Features.Alerters.Queries;

[RequirePermission(ResourceType.Alert, PermissionLevel.Read)]
public record GetAlertRule(Guid Id) : IQuery<Result<AlertRule>>;

internal sealed class GetAlertRuleHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetAlertRule, Result<AlertRule>>
{
    public async ValueTask<Result<AlertRule>> Handle(GetAlertRule query, CancellationToken cancellationToken)
    {
        var rule = await unitOfWork.AlertRules.GetByIdAsync(query.Id, cancellationToken);
        if (rule is null)
        {
            return Result.Failure<AlertRule>(new NotFoundError("Alert rule does not exist"));
        }

        return rule;
    }
}
