using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Hosting.Common;
using Hosting.Common.Abstraction;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Queries;

public sealed record GetAlertRules : IQuery<Result<(IEnumerable<AlertRule> Rules, IEnumerable<AlertChannel> Channels)>>;

internal sealed class GetAlertRulesHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor) : IQueryHandler<GetAlertRules, Result<(IEnumerable<AlertRule> Rules, IEnumerable<AlertChannel> Channels)>>
{
    public async ValueTask<Result<(IEnumerable<AlertRule> Rules, IEnumerable<AlertChannel> Channels)>> Handle(GetAlertRules query, CancellationToken cancellationToken)
    {
        var user = userContextAccessor.Current;
        var rules = user is not null && !user.IsAdmin
            ? await unitOfWork.AlertRules.GetAuthorizedAsync(user.UserId, ResourceType.Alert, PermissionLevel.Read, SpecificPermission.None, cancellationToken)
            : await unitOfWork.AlertRules.GetAllAsync(cancellationToken);

        var channels = user is not null && !user.IsAdmin
            ? await unitOfWork.AlertRules.GetAuthorizedChannelsAsync(user.UserId, ResourceType.AlertChannel, PermissionLevel.Read, SpecificPermission.None, cancellationToken)
            : await unitOfWork.AlertRules.GetAllChannelsAsync(cancellationToken);

        return Result.Success((rules, channels));
    }
}
