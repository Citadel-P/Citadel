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
    GitRepositorySyncMode SyncMode,
    int? SyncIntervalMinutes,
    RepoWebhookConfig? Webhook,
    RepoCommand? OnClone,
    RepoCommand? OnPull,
    string? ResolvedCommitSha = null);

public static class GitRepositorySnapshotExtensions
{
    public static GitRepositorySnapshot ToSnapshot(
        this GitRepository repository,
        Guid? id = null,
        string? resolvedCommitSha = null)
        => new (
            Id: id ?? repository.Id,
            repository.Name,
            repository.Description,
            repository.Url,
            repository.DefaultBranch ?? string.Empty,
            repository.GitAccountId,
            repository.SyncMode,
            repository.SyncIntervalMinutes,
            repository.Webhook is null ? null : repository.Webhook with { Secret = repository.Webhook.Secret.MaskValue() },
            repository.OnClone,
            repository.OnPull,
            resolvedCommitSha);
}

public sealed record RepoSyncResultSnapshot(string? CommitSha = null, string? Message = null);
