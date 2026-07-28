using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.GitRepositories.Queries;

[RequirePermission(ResourceType.GitRepository, PermissionLevel.Read, ResourceIdProperty = nameof(DiscoverGitRepositoryBranches.RepositoryId))]
public sealed record DiscoverGitRepositoryBranches(Guid RepositoryId)
    : IQuery<Result<IReadOnlyList<GitRemoteBranchRef>>>;

internal sealed class DiscoverGitRepositoryBranchesHandler(
    IUnitOfWork unitOfWork,
    IRepoCacheManager repoCacheManager,
    IGitCliRepository gitCliRepository)
    : IQueryHandler<DiscoverGitRepositoryBranches, Result<IReadOnlyList<GitRemoteBranchRef>>>
{
    public async ValueTask<Result<IReadOnlyList<GitRemoteBranchRef>>> Handle(
        DiscoverGitRepositoryBranches query,
        CancellationToken cancellationToken)
    {
        var repo = await unitOfWork.GitRepositories.GetWithAccountAsync(query.RepositoryId, cancellationToken);
        if (repo is null)
        {
            return Result.Failure<IReadOnlyList<GitRemoteBranchRef>>(
                new NotFoundError($"Git repository with ID {query.RepositoryId} does not exist"));
        }

        string remoteUrl;
        try
        {
            remoteUrl = repoCacheManager.GetRemoteUrl(repo, repo.GitAccount);
        }
        catch (InvalidOperationException ex)
        {
            return Result.Failure<IReadOnlyList<GitRemoteBranchRef>>(ex.Message);
        }

        return await gitCliRepository.ListRemoteBranchesAsync(remoteUrl, repo.GitAccount, cancellationToken);
    }
}
