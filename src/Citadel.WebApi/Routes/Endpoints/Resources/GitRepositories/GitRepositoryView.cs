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
    bool WebHookEnabled,
    string? WebHookSecret,
    IEnumerable<RepoCommand> OnClone,
    IEnumerable<RepoCommand> OnPull,
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
        gitRepository.WebHookEnabled,
        gitRepository.WebHookSecret,
        gitRepository.OnClone,
        gitRepository.OnPull,
        gitRepository.CreatedAt);
}
