using Hosting.Common;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.GitRepositories.Queries;

[RequirePermission(ResourceType.GitRepository, PermissionLevel.Read)]
public sealed record GetGitRepository(Guid Id) : IQuery<Result<GitRepository>>;

internal sealed class GetGitRepositoryHandler(IUnitOfWork unitOfWork) : IQueryHandler<GetGitRepository, Result<GitRepository>>
{
    public async ValueTask<Result<GitRepository>> Handle(GetGitRepository query, CancellationToken cancellationToken)
    {
        var gitRepository = await unitOfWork.GitRepositories.GetAsync(query.Id, cancellationToken);
        return gitRepository ?? Result.Failure<GitRepository>(new NotFoundError($"Git repository with ID {query.Id} does not exist"));
    }
}
