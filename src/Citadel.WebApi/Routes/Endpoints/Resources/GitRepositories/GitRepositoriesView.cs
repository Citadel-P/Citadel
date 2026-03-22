using Domain.Entities.Git;

namespace WebApi.Routes.Endpoints.Resources.GitRepositories;

public sealed record GitRepositoriesView(IEnumerable<GitRepositoryView> GitRepositories)
{
    internal static GitRepositoriesView Map(IEnumerable<GitRepository> gitRepositories) => new([.. gitRepositories.Select(GitRepositoryView.Map)]);
}
