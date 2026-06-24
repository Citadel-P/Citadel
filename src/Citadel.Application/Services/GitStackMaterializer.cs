using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Domain.Entities.Stacks;
using LightResults;

namespace Application.Services;

internal interface IGitStackMaterializer
{
    Task<Result<GitStackMaterializationResult>> MaterializeAsync(
        Stack stack,
        GitStack spec,
        GitRepository repository,
        CancellationToken cancellationToken);
}

internal sealed class GitStackMaterializer(
    IRepoCacheManager repoCacheManager,
    IGitCliRepository gitCliRepository) : IGitStackMaterializer
{
    public async Task<Result<GitStackMaterializationResult>> MaterializeAsync(
        Stack stack,
        GitStack spec,
        GitRepository repository,
        CancellationToken cancellationToken)
    {
        if (string.IsNullOrWhiteSpace(spec.Branch))
            return Result.Failure<GitStackMaterializationResult>("Git stack branch is required.");

        if (spec.ComposePaths is not { Count: > 0 })
            return Result.Failure<GitStackMaterializationResult>("At least one compose path is required for Git stacks.");

        var sync = await repoCacheManager.SynchronizeAsync(repository, repository.GitAccount, spec.Branch, cancellationToken);
        if (sync.Success != true)
            return Result.Failure<GitStackMaterializationResult>(sync.Error ?? "Repository sync failed.");

        var repoPath = repository.GetCachePath();
        string resolvedCommit;

        if (!string.IsNullOrWhiteSpace(spec.CommitSha))
        {
            var exists = await gitCliRepository.CommitExistsAsync(repoPath, spec.CommitSha, cancellationToken);
            if (exists.IsFailure())
            {
                return Result.Failure<GitStackMaterializationResult>(
                    $"Git stack apply failed: commit {spec.CommitSha} was not found in repository {repository.Name}.");
            }

            var requestedCommit = await gitCliRepository.ResolveSnapshotCommitAsync(repoPath, spec.CommitSha, cancellationToken);
            if (requestedCommit.IsFailure(out var error, out var hash))
                return Result.Failure<GitStackMaterializationResult>(error.Message);

            resolvedCommit = hash.Trim();
        }
        else
        {
            if (string.IsNullOrWhiteSpace(sync.Hash))
                return Result.Failure<GitStackMaterializationResult>("Repository sync did not resolve a commit.");

            resolvedCommit = sync.Hash.Trim();
        }

        var snapshotRoot = CreateSnapshotDirectory(stack.Id, stack.CurrentStackReleaseId, resolvedCommit);
        var materialize = await gitCliRepository.MaterializeSnapshotAsync(repoPath, resolvedCommit, snapshotRoot, cancellationToken);
        if (materialize.IsFailure(out var materializeError))
            return Result.Failure<GitStackMaterializationResult>(materializeError.Message);

        var composePaths = new List<string>(spec.ComposePaths.Count);
        var composeContents = new List<string>(spec.ComposePaths.Count);
        foreach (var composePath in spec.ComposePaths)
        {
            var resolvedPath = ResolveRepositoryFile(snapshotRoot, composePath, "Compose path");
            if (resolvedPath.IsFailure(out var pathError, out var fullPath))
                return Result.Failure<GitStackMaterializationResult>(pathError.Message);

            composePaths.Add(NormalizeRepositoryPath(composePath));
            composeContents.Add(await File.ReadAllTextAsync(fullPath, cancellationToken));
        }

        var envVars = new List<string>();
        var envFilePaths = new List<string>();

        if (spec.AdditionalEnvFileFromRepo is { Count: > 0 })
        {
            foreach (var envPath in spec.AdditionalEnvFileFromRepo)
            {
                var resolvedPath = ResolveRepositoryFile(snapshotRoot, envPath, "Environment file path");
                if (resolvedPath.IsFailure(out var pathError, out var fullPath))
                    return Result.Failure<GitStackMaterializationResult>(pathError.Message);

                envFilePaths.Add(NormalizeRepositoryPath(envPath));
                envVars.AddRange(await File.ReadAllLinesAsync(fullPath, cancellationToken));
            }
        }

        if (spec.EnvVars is { Count: > 0 })
        {
            envVars.AddRange(spec.EnvVars.Where(value => !string.IsNullOrWhiteSpace(value)));
        }

        return Result.Success(new GitStackMaterializationResult(
            ResolvedCommitSha: resolvedCommit,
            SourceBranch: spec.Branch,
            WorkingDirectory: snapshotRoot,
            ComposeContent: string.Join(Environment.NewLine, composeContents),
            EnvFilePath: spec.EnvFilePath,
            EnvironmentVariables: envVars,
            ComposePaths: composePaths,
            EnvFilePaths: envFilePaths));
    }

    private static string CreateSnapshotDirectory(Guid stackId, Guid releaseId, string commitSha)
    {
        var safeCommit = commitSha.Length > 12 ? commitSha[..12] : commitSha;
        var root = Path.Combine(Path.GetTempPath(), "citadel", "git-stacks", stackId.ToString("N"));
        Directory.CreateDirectory(root);
        return Path.Combine(root, $"{releaseId:N}-{safeCommit}-{Guid.CreateVersion7():N}");
    }

    private static Result<string> ResolveRepositoryFile(string snapshotRoot, string relativePath, string fieldName)
    {
        if (string.IsNullOrWhiteSpace(relativePath))
            return Result.Failure<string>($"{fieldName} cannot be empty.");

        if (Path.IsPathRooted(relativePath))
            return Result.Failure<string>($"{fieldName} '{relativePath}' must be relative to the repository root.");

        var normalizedRoot = Path.GetFullPath(snapshotRoot);
        var fullPath = Path.GetFullPath(Path.Combine(normalizedRoot, relativePath));

        if (!IsUnderRoot(normalizedRoot, fullPath))
            return Result.Failure<string>($"{fieldName} '{relativePath}' escapes the repository root.");

        if (Directory.Exists(fullPath))
            return Result.Failure<string>($"{fieldName} '{relativePath}' points to a directory.");

        if (!File.Exists(fullPath))
            return Result.Failure<string>($"{fieldName} '{relativePath}' does not exist.");

        var fileInfo = new FileInfo(fullPath);
        var target = fileInfo.ResolveLinkTarget(returnFinalTarget: true);
        if (target is not null && !IsUnderRoot(normalizedRoot, target.FullName))
            return Result.Failure<string>($"{fieldName} '{relativePath}' points outside the repository root.");

        return Result.Success(fullPath);
    }

    private static bool IsUnderRoot(string root, string path)
    {
        var normalizedRoot = Path.GetFullPath(root);
        if (!normalizedRoot.EndsWith(Path.DirectorySeparatorChar))
            normalizedRoot += Path.DirectorySeparatorChar;

        var normalizedPath = Path.GetFullPath(path);
        return normalizedPath.StartsWith(normalizedRoot, StringComparison.OrdinalIgnoreCase);
    }

    private static string NormalizeRepositoryPath(string path)
        => path.Replace('\\', '/').TrimStart('/');
}

internal sealed record GitStackMaterializationResult(
    string ResolvedCommitSha,
    string SourceBranch,
    string WorkingDirectory,
    string ComposeContent,
    string? EnvFilePath,
    IReadOnlyList<string> EnvironmentVariables,
    IReadOnlyList<string> ComposePaths,
    IReadOnlyList<string> EnvFilePaths);
