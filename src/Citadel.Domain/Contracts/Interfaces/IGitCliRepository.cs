using Domain.Entities.Git;
using LightResults;

namespace Domain.Contracts.Interfaces;

///<summary>
/// This class is intended to be used as a building block for higher-level features such as:
/// - Repository caching and synchronization
/// - Git-based deployments (GitOps)
/// - CI/CD pipelines
/// Key responsibilities:
/// - Clone repositories as bare repositories (no working directory)
/// - Perform shallow fetches of specific branches into isolated internal refs (refs/citadel/*)
/// - Resolve commit hashes deterministically from those refs
/// - Materialize a specific commit into a filesystem directory (snapshot extraction)
/// - Configure authentication for HTTP (token) and SSH (private key) access
/// - Ensure all Git commands are non-interactive and safe for server environments
/// </summary>
public interface IGitCliRepository
{
    Task<Result> CloneBareAsync(string url, string targetPath, GitAccount? account, CancellationToken ct = default);
    Task<Result> FetchBranchSnapshotAsync(string repoPath, string branch, GitAccount? account, CancellationToken ct = default);
    Task<Result<string>> ResolveSnapshotCommitAsync(string repoPath, string branch, CancellationToken ct = default);
    Task<Result> MaterializeAsync(string repoPath, string commitHash, string targetPath, CancellationToken ct = default);
}
