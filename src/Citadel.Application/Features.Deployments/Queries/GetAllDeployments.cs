using Application.Features.Tags.Queries;
using Domain.Contracts.Interfaces;
using Domain.Entities.Deployments;
using Hosting.Common;
using Hosting.Common.Abstraction;
using LightResults;
using Mediator;

namespace Application.Features.Deployments.Queries;

public sealed record GetAllDeployments(IReadOnlyCollection<string>? Tags = null, Guid? PlatformId = null) : IQuery<Result<IEnumerable<Deployment>>>;

internal sealed class GetAllDeploymentsHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor) : IQueryHandler<GetAllDeployments, Result<IEnumerable<Deployment>>>
{
    public async ValueTask<Result<IEnumerable<Deployment>>> Handle(GetAllDeployments query, CancellationToken cancellationToken)
    {
        var tagFilter = await TagFilterResolver.ResolveAsync(unitOfWork, query.Tags, cancellationToken);
        if (tagFilter.NoMatch)
            return Result.Success<IEnumerable<Deployment>>([]);

        var user = userContextAccessor.Current;
        var deployments = user is not null && !user.IsAdmin
            ? await unitOfWork.Deployments.GetAuthorizedInfoAsync(user.UserId, ResourceType.Deployment, PermissionLevel.Read, SpecificPermission.None, cancellationToken, tagFilter.TagIds, query.PlatformId)
            : await unitOfWork.Deployments.GetInfoAsync(cancellationToken, tagFilter.TagIds, query.PlatformId);

        return Result.Success(deployments);
    }
}
