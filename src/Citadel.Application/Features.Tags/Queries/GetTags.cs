using Domain.Contracts.Interfaces;
using Domain.Entities.Tags;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.Tags.Queries;

//[RequirePermission(ResourceType.Tag, PermissionLevel.Read)]
public sealed record GetTags : IQuery<Result<IReadOnlyList<TagWithUsage>>>;

internal sealed class GetTagsHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetTags, Result<IReadOnlyList<TagWithUsage>>>
{
    public async ValueTask<Result<IReadOnlyList<TagWithUsage>>> Handle(GetTags query, CancellationToken cancellationToken)
        => Result.Success(await unitOfWork.Tags.ListAsync(cancellationToken));
}
