using Application.Features.Tags.Queries;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common;
using Hosting.Common.Abstraction;
using LightResults;
using Mediator;

namespace Application.Features.GitRepositories.Queries;

public sealed record GetAllGitRepositories(IReadOnlyCollection<string>? Tags = null) : IQuery<Result<IEnumerable<GitRepository>>>;

internal sealed class GetAllGitRepositoriesHandler(IUnitOfWork unitOfWork, IUserContextAccessor userContextAccessor) : IQueryHandler<GetAllGitRepositories, Result<IEnumerable<GitRepository>>>
{
    public async ValueTask<Result<IEnumerable<GitRepository>>> Handle(GetAllGitRepositories query, CancellationToken cancellationToken)
    {
        var tagFilter = await TagFilterResolver.ResolveAsync(unitOfWork, query.Tags, cancellationToken);
        if (tagFilter.NoMatch)
            return Result.Success<IEnumerable<GitRepository>>([]);

        var user = userContextAccessor.Current;
        var gitRepositories = user is not null && !user.IsAdmin
            ? await unitOfWork.GitRepositories.GetAuthorizedAsync(user.ActorId, ResourceType.GitRepository, PermissionLevel.Read, SpecificPermission.None, cancellationToken, tagFilter.TagIds)
            : await unitOfWork.GitRepositories.GetAllAsync(cancellationToken, tagFilter.TagIds);

        return Result.Success(gitRepositories);
    }
}
