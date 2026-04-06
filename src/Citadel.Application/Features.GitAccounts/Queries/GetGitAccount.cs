using Hosting.Common;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.GitAccounts.Queries;

[RequirePermission(ResourceType.GitAccount, ResourceAction.View)]
public sealed record GetGitAccount(Guid Id) : IQuery<Result<GitAccount>>;

internal sealed class GetGitAccountHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetGitAccount, Result<GitAccount>>
{
    public async ValueTask<Result<GitAccount>> Handle(GetGitAccount query, CancellationToken cancellationToken)
    {
        var gitAccount = await unitOfWork.GitAccounts.GetAsync(query.Id, cancellationToken);
        return gitAccount ?? Result.Failure<GitAccount>(new NotFoundError($"Git account with ID {query.Id} does not exist"));
    }
}
