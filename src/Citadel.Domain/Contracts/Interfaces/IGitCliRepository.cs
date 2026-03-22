using Domain.Entities.Git;
using LightResults;

namespace Domain.Contracts.Interfaces;

public interface IGitCliRepository
{
    Task<Result> CloneBareAsync(string url, string targetPath, GitAccount? account, CancellationToken ct = default);
    Task<Result> FetchAsync(string repoPath, string branch, GitAccount? account, CancellationToken ct = default);
    Task<Result<string>> ResolveCommitHashAsync(string repoPath, string branch, CancellationToken ct = default);
    Task<Result> MaterializeAsync(string repoPath, string commitHash, string targetPath, CancellationToken ct = default);
}
