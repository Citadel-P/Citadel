using Domain;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.Common.Extensions;
using LightResults;
using Mediator;
using Hosting.Common;
using Microsoft.AspNetCore.Http;

namespace Application.Features.GitRepositories.Queries;

public sealed record GetAllGitRepositories() : IQuery<Result<IEnumerable<GitRepository>>>;

internal sealed class GetAllGitRepositoriesHandler(IUnitOfWork unitOfWork, IHttpContextAccessor httpContextAccessor) : IQueryHandler<GetAllGitRepositories, Result<IEnumerable<GitRepository>>>
{
    public async ValueTask<Result<IEnumerable<GitRepository>>> Handle(GetAllGitRepositories query, CancellationToken cancellationToken)
    {
        var user = httpContextAccessor.HttpContext?.User;
        var gitRepositories = user is not null && !user.IsAdmin()
            ? await unitOfWork.GitRepositories.GetAuthorizedAsync(user.GetUserId(), ResourceType.GitRepository, PermissionLevel.Read, SpecificPermission.None, cancellationToken)
            : await unitOfWork.GitRepositories.GetAllAsync(cancellationToken);

        IEnumerable<GitRepository> orderedGitRepositories = gitRepositories.OrderByDescending(x => x.CreatedAt);
        return Result.Success(orderedGitRepositories);
    }
}
