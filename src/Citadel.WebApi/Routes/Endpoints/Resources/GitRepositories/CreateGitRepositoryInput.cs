using Application.Features.GitRepositories.Commands;
using Domain;
using Domain.Entities.Git;

namespace WebApi.Routes.Endpoints.Resources.GitRepositories;

public sealed record CreateGitRepositoryInput(
    string Name,
    string? Description,
    string Url,
    string DefaultBranch,
    Guid? GitAccountId,
    GitRepositorySyncMode SyncMode,
    int? SyncIntervalMinutes,
    RepoWebhookConfig? Webhook,
    RepoCommand? OnClone,
    RepoCommand? OnPull,
    IReadOnlyCollection<Guid>? TagIds = null)
{
    internal CreateGitRepository ToCommand()
        => new(Name, Description, Url, DefaultBranch, GitAccountId, Webhook, OnClone, OnPull, SyncMode, SyncIntervalMinutes, TagIds);
}
