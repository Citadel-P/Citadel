using Domain;
using Domain.Entities.Git;

namespace WebApi.Routes.Endpoints.Resources.GitRepositories;

public sealed record PatchGitRepositoryInput(
    string Url,
    string DefaultBranch,
    Guid? GitAccountId,
    GitRepositorySyncMode SyncMode,
    int? SyncIntervalMinutes,
    bool WebHookEnabled,
    string? WebHookSecret,
    RepoCommand? OnClone,
    RepoCommand? OnPull);
