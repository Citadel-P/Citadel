using Domain.Entities.Git;

namespace Application.Services;

internal static class ApplicationStoragePaths
{
    public static string StacksRoot =>
        Path.GetFullPath(Path.Combine("data", "stacks"));

    public static string GetRepositoryCachePath(GitRepository repository) =>
        Path.Combine(
            Path.GetFullPath(Path.Combine("data", "repos")),
            repository.Id.ToString("D"));
}
