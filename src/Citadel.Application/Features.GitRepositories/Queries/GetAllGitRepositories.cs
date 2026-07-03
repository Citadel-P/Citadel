using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common;
using Hosting.Common.Abstraction;
using LightResults;
using Mediator;

namespace Application.Features.GitRepositories.Queries;

public sealed record GetAllGitRepositories(IReadOnlyCollection<Guid>? TagIds = null) : IQuery<Result<IEnumerable<GitRepository>>>;

internal sealed class GetAllGitRepositoriesHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor) : IQueryHandler<GetAllGitRepositories, Result<IEnumerable<GitRepository>>>
{
    public async ValueTask<Result<IEnumerable<GitRepository>>> Handle(GetAllGitRepositories query, CancellationToken cancellationToken)
    {
        var user = userContextAccessor.Current;
        var gitRepositories = user is not null && !user.IsAdmin
            ? await unitOfWork.GitRepositories.GetAuthorizedAsync(user.UserId, ResourceType.GitRepository, PermissionLevel.Read, SpecificPermission.None, cancellationToken, query.TagIds)
            : await unitOfWork.GitRepositories.GetAllAsync(cancellationToken, query.TagIds);

        return Result.Success(gitRepositories);
    }
}
