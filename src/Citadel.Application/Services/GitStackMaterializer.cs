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

    Task ActivateCurrentAsync(Guid stackId, string snapshotRoot, CancellationToken cancellationToken);

    Task DiscardSnapshotAsync(Guid stackId, Guid releaseId, CancellationToken cancellationToken);

    Task PruneSnapshotsAsync(Guid stackId, IReadOnlyCollection<Guid> retainedReleaseIds, CancellationToken cancellationToken);
}

internal sealed class GitStackMaterializer(
    IRepoCacheManager repoCacheManager,
    IGitCliRepository gitCliRepository,
    IStackStoragePathProvider stackStoragePathProvider) : IGitStackMaterializer
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

        var releaseRoot = CreateReleaseDirectory(stack.Id, stack.CurrentStackReleaseId);
        var snapshotRoot = Path.Combine(releaseRoot, "source");
        var generatedFilesDirectory = Path.Combine(releaseRoot, "citadel");
        Result<GitStackMaterializationResult> FailAfterSnapshot(string message)
        {
            DeletePath(releaseRoot);
            return Result.Failure<GitStackMaterializationResult>(message);
        }

        var materialize = await gitCliRepository.MaterializeSnapshotAsync(repoPath, resolvedCommit, snapshotRoot, cancellationToken);
        if (materialize.IsFailure(out var materializeError))
            return FailAfterSnapshot(materializeError.Message);

        var composePaths = new List<string>(spec.ComposePaths.Count);
        var composeFilePaths = new List<string>(spec.ComposePaths.Count);
        var composeContents = new List<string>(spec.ComposePaths.Count);
        foreach (var composePath in spec.ComposePaths)
        {
            var resolvedPath = ResolveRepositoryFile(snapshotRoot, composePath, "Compose path");
            if (resolvedPath.IsFailure(out var pathError, out var fullPath))
                return FailAfterSnapshot(pathError.Message);

            var normalizedPath = NormalizeRepositoryPath(composePath);
            composePaths.Add(normalizedPath);
            composeFilePaths.Add(fullPath);
            composeContents.Add(await File.ReadAllTextAsync(fullPath, cancellationToken));
        }

        var workingDirectory = ResolveWorkingDirectory(snapshotRoot, spec.ComposePaths[0], spec.WorkingDirectory);
        if (workingDirectory.IsFailure(out var workingDirectoryError, out var sourceWorkingDirectory))
            return FailAfterSnapshot(workingDirectoryError.Message);

        var envVars = new List<string>();
        var envFilePaths = new List<string>();
        var sourceEnvFilePaths = new List<string>();
        var repoEnvFiles = spec.ComposeEnvFilesFromRepo ?? spec.AdditionalEnvFileFromRepo;

        if (repoEnvFiles is { Count: > 0 })
        {
            foreach (var envPath in repoEnvFiles)
            {
                var resolvedPath = ResolveRepositoryFile(snapshotRoot, envPath, "Environment file path");
                if (resolvedPath.IsFailure(out var pathError, out var fullPath))
                    return FailAfterSnapshot(pathError.Message);

                envFilePaths.Add(NormalizeRepositoryPath(envPath));
                sourceEnvFilePaths.Add(fullPath);
            }
        }

        if (spec.EnvVars is { Count: > 0 })
        {
            envVars.AddRange(spec.EnvVars.Where(value => !string.IsNullOrWhiteSpace(value)));
        }

        var watchPaths = NormalizeWatchPaths(snapshotRoot, spec.WatchPaths);
        if (watchPaths.IsFailure(out var watchPathError, out var normalizedWatchPaths))
            return FailAfterSnapshot(watchPathError.Message);

        Directory.CreateDirectory(generatedFilesDirectory);
        var labelsOverrideFilePath = Path.Combine(generatedFilesDirectory, "citadel.labels.yml");
        await File.WriteAllTextAsync(
            labelsOverrideFilePath,
            StackComposeLabelInjector.CreateLabelsOverride(composeContents, stack.Id, stack.CurrentStackReleaseId),
            cancellationToken);

        return Result.Success(new GitStackMaterializationResult(
            ResolvedCommitSha: resolvedCommit,
            SourceBranch: spec.Branch,
            SnapshotRoot: snapshotRoot,
            SourceWorkingDirectory: sourceWorkingDirectory,
            GeneratedFilesDirectory: generatedFilesDirectory,
            LabelsOverrideFilePath: labelsOverrideFilePath,
            EnvFilePath: spec.EnvFilePath,
            EnvironmentVariables: envVars,
            SourceComposeFilePaths: composeFilePaths,
            SourceEnvFilePaths: sourceEnvFilePaths,
            ComposePaths: composePaths,
            EnvFilePaths: envFilePaths,
            WatchPaths: normalizedWatchPaths));
    }

    public async Task ActivateCurrentAsync(Guid stackId, string snapshotRoot, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();

        var stackRoot = GetStackRoot(stackId);
        Directory.CreateDirectory(stackRoot);
        var currentPath = Path.Combine(stackRoot, "current");
        var pointerPath = Path.Combine(stackRoot, "current.source");
        var tempLinkPath = Path.Combine(stackRoot, $".current-{Guid.NewGuid():N}");

        try
        {
            Directory.CreateSymbolicLink(tempLinkPath, snapshotRoot);
            ReplacePath(currentPath, tempLinkPath);
            DeletePath(pointerPath);
            return;
        }
        catch
        {
            DeletePath(tempLinkPath);
        }

        var tempPointerPath = Path.Combine(stackRoot, $".current.source-{Guid.NewGuid():N}.tmp");

        DeletePath(currentPath);
        await File.WriteAllTextAsync(tempPointerPath, snapshotRoot, cancellationToken);
        File.Move(tempPointerPath, pointerPath, overwrite: true);
    }

    public Task PruneSnapshotsAsync(Guid stackId, IReadOnlyCollection<Guid> retainedReleaseIds, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();

        var releasesRoot = Path.Combine(GetStackRoot(stackId), "releases");
        if (!Directory.Exists(releasesRoot))
        {
            return Task.CompletedTask;
        }

        foreach (var releaseDirectory in Directory.EnumerateDirectories(releasesRoot))
        {
            cancellationToken.ThrowIfCancellationRequested();

            var releaseDirectoryName = Path.GetFileName(releaseDirectory);
            if (!Guid.TryParse(releaseDirectoryName, out var releaseId)
                || retainedReleaseIds.Contains(releaseId))
            {
                continue;
            }

            Directory.Delete(releaseDirectory, recursive: true);
        }

        return Task.CompletedTask;
    }

    public Task DiscardSnapshotAsync(Guid stackId, Guid releaseId, CancellationToken cancellationToken)
    {
        cancellationToken.ThrowIfCancellationRequested();

        var releaseRoot = Path.Combine(GetStackRoot(stackId), "releases", releaseId.ToString("D"));
        if (Directory.Exists(releaseRoot))
        {
            Directory.Delete(releaseRoot, recursive: true);
        }

        return Task.CompletedTask;
    }

    private string CreateReleaseDirectory(Guid stackId, Guid releaseId)
    {
        var releaseRoot = Path.Combine(GetStackRoot(stackId), "releases", releaseId.ToString("D"));
        if (Directory.Exists(releaseRoot))
            Directory.Delete(releaseRoot, recursive: true);

        Directory.CreateDirectory(releaseRoot);
        return releaseRoot;
    }

    private string GetStackRoot(Guid stackId)
        => Path.Combine(stackStoragePathProvider.StacksRoot, stackId.ToString("D"));

    private static Result<string> ResolveWorkingDirectory(string snapshotRoot, string firstComposePath, string? configuredWorkingDirectory)
    {
        var workingDirectory = string.IsNullOrWhiteSpace(configuredWorkingDirectory)
            ? Path.GetDirectoryName(NormalizeRepositoryPath(firstComposePath)) ?? string.Empty
            : configuredWorkingDirectory;

        if (string.IsNullOrWhiteSpace(workingDirectory))
        {
            return Result.Success(Path.GetFullPath(snapshotRoot));
        }

        var resolved = ResolveRepositoryDirectory(snapshotRoot, workingDirectory, "Working directory");
        return resolved;
    }

    private static void ReplacePath(string currentPath, string replacementPath)
    {
        DeletePath(currentPath);
        Directory.Move(replacementPath, currentPath);
    }

    private static void DeletePath(string path)
    {
        if (!Directory.Exists(path) && !File.Exists(path))
        {
            return;
        }

        var attributes = File.GetAttributes(path);
        if ((attributes & FileAttributes.Directory) != 0)
        {
            var isSymlink = (attributes & FileAttributes.ReparsePoint) != 0;
            Directory.Delete(path, recursive: !isSymlink);
            return;
        }

        File.Delete(path);
    }

    private static Result<string> ResolveRepositoryFile(string snapshotRoot, string relativePath, string fieldName)
    {
        if (string.IsNullOrWhiteSpace(relativePath))
            return Result.Failure<string>($"{fieldName} cannot be empty.");

        if (Path.IsPathRooted(relativePath))
            return Result.Failure<string>($"{fieldName} '{relativePath}' must be relative to the repository root.");

        var normalizedRoot = Path.GetFullPath(snapshotRoot);
        var fullPath = Path.GetFullPath(Path.Combine(normalizedRoot, relativePath));

        if (!IsUnderRootOrEqual(normalizedRoot, fullPath))
            return Result.Failure<string>($"{fieldName} '{relativePath}' escapes the repository root.");

        if (Directory.Exists(fullPath))
            return Result.Failure<string>($"{fieldName} '{relativePath}' points to a directory.");

        if (!File.Exists(fullPath))
            return Result.Failure<string>($"{fieldName} '{relativePath}' does not exist.");

        if (PathContainsLinkOutsideRoot(normalizedRoot, fullPath))
            return Result.Failure<string>($"{fieldName} '{relativePath}' points outside the repository root.");

        return Result.Success(fullPath);
    }

    private static Result<string> ResolveRepositoryDirectory(string snapshotRoot, string relativePath, string fieldName)
    {
        if (string.IsNullOrWhiteSpace(relativePath) || relativePath == ".")
            return Result.Success(Path.GetFullPath(snapshotRoot));

        if (Path.IsPathRooted(relativePath))
            return Result.Failure<string>($"{fieldName} '{relativePath}' must be relative to the repository root.");

        var normalizedRoot = Path.GetFullPath(snapshotRoot);
        var fullPath = Path.GetFullPath(Path.Combine(normalizedRoot, relativePath));

        if (!IsUnderRootOrEqual(normalizedRoot, fullPath))
            return Result.Failure<string>($"{fieldName} '{relativePath}' escapes the repository root.");

        if (!Directory.Exists(fullPath))
            return Result.Failure<string>($"{fieldName} '{relativePath}' does not exist.");

        if (PathContainsLinkOutsideRoot(normalizedRoot, fullPath))
            return Result.Failure<string>($"{fieldName} '{relativePath}' points outside the repository root.");

        return Result.Success(fullPath);
    }

    private static Result<IReadOnlyList<string>> NormalizeWatchPaths(string snapshotRoot, IReadOnlyList<string>? watchPaths)
    {
        if (watchPaths is not { Count: > 0 })
            return Result.Success<IReadOnlyList<string>>([]);

        var normalized = new List<string>(watchPaths.Count);
        foreach (var watchPath in watchPaths)
        {
            if (string.IsNullOrWhiteSpace(watchPath))
                continue;

            var hasWildcardSuffix = watchPath.EndsWith("/**", StringComparison.Ordinal)
                || watchPath.EndsWith("\\**", StringComparison.Ordinal);
            var pathToValidate = hasWildcardSuffix ? watchPath[..^3] : watchPath;

            if (Path.IsPathRooted(pathToValidate))
                return Result.Failure<IReadOnlyList<string>>($"Watch path '{watchPath}' must be relative to the repository root.");

            var normalizedRoot = Path.GetFullPath(snapshotRoot);
            var fullPath = Path.GetFullPath(Path.Combine(normalizedRoot, pathToValidate));
            if (!IsUnderRootOrEqual(normalizedRoot, fullPath))
                return Result.Failure<IReadOnlyList<string>>($"Watch path '{watchPath}' escapes the repository root.");

            if (PathContainsLinkOutsideRoot(normalizedRoot, fullPath))
                return Result.Failure<IReadOnlyList<string>>($"Watch path '{watchPath}' points outside the repository root.");

            normalized.Add(NormalizeRepositoryPath(watchPath));
        }

        return Result.Success<IReadOnlyList<string>>(normalized);
    }

    private static bool IsUnderRootOrEqual(string root, string path)
    {
        var normalizedRoot = Path.GetFullPath(root);
        var normalizedPath = Path.GetFullPath(path);

        if (string.Equals(
            normalizedPath.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar),
            normalizedRoot.TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar),
            StringComparison.OrdinalIgnoreCase))
        {
            return true;
        }

        if (!normalizedRoot.EndsWith(Path.DirectorySeparatorChar))
            normalizedRoot += Path.DirectorySeparatorChar;

        return normalizedPath.StartsWith(normalizedRoot, StringComparison.OrdinalIgnoreCase);
    }

    private static bool PathContainsLinkOutsideRoot(string normalizedRoot, string fullPath)
    {
        var root = Path.GetFullPath(normalizedRoot)
            .TrimEnd(Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar);
        var path = Path.GetFullPath(fullPath);

        if (!IsUnderRootOrEqual(root, path))
            return true;

        var relativePath = Path.GetRelativePath(root, path);
        if (string.IsNullOrWhiteSpace(relativePath) || relativePath == ".")
            return false;

        var current = root;
        foreach (var segment in relativePath.Split(
                     [Path.DirectorySeparatorChar, Path.AltDirectorySeparatorChar],
                     StringSplitOptions.RemoveEmptyEntries))
        {
            current = Path.Combine(current, segment);
            if (!Directory.Exists(current) && !File.Exists(current))
                return false;

            FileSystemInfo info = Directory.Exists(current)
                ? new DirectoryInfo(current)
                : new FileInfo(current);
            var target = info.ResolveLinkTarget(returnFinalTarget: true);
            if (target is not null && !IsUnderRootOrEqual(root, target.FullName))
                return true;
        }

        return false;
    }

    private static string NormalizeRepositoryPath(string path)
        => path.Replace('\\', '/').TrimStart('/');
}

internal sealed record GitStackMaterializationResult(
    string ResolvedCommitSha,
    string SourceBranch,
    string SnapshotRoot,
    string SourceWorkingDirectory,
    string GeneratedFilesDirectory,
    string LabelsOverrideFilePath,
    string? EnvFilePath,
    IReadOnlyList<string> EnvironmentVariables,
    IReadOnlyList<string> SourceComposeFilePaths,
    IReadOnlyList<string> SourceEnvFilePaths,
    IReadOnlyList<string> ComposePaths,
    IReadOnlyList<string> EnvFilePaths,
    IReadOnlyList<string> WatchPaths);
