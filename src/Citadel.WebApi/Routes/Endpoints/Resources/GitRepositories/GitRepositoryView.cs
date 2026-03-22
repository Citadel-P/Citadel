using Domain;
using Domain.Entities.Git;

namespace WebApi.Routes.Endpoints.Resources.GitRepositories;

public sealed record GitRepositoryView(
    Guid Id,
    Guid CreatedByActorId,
    string Name,
    string? Description,
    GitReposStatus Status,
    string Url,
    string? DefaultBranch,
    Guid? GitAccountId,
    DateTime CreatedAt)
{
    internal static GitRepositoryView Map(GitRepository gitRepository) => new(
        gitRepository.Id,
        gitRepository.CreatedByActorId,
        gitRepository.Name,
        gitRepository.Description,
        gitRepository.Status,
        gitRepository.Url,
        gitRepository.DefaultBranch,
        gitRepository.GitAccountId,
        gitRepository.CreatedAt);
}
