using Domain.Contracts.Interfaces;
using Domain.Entities.Alerts;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Hosting.Common;
using Microsoft.AspNetCore.Http;

namespace Application.Features.Alerters.Queries;

public record GetAlertChannels : IQuery<Result<IEnumerable<AlertChannel>>>;

internal sealed class GetAlertChannelsHandler(IUnitOfWork unitOfWork, IHttpContextAccessor httpContextAccessor) : IQueryHandler<GetAlertChannels, Result<IEnumerable<AlertChannel>>>
{
    public async ValueTask<Result<IEnumerable<AlertChannel>>> Handle(GetAlertChannels query, CancellationToken cancellationToken)
    {
        var user = httpContextAccessor.HttpContext?.User;
        var channels = user is not null && !user.IsAdmin()
            ? await unitOfWork.AlertRules.GetAuthorizedChannelsAsync(user.GetUserId(), ResourceType.AlertChannel, ResourceAction.View, cancellationToken)
            : await unitOfWork.AlertRules.GetAllChannelsAsync(cancellationToken);

        return Result.Success(channels);
    }
}
