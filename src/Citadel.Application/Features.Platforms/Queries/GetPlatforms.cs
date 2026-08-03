using Domain;
using Application.Features.Tags.Queries;
using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using Hosting.Common;
using Hosting.Common.Abstraction;
using LightResults;
using Mediator;

namespace Application.Features.Platforms.Queries;

public sealed record GetPlatforms(IReadOnlyCollection<string>? Tags = null) : IQuery<Result<IEnumerable<Platform>>>;

internal class GetPlatformsHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor) : IQueryHandler<GetPlatforms, Result<IEnumerable<Platform>>>
{
    public async ValueTask<Result<IEnumerable<Platform>>> Handle(GetPlatforms request, CancellationToken cancellationToken)
    {
        var tagFilter = await TagFilterResolver.ResolveAsync(unitOfWork, request.Tags, cancellationToken);
        if (tagFilter.NoMatch)
            return Result.Success<IEnumerable<Platform>>([]);

        var user = userContextAccessor.Current;
        var platforms = user is not null && !user.IsAdmin
            ? await unitOfWork.Platforms.GetAuthorizedWithLatestStatAsync(user.UserId, ResourceType.Platform, PermissionLevel.Read, SpecificPermission.None, cancellationToken, tagFilter.TagIds)
            : await unitOfWork.Platforms.GetPlatformsWithLatestStatAsync(cancellationToken, tagFilter.TagIds) ?? [];

        return Result.Success(platforms ?? []);
    }
}
