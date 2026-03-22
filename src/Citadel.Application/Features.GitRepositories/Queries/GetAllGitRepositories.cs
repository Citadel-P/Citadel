using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using LightResults;
using Mediator;

namespace Application.Features.GitRepositories.Queries;

public sealed record GetAllGitRepositories() : IQuery<Result<IEnumerable<GitRepository>>>;

internal sealed class GetAllGitRepositoriesHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetAllGitRepositories, Result<IEnumerable<GitRepository>>>
{
    public async ValueTask<Result<IEnumerable<GitRepository>>> Handle(GetAllGitRepositories query, CancellationToken cancellationToken)
    {
        var gitRepositories = await unitOfWork.GitRepositories.GetAllAsync(cancellationToken) ?? [];
        IEnumerable<GitRepository> orderedGitRepositories = gitRepositories.OrderByDescending(x => x.CreatedAt);
        return Result.Success(orderedGitRepositories);
    }
}
