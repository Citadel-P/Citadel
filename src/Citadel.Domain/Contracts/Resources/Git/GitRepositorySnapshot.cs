using Domain.Entities.Git;
using Hosting.Common;

namespace Domain.Contracts.Resources.Git;

public sealed record GitRepositorySnapshot(
    Guid Id,
    string Name,
    string? Description,
    string Url,
    string DefaultBranch,
    Guid? GitAccountId,
    bool WebHookEnabled,
    string? WebHookSecret,
    RepoCommand? OnClone,
    RepoCommand? OnPull);

public static class GitRepositorySnapshotExtensions
{
    public static GitRepositorySnapshot ToSnapshot(this GitRepository repository, Guid? id = null) 
        => new (
            Id: id ?? repository.Id,
            repository.Name,
            repository.Description,
            repository.Url,
            repository.DefaultBranch ?? string.Empty,
            repository.GitAccountId,
            repository.WebHookEnabled,
            repository.WebHookSecret.MaskValue(),
            repository.OnClone,
            repository.OnPull);
}

public sealed record RepoSyncResultSnapshot(string? CommitSha = null, string? Message = null);