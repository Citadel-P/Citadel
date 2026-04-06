using Hosting.Common;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Microsoft.AspNetCore.Http;

namespace Application.Features.GitAccounts.Queries;

public sealed record GetAllGitAccounts() : IQuery<Result<IEnumerable<GitAccount>>>;

internal sealed class GetAllGitAccountsHandler(IUnitOfWork unitOfWork, IHttpContextAccessor httpContextAccessor) : IQueryHandler<GetAllGitAccounts, Result<IEnumerable<GitAccount>>>
{
    public async ValueTask<Result<IEnumerable<GitAccount>>> Handle(GetAllGitAccounts query, CancellationToken cancellationToken)
    {
        var user = httpContextAccessor.HttpContext?.User;
        var gitAccounts = user is not null && !user.IsAdmin()
            ? await unitOfWork.GitAccounts.GetAuthorizedAsync(user.GetUserId(), ResourceType.GitAccount, ResourceAction.View, cancellationToken)
            : await unitOfWork.GitAccounts.GetAllAsync(cancellationToken);

        IEnumerable<GitAccount> orderedGitAccounts = gitAccounts.OrderByDescending(x => x.CreatedAt);
        return Result.Success(orderedGitAccounts);
    }
}
