using Domain;
using Domain.Entities.Git;

namespace WebApi.Routes.Endpoints.Resources.GitRepositories;

public sealed record GitRepositoryRefsView(IReadOnlyList<GitRepositoryRefView> Refs)
{
    internal static GitRepositoryRefsView Map(IReadOnlyList<GitRepositoryRef> refs)
        => new([.. refs.Select(GitRepositoryRefView.Map)]);
}

public sealed record GitRepositoryRefView(
    Guid Id,
    Guid GitRepositoryId,
    string Branch,
    string? ResolvedCommitSha,
    GitReposStatus Status,
    string? LastError,
    DateTime LastSyncedAt)
{
    internal static GitRepositoryRefView Map(GitRepositoryRef gitRepositoryRef)
        => new(
            gitRepositoryRef.Id,
            gitRepositoryRef.GitRepositoryId,
            gitRepositoryRef.Branch,
            gitRepositoryRef.ResolvedCommitSha,
            gitRepositoryRef.Status,
            gitRepositoryRef.LastError,
            gitRepositoryRef.LastSyncedAt);
}
