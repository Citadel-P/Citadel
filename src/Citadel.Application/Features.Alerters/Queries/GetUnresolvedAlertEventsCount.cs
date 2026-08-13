using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Abstraction;
using LightResults;
using Mediator;

namespace Application.Features.Alerters.Queries;

public sealed record GetUnresolvedAlertEventsCount : IQuery<Result<int>>;

internal sealed class GetUnresolvedAlertEventsCountHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor) : IQueryHandler<GetUnresolvedAlertEventsCount, Result<int>>
{
    public async ValueTask<Result<int>> Handle(GetUnresolvedAlertEventsCount query, CancellationToken cancellationToken)
    {
        var user = userContextAccessor.Current;
        var count = user is not null && !user.IsAdmin
            ? await unitOfWork.AlertEvents.CountAuthorizedUnresolvedAsync(
                user.ActorId,
                ResourceType.Alert,
                PermissionLevel.Read,
                SpecificPermission.None,
                cancellationToken)
            : await unitOfWork.AlertEvents.CountUnresolvedAsync(cancellationToken);

        return Result.Success(count);
    }
}
