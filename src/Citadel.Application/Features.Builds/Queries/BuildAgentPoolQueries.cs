using Application.Features.Builds.Models;
using Application.Features.Tags.Queries;
using Domain;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Abstraction;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Builds.Queries;

[RequirePermission(ResourceType.BuildAgentPool, PermissionLevel.Read)]
public sealed record GetBuildAgentPools(IReadOnlyCollection<string>? Tags = null) : IQuery<Result<BuildAgentPoolListResult>>;

[RequirePermission(ResourceType.BuildAgentPool, PermissionLevel.Read)]
public sealed record GetBuildAgentPool(Guid PoolId) : IQuery<Result<BuildAgentPoolResult>>;

internal sealed class GetBuildAgentPoolsHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor)
    : IQueryHandler<GetBuildAgentPools, Result<BuildAgentPoolListResult>>
{
    public async ValueTask<Result<BuildAgentPoolListResult>> Handle(GetBuildAgentPools query, CancellationToken cancellationToken)
    {
        var tagFilter = await TagFilterResolver.ResolveAsync(unitOfWork, query.Tags, cancellationToken);
        if (tagFilter.NoMatch)
            return Result.Success(new BuildAgentPoolListResult([]));

        var user = userContextAccessor.Current;
        var pools = user is not null && !user.IsAdmin
            ? await unitOfWork.BuildAgentPools.GetAuthorizedAsync(
                user.UserId,
                ResourceType.BuildAgentPool,
                PermissionLevel.Read,
                SpecificPermission.None,
                cancellationToken,
                tagFilter.TagIds)
            : await unitOfWork.BuildAgentPools.GetAllAsync(cancellationToken, tagFilter.TagIds);

        return Result.Success(new BuildAgentPoolListResult([.. pools.OrderByDescending(static x => x.CreatedAt)]));
    }
}

internal sealed class GetBuildAgentPoolHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetBuildAgentPool, Result<BuildAgentPoolResult>>
{
    public async ValueTask<Result<BuildAgentPoolResult>> Handle(GetBuildAgentPool query, CancellationToken cancellationToken)
    {
        var pool = await unitOfWork.BuildAgentPools.GetAsync(query.PoolId, cancellationToken);
        return pool is null
            ? Result.Failure<BuildAgentPoolResult>(new NotFoundError("Build pool not found."))
            : Result.Success(new BuildAgentPoolResult(pool));
    }
}
