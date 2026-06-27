using Application.Services;
using Domain;
using Domain.Contracts.Interfaces;
using Hosting.Common;
using Hosting.Common.Attributes;
using Hosting.Common.ErrorTypes;
using LightResults;
using Mediator;

namespace Application.Features.GitRepositories.Queries;

[RequirePermission(ResourceType.GitRepository, PermissionLevel.Read)]
public sealed record DiscoverGitRepositoryComposeProjects(Guid RepositoryId, string? Branch)
    : IQuery<Result<GitRepositoryComposeDiscovery>>;

public sealed record GitRepositoryComposeDiscovery(
    Guid RepositoryId,
    string Branch,
    string ResolvedCommitSha,
    IReadOnlyList<GitComposeProjectCandidate> Projects);

public sealed record GitComposeProjectCandidate(
    string WorkingDirectory,
    IReadOnlyList<string> ComposePaths,
    IReadOnlyList<string> EnvFilePaths,
    IReadOnlyList<string> SuggestedWatchPaths);

internal sealed class DiscoverGitRepositoryComposeProjectsHandler(
    IUnitOfWork unitOfWork,
    IRepoCacheManager repoCacheManager,
    IGitCliRepository gitCliRepository)
    : IQueryHandler<DiscoverGitRepositoryComposeProjects, Result<GitRepositoryComposeDiscovery>>
{
    private static readonly HashSet<string> KnownComposeFileNames = new(StringComparer.OrdinalIgnoreCase)
    {
        "compose.yml",
        "compose.yaml",
        "docker-compose.yml",
        "docker-compose.yaml"
    };

    public async ValueTask<Result<GitRepositoryComposeDiscovery>> Handle(
        DiscoverGitRepositoryComposeProjects query,
        CancellationToken cancellationToken)
    {
        var repo = await unitOfWork.GitRepositories.GetWithAccountAsync(query.RepositoryId, cancellationToken);
        if (repo is null)
        {
            return Result.Failure<GitRepositoryComposeDiscovery>(
                new NotFoundError($"Git repository with ID {query.RepositoryId} does not exist"));
        }

        var branch = string.IsNullOrWhiteSpace(query.Branch)
            ? repo.DefaultBranch ?? "main"
            : query.Branch.Trim();

        var sync = await repoCacheManager.SynchronizeAsync(repo, repo.GitAccount, branch, cancellationToken);
        if (sync.Success != true || string.IsNullOrWhiteSpace(sync.Hash))
        {
            return Result.Failure<GitRepositoryComposeDiscovery>(
                sync.Error ?? "Repository sync did not resolve a commit.");
        }

        var snapshotRoot = Path.Combine(
            Path.GetTempPath(),
            "citadel",
            "git-compose-discovery",
            Guid.NewGuid().ToString("N"));

        try
        {
            var materialize = await gitCliRepository.MaterializeSnapshotAsync(
                repo.GetCachePath(),
                sync.Hash,
                snapshotRoot,
                cancellationToken);

            if (materialize.IsFailure(out var error))
            {
                return Result.Failure<GitRepositoryComposeDiscovery>(error.Message);
            }

            var projects = DiscoverProjects(snapshotRoot);
            return Result.Success(new GitRepositoryComposeDiscovery(
                repo.Id,
                branch,
                sync.Hash,
                projects));
        }
        finally
        {
            if (Directory.Exists(snapshotRoot))
            {
                Directory.Delete(snapshotRoot, recursive: true);
            }
        }
    }

    private static IReadOnlyList<GitComposeProjectCandidate> DiscoverProjects(string snapshotRoot)
    {
        if (!Directory.Exists(snapshotRoot))
        {
            return [];
        }

        return Directory.EnumerateFiles(snapshotRoot, "*.*", SearchOption.AllDirectories)
            .Where(IsComposeFile)
            .Select(path => new
            {
                FullPath = path,
                RelativePath = NormalizePath(Path.GetRelativePath(snapshotRoot, path)),
                Directory = NormalizeDirectory(Path.GetRelativePath(snapshotRoot, Path.GetDirectoryName(path)!)),
                Name = Path.GetFileName(path)
            })
            .GroupBy(item => item.Directory, StringComparer.OrdinalIgnoreCase)
            .Select(group =>
            {
                var composePaths = group
                    .OrderBy(item => GetComposeOrder(item.Name))
                    .ThenBy(item => item.RelativePath, StringComparer.OrdinalIgnoreCase)
                    .Select(item => item.RelativePath)
                    .ToArray();

                var envFiles = DiscoverEnvFiles(snapshotRoot, group.Key);
                return new GitComposeProjectCandidate(
                    WorkingDirectory: group.Key,
                    ComposePaths: composePaths,
                    EnvFilePaths: envFiles,
                    SuggestedWatchPaths: GetSuggestedWatchPaths(group.Key, composePaths, envFiles));
            })
            .OrderBy(project => project.WorkingDirectory == "." ? string.Empty : project.WorkingDirectory, StringComparer.OrdinalIgnoreCase)
            .ToArray();
    }

    private static bool IsComposeFile(string path)
    {
        var fileName = Path.GetFileName(path);
        if (KnownComposeFileNames.Contains(fileName))
        {
            return true;
        }

        if (!fileName.EndsWith(".yml", StringComparison.OrdinalIgnoreCase)
            && !fileName.EndsWith(".yaml", StringComparison.OrdinalIgnoreCase))
        {
            return false;
        }

        var name = Path.GetFileNameWithoutExtension(fileName);
        return name.EndsWith(".compose", StringComparison.OrdinalIgnoreCase)
            || name.Equals("compose", StringComparison.OrdinalIgnoreCase)
            || name.StartsWith("compose.", StringComparison.OrdinalIgnoreCase)
            || name.StartsWith("compose-", StringComparison.OrdinalIgnoreCase)
            || name.Equals("docker-compose", StringComparison.OrdinalIgnoreCase)
            || name.StartsWith("docker-compose.", StringComparison.OrdinalIgnoreCase)
            || name.StartsWith("docker-compose-", StringComparison.OrdinalIgnoreCase);
    }

    private static int GetComposeOrder(string fileName)
    {
        if (fileName.Equals("compose.yml", StringComparison.OrdinalIgnoreCase)
            || fileName.Equals("compose.yaml", StringComparison.OrdinalIgnoreCase)
            || fileName.Equals("docker-compose.yml", StringComparison.OrdinalIgnoreCase)
            || fileName.Equals("docker-compose.yaml", StringComparison.OrdinalIgnoreCase))
        {
            return 0;
        }

        if (fileName.Contains(".override.", StringComparison.OrdinalIgnoreCase))
        {
            return 10;
        }

        return 20;
    }

    private static IReadOnlyList<string> DiscoverEnvFiles(string snapshotRoot, string workingDirectory)
    {
        var directory = workingDirectory == "."
            ? snapshotRoot
            : Path.Combine(snapshotRoot, workingDirectory.Replace('/', Path.DirectorySeparatorChar));

        if (!Directory.Exists(directory))
        {
            return [];
        }

        return Directory.EnumerateFiles(directory, ".env", SearchOption.TopDirectoryOnly)
            .Select(path => NormalizePath(Path.GetRelativePath(snapshotRoot, path)))
            .ToArray();
    }

    private static IReadOnlyList<string> GetSuggestedWatchPaths(
        string workingDirectory,
        IReadOnlyList<string> composePaths,
        IReadOnlyList<string> envFilePaths)
    {
        if (workingDirectory != ".")
        {
            return [$"{workingDirectory}/**"];
        }

        return composePaths.Concat(envFilePaths).Distinct(StringComparer.Ordinal).ToArray();
    }

    private static string NormalizeDirectory(string path)
    {
        var normalized = NormalizePath(path);
        return string.IsNullOrWhiteSpace(normalized) ? "." : normalized;
    }

    private static string NormalizePath(string path)
        => path.Replace('\\', '/').TrimStart('/');
}
