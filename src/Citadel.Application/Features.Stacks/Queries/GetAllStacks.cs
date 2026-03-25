using Domain.Contracts.Interfaces;
using Domain.Entities.Stacks;
using LightResults;
using Mediator;

namespace Application.Features.Stacks.Queries;

public sealed record GetAllStacks : IQuery<Result<IEnumerable<Stack>>>;

internal sealed class GetAllStacksHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetAllStacks, Result<IEnumerable<Stack>>>
{
    public async ValueTask<Result<IEnumerable<Stack>>> Handle(GetAllStacks query, CancellationToken cancellationToken)
    {
        var stacks = await unitOfWork.Stacks.GetInfoAsync(cancellationToken) ?? [];
        return Result.Success(stacks);
    }
}