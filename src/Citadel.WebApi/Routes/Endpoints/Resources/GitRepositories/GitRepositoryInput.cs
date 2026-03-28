using Application.Features.GitRepositories.Commands;
using Domain;
using Domain.Entities.Git;

namespace WebApi.Routes.Endpoints.Resources.GitRepositories;

public sealed record GitRepositoryInput(
    string Name,
    string? Description,
    string Url,
    string DefaultBranch,
    GitReposStatus Status,
    Guid? GitAccountId,
    bool WebHookEnabled,
    string? WebHookSecret,
    IEnumerable<RepoCommand>? OnClone,
    IEnumerable<RepoCommand>? OnPull)
{
    internal CreateGitRepository ToCommand() => new(Name, Description, Url, DefaultBranch, Status, GitAccountId, WebHookEnabled, WebHookSecret, OnClone, OnPull);

}
