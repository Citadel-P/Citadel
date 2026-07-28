using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common;
using Hosting.Common.Attributes;
using LightResults;
using Mediator;

namespace Application.Features.GitRepositories.Queries;

[RequirePermission(ResourceType.GitRepository, PermissionLevel.Read)]
public sealed record GetGitRepositoryRefs(Guid Id) : IQuery<Result<IReadOnlyList<GitRepositoryRef>>>;

internal sealed class GetGitRepositoryRefsHandler(IUnitOfWork unitOfWork)
    : IQueryHandler<GetGitRepositoryRefs, Result<IReadOnlyList<GitRepositoryRef>>>
{
    public async ValueTask<Result<IReadOnlyList<GitRepositoryRef>>> Handle(
        GetGitRepositoryRefs query,
        CancellationToken cancellationToken)
    {
        var repo = await unitOfWork.GitRepositories.GetAsync(query.Id, cancellationToken);
        if (repo is null)
        {
            return Result.Failure<IReadOnlyList<GitRepositoryRef>>(
                $"Git repository with ID {query.Id} does not exist");
        }

        var refs = await unitOfWork.GitRepositories.GetRefsByRepositoryIdAsync(query.Id, cancellationToken);
        return Result.Success<IReadOnlyList<GitRepositoryRef>>([.. refs]);
    }
}
