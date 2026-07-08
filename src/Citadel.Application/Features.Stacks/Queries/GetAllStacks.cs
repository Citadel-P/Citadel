using Application.Features.Tags.Queries;
using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Abstraction;
using LightResults;
using Mediator;

namespace Application.Features.Stacks.Queries;

public sealed record GetAllStacks(IReadOnlyCollection<string>? Tags = null) : IQuery<Result<IEnumerable<Stack>>>;

internal sealed class GetAllStacksHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor) : IQueryHandler<GetAllStacks, Result<IEnumerable<Stack>>>
{
    public async ValueTask<Result<IEnumerable<Stack>>> Handle(GetAllStacks query, CancellationToken cancellationToken)
    {
        var tagFilter = await TagFilterResolver.ResolveAsync(unitOfWork, query.Tags, cancellationToken);
        if (tagFilter.NoMatch)
            return Result.Success<IEnumerable<Stack>>([]);

        var user = userContextAccessor.Current;
        var stacks = user is not null && !user.IsAdmin
            ? await unitOfWork.Stacks.GetAuthorizedInfoAsync(user.UserId, ResourceType.Stack, PermissionLevel.Read, SpecificPermission.None, cancellationToken, tagFilter.TagIds)
            : await unitOfWork.Stacks.GetInfoAsync(cancellationToken, tagFilter.TagIds);

        return Result.Success(stacks);
    }
}
