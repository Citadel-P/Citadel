using Hosting.Common;
using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;

namespace Application.Features.Stacks.Queries;

public sealed record GetAllStacks : IQuery<Result<IEnumerable<Stack>>>;

internal sealed class GetAllStacksHandler(IUnitOfWork unitOfWork, IHttpContextAccessor httpContextAccessor) : IQueryHandler<GetAllStacks, Result<IEnumerable<Stack>>>
{
    public async ValueTask<Result<IEnumerable<Stack>>> Handle(GetAllStacks query, CancellationToken cancellationToken)
    {
        var user = httpContextAccessor.HttpContext?.User;
        var stacks = user is not null && !user.IsAdmin()
            ? await unitOfWork.Stacks.GetAuthorizedInfoAsync(user.GetUserId(), ResourceType.Stack, ResourceAction.View, cancellationToken)
            : await unitOfWork.Stacks.GetInfoAsync(cancellationToken);

        return Result.Success(stacks);
    }
}