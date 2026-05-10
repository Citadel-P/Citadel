using Domain.Contracts.Interfaces;
using Domain.Entities.Platforms;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Hosting.Common;
using Microsoft.AspNetCore.Http;

namespace Application.Features.Platforms.Queries;

public sealed record GetPlatforms() : IQuery<Result<IEnumerable<Platform>>>;

internal class GetPlatformsHandler(IUnitOfWork unitOfWork, IHttpContextAccessor httpContextAccessor) : IQueryHandler<GetPlatforms, Result<IEnumerable<Platform>>>
{
    public async ValueTask<Result<IEnumerable<Platform>>> Handle(GetPlatforms request, CancellationToken cancellationToken)
    {
        var user = httpContextAccessor.HttpContext?.User;
        var platforms = user is not null && !user.IsAdmin()
            ? await unitOfWork.Platforms.GetAuthorizedWithLatestStatAsync(user.GetUserId(), ResourceType.Platform, PermissionLevel.Read, SpecificPermission.None, cancellationToken)
            : await unitOfWork.Platforms.GetPlatformsWithLatestStatAsync(cancellationToken) ?? [];

        return Result.Success(platforms ?? []);
    }
}