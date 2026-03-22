using Application.Features.GitRepositories.Commands;
using Domain;

namespace WebApi.Routes.Endpoints.Resources.GitRepositories;

public sealed record GitRepositoryInput(
    string Name,
    string? Description,
    string Url,
    string DefaultBranch,
    GitReposStatus Status,
    Guid? GitAccountId)
{
    internal CreateGitRepository ToCommand() => new(Name, Description, Url, DefaultBranch, Status, GitAccountId);
}
