using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using LightResults;
using Mediator;

namespace Application.Features.GitAccounts.Queries;

public sealed record GetAllGitAccounts() : IQuery<Result<IEnumerable<GitAccount>>>;

internal sealed class GetAllGitAccountsHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetAllGitAccounts, Result<IEnumerable<GitAccount>>>
{
    public async ValueTask<Result<IEnumerable<GitAccount>>> Handle(GetAllGitAccounts query, CancellationToken cancellationToken)
    {
        var gitAccounts = await unitOfWork.GitAccounts.GetAllAsync(cancellationToken) ?? [];
        IEnumerable<GitAccount> orderedGitAccounts = gitAccounts.OrderByDescending(x => x.CreatedAt);
        return Result.Success(orderedGitAccounts);
    }
}
