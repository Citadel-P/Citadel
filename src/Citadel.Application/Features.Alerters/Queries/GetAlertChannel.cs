using Hosting.Common;
using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Queries;

[RequirePermission(ResourceType.AlertChannel, ResourceAction.View)]
public record GetAlertChannel(Guid Id) : IQuery<Result<AlertChannel>>;

internal sealed class GetAlertChannelHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetAlertChannel, Result<AlertChannel>>
{
    public async ValueTask<Result<AlertChannel>> Handle(GetAlertChannel query, CancellationToken cancellationToken)
    {
        var channel = await unitOfWork.AlertRules.GetChannelByIdAsync(query.Id, cancellationToken);
        return channel ?? Result.Failure<AlertChannel>(new NotFoundError("Alert channel does not exist"));
    }
}
