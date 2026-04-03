using Domain.Entities.Git;

namespace WebApi.Routes.Endpoints.Resources.GitRepositories;

public sealed record PatchGitRepositoryInput(
    string Url,
    string DefaultBranch,
    Guid? GitAccountId,
    bool WebHookEnabled,
    string? WebHookSecret,
    RepoCommand? OnClone,
    RepoCommand? OnPull);
