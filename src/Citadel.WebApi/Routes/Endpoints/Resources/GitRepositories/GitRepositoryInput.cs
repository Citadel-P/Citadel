using Application.Features.GitRepositories.Commands;
using Domain;
using Domain.Entities.Git;

namespace WebApi.Routes.Endpoints.Resources.GitRepositories;

public sealed record GitRepositoryInput(
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
    internal CreateGitRepository ToCommand() => new(Name, Description, Url, DefaultBranch, GitAccountId, WebHookEnabled, WebHookSecret, OnClone, OnPull);

}
