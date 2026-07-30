using Domain.Contracts.Interfaces;
using Domain.Entities.Tags;
using Hosting.Common;
using Hosting.Common.Abstraction;
using LightResults;
using Mediator;

namespace Application.Features.Tags.Queries;

public sealed record GetTags : IQuery<Result<IReadOnlyList<TagWithUsage>>>;

internal sealed class GetTagsHandler(
    IUnitOfWork unitOfWork,
    IUserContextAccessor userContextAccessor) : IQueryHandler<GetTags, Result<IReadOnlyList<TagWithUsage>>>
{
    public async ValueTask<Result<IReadOnlyList<TagWithUsage>>> Handle(GetTags query, CancellationToken cancellationToken)
    {
        var user = userContextAccessor.Current;
        var tags = user is not null && !user.IsAdmin
            ? await unitOfWork.Tags.ListAuthorizedAsync(
                user.UserId,
                ResourceType.Tag,
                PermissionLevel.Read,
                SpecificPermission.None,
                cancellationToken)
            : await unitOfWork.Tags.ListAsync(cancellationToken);

        return Result.Success(tags);
    }
}
