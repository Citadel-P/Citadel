using Hosting.Common;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;

namespace Application.Features.Alerters.Queries;

public sealed record GetAlertRules : IQuery<Result<(IEnumerable<AlertRule> Rules, IEnumerable<AlertChannel> Channels)>>;

internal sealed class GetAlertRulesHandler(IUnitOfWork unitOfWork, IHttpContextAccessor httpContextAccessor) : IQueryHandler<GetAlertRules, Result<(IEnumerable<AlertRule> Rules, IEnumerable<AlertChannel> Channels)>>
{
    public async ValueTask<Result<(IEnumerable<AlertRule> Rules, IEnumerable<AlertChannel> Channels)>> Handle(GetAlertRules query, CancellationToken cancellationToken)
    {
        var user = httpContextAccessor.HttpContext?.User;
        var rules = user is not null && !user.IsAdmin()
            ? await unitOfWork.AlertRules.GetAuthorizedAsync(user.GetUserId(), ResourceType.Alert, ResourceAction.View, cancellationToken)
            : await unitOfWork.AlertRules.GetAllAsync(cancellationToken);

        var channels = user is not null && !user.IsAdmin()
            ? await unitOfWork.AlertRules.GetAuthorizedChannelsAsync(user.GetUserId(), ResourceType.AlertChannel, ResourceAction.View, cancellationToken)
            : await unitOfWork.AlertRules.GetAllChannelsAsync(cancellationToken);

        return Result.Success((rules, channels));
    }
}
