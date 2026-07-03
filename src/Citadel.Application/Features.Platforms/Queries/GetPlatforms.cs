using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using Hosting.Common;
using Hosting.Common.Abstraction;
using LightResults;
using Mediator;

namespace Application.Features.Platforms.Queries;

public sealed record GetPlatforms(IReadOnlyCollection<Guid>? TagIds = null) : IQuery<Result<IEnumerable<Platform>>>;

internal class GetPlatformsHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor) : IQueryHandler<GetPlatforms, Result<IEnumerable<Platform>>>
{
    public async ValueTask<Result<IEnumerable<Platform>>> Handle(GetPlatforms request, CancellationToken cancellationToken)
    {
        var user = userContextAccessor.Current;
        var platforms = user is not null && !user.IsAdmin
            ? await unitOfWork.Platforms.GetAuthorizedWithLatestStatAsync(user.UserId, ResourceType.Platform, PermissionLevel.Read, SpecificPermission.None, cancellationToken, request.TagIds)
            : await unitOfWork.Platforms.GetPlatformsWithLatestStatAsync(cancellationToken, request.TagIds) ?? [];

        return Result.Success(platforms ?? []);
    }
}
