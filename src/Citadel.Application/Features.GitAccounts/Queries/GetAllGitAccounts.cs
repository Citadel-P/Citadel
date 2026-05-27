using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common;
using Hosting.Common.Abstraction;
using LightResults;
using Mediator;

namespace Application.Features.GitAccounts.Queries;

public sealed record GetAllGitAccounts() : IQuery<Result<IEnumerable<GitAccount>>>;

internal sealed class GetAllGitAccountsHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor) : IQueryHandler<GetAllGitAccounts, Result<IEnumerable<GitAccount>>>
{
    public async ValueTask<Result<IEnumerable<GitAccount>>> Handle(GetAllGitAccounts query, CancellationToken cancellationToken)
    {
        var user = userContextAccessor.Current;
        var gitAccounts = user is not null && !user.IsAdmin
            ? await unitOfWork.GitAccounts.GetAuthorizedAsync(user.UserId, ResourceType.GitAccount, PermissionLevel.Read, SpecificPermission.None, cancellationToken)
            : await unitOfWork.GitAccounts.GetAllAsync(cancellationToken);

        IEnumerable<GitAccount> orderedGitAccounts = gitAccounts.OrderByDescending(x => x.CreatedAt);
        return Result.Success(orderedGitAccounts);
    }
}
