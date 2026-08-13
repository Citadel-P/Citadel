using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Hosting.Common;
using Hosting.Common.Abstraction;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Queries;

public record GetAlertChannels : IQuery<Result<IEnumerable<AlertChannel>>>;

internal sealed class GetAlertChannelsHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor) : IQueryHandler<GetAlertChannels, Result<IEnumerable<AlertChannel>>>
{
    public async ValueTask<Result<IEnumerable<AlertChannel>>> Handle(GetAlertChannels query, CancellationToken cancellationToken)
    {
        var user = userContextAccessor.Current;
        var channels = user is not null && !user.IsAdmin
            ? await unitOfWork.AlertRules.GetAuthorizedChannelsAsync(user.ActorId, ResourceType.AlertChannel, PermissionLevel.Read, SpecificPermission.None, cancellationToken)
            : await unitOfWork.AlertRules.GetAllChannelsAsync(cancellationToken);

        return Result.Success(channels);
    }
}
