using Hosting.Common;
using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.Stacks.Queries;

[RequirePermission(ResourceType.Stack, PermissionLevel.Read)]
public sealed record GetStack(Guid Id) : IQuery<Result<Stack>>;

internal sealed class GetStackHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetStack, Result<Stack>>
{
    public async ValueTask<Result<Stack>> Handle(GetStack query, CancellationToken cancellationToken)
    {
        var stack = await unitOfWork.Stacks.GetAsync(query.Id, cancellationToken);
        return stack ?? Result.Failure<Stack>(new NotFoundError($"Stack with ID {query.Id} does not exist"));
    }
}