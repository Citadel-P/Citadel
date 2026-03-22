using Application.Features.GitRepositories.Commands;

namespace WebApi.Routes.Endpoints.Resources.GitRepositories;

public sealed record GitRepositoryInput(
    string Name,
    string Url,
    string DefaultBranch,
    Guid? GitAccountId)
{
    internal CreateGitRepository ToCommand() => new(Name, Url, DefaultBranch, GitAccountId);
}
