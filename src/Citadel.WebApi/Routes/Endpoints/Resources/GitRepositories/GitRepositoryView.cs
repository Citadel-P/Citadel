using Domain;
using Domain.Entities.Git;
using WebApi.Routes.Endpoints.Resources.Activities;

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
    RepoCommand? OnClone,
    RepoCommand? OnPull,
    DateTime CreatedAt,
    ResourceControlState ControlState,
    LatestActivityView? LatestActivityView)
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
        gitRepository.CreatedAt,
        gitRepository.ControlState,
        gitRepository.LatestActivityEvent?.Map());
}


public sealed record GitRepositoryConfigView(
    Guid Id,
    string Name,
    string? Description,
    string Url,
    string DefaultBranch,
    Guid? GitAccountId,
    bool WebHookEnabled,
    string? WebHookSecret,
    RepoCommand? OnClone,
    RepoCommand? OnPull)
{
    internal static GitRepositoryConfigView Map(GitRepository gitRepository) => new(
        gitRepository.Id,
        gitRepository.Name,
        gitRepository.Description,
        gitRepository.Url,
        gitRepository.DefaultBranch ?? "main",
        gitRepository.GitAccountId,
        gitRepository.WebHookEnabled,
        gitRepository.WebHookSecret,
        gitRepository.OnClone,
        gitRepository.OnPull);
}