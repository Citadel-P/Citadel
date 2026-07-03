using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using Hosting.Common;
using Hosting.Common.Abstraction;
using LightResults;
using Mediator;

namespace Application.Features.Stacks.Queries;

public sealed record GetAllStacks(IReadOnlyCollection<Guid>? TagIds = null) : IQuery<Result<IEnumerable<Stack>>>;

internal sealed class GetAllStacksHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor) : IQueryHandler<GetAllStacks, Result<IEnumerable<Stack>>>
{
    public async ValueTask<Result<IEnumerable<Stack>>> Handle(GetAllStacks query, CancellationToken cancellationToken)
    {
        var user = userContextAccessor.Current;
        var stacks = user is not null && !user.IsAdmin
            ? await unitOfWork.Stacks.GetAuthorizedInfoAsync(user.UserId, ResourceType.Stack, PermissionLevel.Read, SpecificPermission.None, cancellationToken, query.TagIds)
            : await unitOfWork.Stacks.GetInfoAsync(cancellationToken, query.TagIds);

        return Result.Success(stacks);
    }
}
