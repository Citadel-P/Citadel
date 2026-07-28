using Domain.Contracts.Interfaces;
using Domain;
using Domain.Contracts.Resources.Git;
using Domain.Entities.Git;
using Hosting.DockerClient.Services;
using Infrastructure.Repositories.Mappers;
using LightResults;
using System.Collections.Concurrent;
using System.Collections.Frozen;
using System.Globalization;
using System.Runtime.InteropServices;
using System.Security.AccessControl;
using System.Security.Cryptography;
using System.Security.Principal;
using System.Text;

namespace Infrastructure.Repositories;

/// <inheritdoc/>
internal class GitCliRepository(ICommandExecutor processService) : IGitCliRepository
{
    private const string GitExecutable = "git";
    private const int MaximumLookupOutputBytes = 64 * 1024;
    private const int MaximumStandardErrorBytes = 16 * 1024;
    private static readonly UTF8Encoding StrictUtf8 = new(false, true);

    private static readonly FrozenDictionary<string, string> GitEnv = new Dictionary<string, string>
    {
        { "GIT_TERMINAL_PROMPT", "0" },
        { "GIT_ASKPASS", "echo" },
        { "GIT_SSH_COMMAND", "ssh -o BatchMode=yes" },
        { "GIT_LITERAL_PATHSPECS", "1" },
        { "LC_ALL", "C" }
    }.ToFrozenDictionary();

    private static readonly string CitadelTempDir = Path.Combine(Path.GetTempPath(), "citadel");

    // accountId -> resolved key file path (content-addressed); rewrites only when the key changes
    private static readonly ConcurrentDictionary<Guid, string> SshKeyCache = new();

    public async Task<Result> TestConnectionAsync(string url, GitAccount? account, CancellationToken ct = default)
    {
        var (args, env) = PrepareRemoteCmd(account);
        // ls-remote only checks connectivity and refs, it doesn't download the repo
        args.AddRange(["ls-remote", "-h", url]);

        var result = await processService.ExecuteAsync(GitExecutable, args, env, null, ct);

        if (!result.IsSuccess)
        {
            return Result.Failure($"Could not connect to repository. Check your URL and credentials. Error: {result.StandardError}");
        }

        return Result.Success();
    }

    public async Task<Result<IReadOnlyList<GitRemoteBranchRef>>> ListRemoteBranchesAsync(
        string url,
        GitAccount? account = null,
        CancellationToken ct = default)
    {
        var (args, env) = PrepareRemoteCmd(account);
        args.AddRange(["ls-remote", "--heads", url]);

        var result = await processService.ExecuteAsync(GitExecutable, args, env, null, ct);
        if (!result.IsSuccess)
        {
            return Result.Failure<IReadOnlyList<GitRemoteBranchRef>>(
                $"Could not discover repository branches. Error: {result.StandardError}");
        }

        var branches = result.StandardOutput
            .Split(['\r', '\n'], StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
            .Select(ParseRemoteBranchRef)
            .Where(branch => branch is not null)
            .Select(branch => branch!)
            .OrderBy(branch => branch.Branch, StringComparer.OrdinalIgnoreCase)
            .ToArray();

        return Result.Success<IReadOnlyList<GitRemoteBranchRef>>(branches);
    }

    public async Task<Result<string>> ResolveSnapshotCommitAsync(string repoPath, string branch, CancellationToken ct = default)
    {
        var candidates = new[]
        {
            $"refs/remotes/origin/{branch}",
            $"refs/heads/{branch}",
            branch,
            "HEAD"
        };

        foreach (var candidate in candidates)
        {
            var args = new[] { "-C", NormalizeLocalGitPath(repoPath), "rev-parse", candidate };
            var result = await processService.ExecuteAsync(GitExecutable, args, GitEnv, "", ct);

            if (result.IsSuccess)
                return Result.Success(result.StandardOutput.Trim());
        }

        var error = $"Failed to resolve commit hash for repo at {repoPath} and branch {branch}. Tried refs/remotes/origin/{branch}, refs/heads/{branch}, {branch}, and HEAD.";
        return Result.Failure<string>(error);
    }

    public async Task<Result> CloneAsync(string url, string targetPath, string branch, GitAccount? account, CancellationToken ct = default)
    {
        var (args, env) = PrepareRemoteCmd(account);
        args.AddRange(["clone", "-b", branch, "--single-branch", url, NormalizeLocalGitPath(targetPath)]);

        var result = await processService.ExecuteAsync(GitExecutable, args, env, "", ct);
        return result.Map();
    }

    public async Task<Result> PullAsync(string repoPath, string branch, GitAccount? account, CancellationToken ct = default)
    {
        var (args, env) = PrepareRemoteCmd(account);
        args.AddRange(["-C", NormalizeLocalGitPath(repoPath), "pull", "origin", branch]);

        var result = await processService.ExecuteAsync(GitExecutable, args, env, "", ct);
        return result.Map();
    }

    public async Task<Result> FetchAsync(string repoPath, string branch, GitAccount? account, CancellationToken ct = default)
    {
        var (args, env) = PrepareRemoteCmd(account);
        args.AddRange(["-C", NormalizeLocalGitPath(repoPath), "fetch", "origin", $"{branch}:refs/remotes/origin/{branch}"]);

        var result = await processService.ExecuteAsync(GitExecutable, args, env, "", ct);
        return result.Map();
    }

    public async Task<Result> ResetWorkingTreeAsync(string repoPath, string branch, CancellationToken ct = default)
    {
        var normalizedRepoPath = NormalizeLocalGitPath(repoPath);
        var remoteRef = $"refs/remotes/origin/{branch}";

        var checkout = await processService.ExecuteAsync(
            GitExecutable,
            ["-C", normalizedRepoPath, "checkout", "-B", branch, remoteRef],
            GitEnv,
            "",
            ct);

        if (!checkout.IsSuccess)
            return checkout.Map();

        var reset = await processService.ExecuteAsync(
            GitExecutable,
            ["-C", normalizedRepoPath, "reset", "--hard", remoteRef],
            GitEnv,
            "",
            ct);

        return reset.Map();
    }

    public async Task<Result> CommitExistsAsync(string repoPath, string commitSha, CancellationToken ct = default)
    {
        var args = new[] { "-C", NormalizeLocalGitPath(repoPath), "cat-file", "-e", $"{commitSha}^{{commit}}" };
        var result = await processService.ExecuteAsync(GitExecutable, args, GitEnv, "", ct);
        return result.Map();
    }

    public async Task<Result<IReadOnlyList<string>>> GetChangedPathsAsync(
        string repoPath,
        string fromCommitSha,
        string toCommitSha,
        CancellationToken ct = default)
    {
        var args = new[]
        {
            "-C",
            NormalizeLocalGitPath(repoPath),
            "diff",
            "--name-only",
            "--find-renames",
            $"{fromCommitSha}..{toCommitSha}"
        };

        var result = await processService.ExecuteAsync(GitExecutable, args, GitEnv, "", ct);
        if (!result.IsSuccess)
        {
            return Result.Failure<IReadOnlyList<string>>(result.StandardError);
        }

        var paths = result.StandardOutput
            .Split(['\r', '\n'], StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries)
            .Select(path => path.Replace('\\', '/').TrimStart('/'))
            .Where(path => !string.IsNullOrWhiteSpace(path))
            .Distinct(StringComparer.Ordinal)
            .ToArray();

        return Result.Success<IReadOnlyList<string>>(paths);
    }

    public async Task<Result<string>> ResolveCommitAsync(
        string repoPath,
        string commitSha,
        CancellationToken ct = default)
    {
        var result = await processService.ExecuteBoundedAsync(
            GitExecutable,
            ["-C", NormalizeLocalGitPath(repoPath), "rev-parse", "--verify", $"{commitSha}^{{commit}}"],
            maximumStandardOutputBytes: 256,
            maximumStandardErrorBytes: MaximumStandardErrorBytes,
            GitEnv,
            "",
            ct);

        if (!result.IsSuccess || result.StandardOutputTruncated)
            return Result.Failure<string>(GitFailure(result, "Commit could not be resolved."));

        var resolved = Encoding.ASCII.GetString(result.StandardOutput.Span).Trim();
        return IsFullObjectId(resolved)
            ? Result.Success(resolved.ToLowerInvariant())
            : Result.Failure<string>("Git returned an invalid commit identifier.");
    }

    public async Task<Result<GitTreeListing>> ListTreeAsync(
        string repoPath,
        string commitSha,
        string path,
        int maximumEntries,
        int maximumOutputBytes,
        CancellationToken ct = default)
    {
        if (maximumEntries <= 0 || maximumOutputBytes <= 0)
            return Result.Failure<GitTreeListing>("Git tree limits must be positive.");

        string treeObject;
        if (path.Length == 0)
        {
            var rootResult = await ResolveTreeObjectAsync(repoPath, $"{commitSha}^{{tree}}", ct);
            if (rootResult.IsFailure(out var rootError, out treeObject))
                return Result.Failure<GitTreeListing>(rootError);
        }
        else
        {
            var entryResult = await GetTreeEntryAsync(repoPath, commitSha, path, ct);
            if (entryResult.IsFailure(out var entryError, out var entry))
                return Result.Failure<GitTreeListing>(entryError);

            if (entry is null)
                return Result.Failure<GitTreeListing>("Repository path does not exist.");

            if (entry.Type != GitRepositoryEntryType.Directory)
                return Result.Failure<GitTreeListing>("Repository path is not a directory.");

            treeObject = entry.ObjectId;
        }

        var result = await processService.ExecuteBoundedAsync(
            GitExecutable,
            ["-C", NormalizeLocalGitPath(repoPath), "ls-tree", "-z", "-l", treeObject],
            maximumOutputBytes,
            MaximumStandardErrorBytes,
            GitEnv,
            "",
            ct);

        if (!result.IsSuccess && !result.StandardOutputTruncated)
            return Result.Failure<GitTreeListing>(GitFailure(result, "Repository directory could not be read."));

        try
        {
            var parsed = ParseTreeEntries(result.StandardOutput.Span, path, maximumEntries);
            return new GitTreeListing(
                parsed.Entries,
                result.StandardOutputTruncated || parsed.IsTruncated);
        }
        catch (Exception ex) when (ex is FormatException or DecoderFallbackException or OverflowException)
        {
            return Result.Failure<GitTreeListing>("Git returned an invalid tree listing.");
        }
    }

    public async Task<Result<GitTreeEntry?>> GetTreeEntryAsync(
        string repoPath,
        string commitSha,
        string path,
        CancellationToken ct = default)
    {
        var result = await processService.ExecuteBoundedAsync(
            GitExecutable,
            ["-C", NormalizeLocalGitPath(repoPath), "ls-tree", "-z", "-l", commitSha, "--", path],
            MaximumLookupOutputBytes,
            MaximumStandardErrorBytes,
            GitEnv,
            "",
            ct);

        if (!result.IsSuccess || result.StandardOutputTruncated)
            return Result.Failure<GitTreeEntry?>(GitFailure(result, "Repository path could not be resolved."));

        try
        {
            var parsed = ParseTreeEntries(result.StandardOutput.Span, parentPath: null, maximumEntries: 2);
            if (parsed.Entries.Count == 0)
                return Result.Success<GitTreeEntry?>(null);

            var exact = parsed.Entries.FirstOrDefault(entry =>
                string.Equals(entry.Path, path, StringComparison.Ordinal));
            return Result.Success<GitTreeEntry?>(exact);
        }
        catch (Exception ex) when (ex is FormatException or DecoderFallbackException or OverflowException)
        {
            return Result.Failure<GitTreeEntry?>("Git returned invalid path metadata.");
        }
    }

    public async Task<Result<GitBlob>> ReadBlobAsync(
        string repoPath,
        string objectId,
        int maximumBytes,
        CancellationToken ct = default)
    {
        if (maximumBytes <= 0)
            return Result.Failure<GitBlob>("Git blob limit must be positive.");

        var result = await processService.ExecuteBoundedAsync(
            GitExecutable,
            ["-C", NormalizeLocalGitPath(repoPath), "cat-file", "blob", objectId],
            maximumBytes,
            MaximumStandardErrorBytes,
            GitEnv,
            "",
            ct);

        if (!result.IsSuccess && !result.StandardOutputTruncated)
            return Result.Failure<GitBlob>(GitFailure(result, "Repository file could not be read."));

        return new GitBlob(result.StandardOutput, result.StandardOutputTruncated);
    }

    public async Task<Result<GitChangedPathListing>> CompareCommitsAsync(
        string repoPath,
        string baseCommitSha,
        string headCommitSha,
        int maximumEntries,
        int maximumOutputBytes,
        CancellationToken ct = default)
    {
        if (maximumEntries <= 0 || maximumOutputBytes <= 0)
            return Result.Failure<GitChangedPathListing>("Git comparison limits must be positive.");

        var result = await processService.ExecuteBoundedAsync(
            GitExecutable,
            [
                "-C",
                NormalizeLocalGitPath(repoPath),
                "diff",
                "--name-status",
                "-z",
                "--find-renames",
                "--find-copies",
                baseCommitSha,
                headCommitSha,
                "--"
            ],
            maximumOutputBytes,
            MaximumStandardErrorBytes,
            GitEnv,
            "",
            ct);

        if (!result.IsSuccess && !result.StandardOutputTruncated)
            return Result.Failure<GitChangedPathListing>(GitFailure(result, "Repository commits could not be compared."));

        try
        {
            var parsed = ParseChangedPaths(result.StandardOutput.Span, maximumEntries);
            return new GitChangedPathListing(
                parsed.Files,
                result.StandardOutputTruncated || parsed.IsTruncated);
        }
        catch (Exception ex) when (ex is FormatException or DecoderFallbackException)
        {
            return Result.Failure<GitChangedPathListing>("Git returned an invalid comparison.");
        }
    }

    public async Task<Result> MaterializeSnapshotAsync(string repoPath, string commitSha, string targetPath, CancellationToken ct = default)
    {
        if (Directory.Exists(targetPath))
            Directory.Delete(targetPath, recursive: true);

        var normalizedRepoPath = NormalizeLocalGitPath(repoPath);
        var normalizedTargetPath = NormalizeLocalGitPath(targetPath);

        var clone = await processService.ExecuteAsync(
            GitExecutable,
            ["clone", "--no-checkout", "--no-hardlinks", normalizedRepoPath, normalizedTargetPath],
            GitEnv,
            "",
            ct);

        if (!clone.IsSuccess)
            return clone.Map();

        var checkout = await processService.ExecuteAsync(
            GitExecutable,
            ["-C", normalizedTargetPath, "checkout", "--detach", commitSha],
            GitEnv,
            "",
            ct);

        return checkout.Map();
    }

    public async Task<Result> ExecuteShellCommandAsync(string workingDir, string command, CancellationToken ct = default)
    {
        // Detect OS to use correct shell wrapper
        var isWindows = RuntimeInformation.IsOSPlatform(OSPlatform.Windows);
        var fileName = isWindows ? "cmd.exe" : "/bin/sh";
        var args = isWindows ? new[] { "/c", command } : new[] { "-c", command };

        // Execute in the specific sub-path provided by RepoCommand
        var result = await processService.ExecuteAsync(fileName, args, null, workingDir, ct);

        if (!result.IsSuccess)
        {
            return Result.Failure($"Command '{command}' failed in {workingDir}. Error: {result.StandardError}");
        }

        return Result.Success();
    }

    private static (List<string> Args, IDictionary<string, string> Env) PrepareRemoteCmd(GitAccount? account)
    {
        var args = new List<string>(8)
        {
            "-c",
            "core.askPass=echo"
        };

        if (account?.Configuration is TokenAuth token && account.Transport is GitTransport.Http or GitTransport.Https && !string.IsNullOrEmpty(token.Token))
        {
            var helper = $"!f() {{ echo \"username=git\"; echo \"password={token.Token}\"; }}; f";
            args.Add("-c");
            args.Add($"credential.helper={helper}");
            return (args, GitEnv);
        }

        if (account?.Configuration is BasicAuth basic && account.Transport is GitTransport.Http or GitTransport.Https)
        {
            var helper = $"!f() {{ echo \"username={basic.Username}\"; echo \"password={basic.Password}\"; }}; f";
            args.Add("-c");
            args.Add($"credential.helper={helper}");
            return (args, GitEnv);
        }

        if (account?.Configuration is SshKeyAuth ssh && account.Transport == GitTransport.Ssh)
        {
            var keyFile = GetOrWriteSshKey(account.Id, ssh.PrivateKey);
            var sshCmd = $"ssh -i {keyFile} -o IdentitiesOnly=yes -o StrictHostKeyChecking=no -o BatchMode=yes";

            if (!string.IsNullOrEmpty(ssh.Username))
                sshCmd += $" -l {ssh.Username}";

            var env = new Dictionary<string, string>(GitEnv)
            {
                ["GIT_SSH_COMMAND"] = sshCmd
            };
            return (args, env);
        }

        return (args, GitEnv);
    }

    private static GitRemoteBranchRef? ParseRemoteBranchRef(string line)
    {
        var parts = line.Split(['\t', ' '], StringSplitOptions.RemoveEmptyEntries | StringSplitOptions.TrimEntries);
        if (parts.Length < 2)
            return null;

        const string headPrefix = "refs/heads/";
        var refName = parts[1];
        if (!refName.StartsWith(headPrefix, StringComparison.Ordinal))
            return null;

        var branch = refName[headPrefix.Length..];
        if (string.IsNullOrWhiteSpace(branch) || string.IsNullOrWhiteSpace(parts[0]))
            return null;

        return new GitRemoteBranchRef(branch, parts[0]);
    }

    private async Task<Result<string>> ResolveTreeObjectAsync(
        string repoPath,
        string objectSpec,
        CancellationToken ct)
    {
        var result = await processService.ExecuteBoundedAsync(
            GitExecutable,
            ["-C", NormalizeLocalGitPath(repoPath), "rev-parse", "--verify", objectSpec],
            256,
            MaximumStandardErrorBytes,
            GitEnv,
            "",
            ct);

        if (!result.IsSuccess || result.StandardOutputTruncated)
            return Result.Failure<string>(GitFailure(result, "Repository tree could not be resolved."));

        var objectId = Encoding.ASCII.GetString(result.StandardOutput.Span).Trim();
        return IsFullObjectId(objectId)
            ? Result.Success(objectId.ToLowerInvariant())
            : Result.Failure<string>("Git returned an invalid tree identifier.");
    }

    private static (IReadOnlyList<GitTreeEntry> Entries, bool IsTruncated) ParseTreeEntries(
        ReadOnlySpan<byte> output,
        string? parentPath,
        int maximumEntries)
    {
        var entries = new List<GitTreeEntry>(Math.Min(maximumEntries, 64));
        var offset = 0;
        var isTruncated = false;

        while (offset < output.Length)
        {
            var terminator = output[offset..].IndexOf((byte)0);
            if (terminator < 0)
            {
                isTruncated = true;
                break;
            }

            var record = output.Slice(offset, terminator);
            offset += terminator + 1;
            if (record.Length == 0)
                continue;

            if (entries.Count >= maximumEntries)
            {
                isTruncated = true;
                break;
            }

            var separator = record.IndexOf((byte)'\t');
            if (separator <= 0 || separator == record.Length - 1)
                throw new FormatException("Invalid ls-tree record.");

            var metadata = Encoding.ASCII.GetString(record[..separator]);
            var metadataParts = metadata.Split(' ', StringSplitOptions.RemoveEmptyEntries);
            if (metadataParts.Length != 4)
                throw new FormatException("Invalid ls-tree metadata.");

            var reportedPath = StrictUtf8.GetString(record[(separator + 1)..]);
            var path = parentPath is null
                ? reportedPath
                : parentPath.Length == 0
                    ? reportedPath
                    : $"{parentPath}/{reportedPath}";
            var name = reportedPath[(reportedPath.LastIndexOf('/') + 1)..];
            var type = MapEntryType(metadataParts[0]);
            long? size = metadataParts[3] == "-"
                ? null
                : long.Parse(metadataParts[3], NumberStyles.None, CultureInfo.InvariantCulture);

            entries.Add(new GitTreeEntry(
                name,
                path,
                type,
                size,
                metadataParts[0],
                metadataParts[2]));
        }

        return (entries, isTruncated);
    }

    private static (IReadOnlyList<GitChangedPath> Files, bool IsTruncated) ParseChangedPaths(
        ReadOnlySpan<byte> output,
        int maximumEntries)
    {
        var fields = ParseNullDelimitedStrings(output, out var hasIncompleteField);
        var files = new List<GitChangedPath>(Math.Min(maximumEntries, 64));
        var offset = 0;
        var isTruncated = hasIncompleteField;

        while (offset < fields.Count)
        {
            if (files.Count >= maximumEntries)
            {
                isTruncated = true;
                break;
            }

            var statusToken = fields[offset++];
            if (statusToken.Length == 0)
                continue;

            var status = statusToken[0];
            if (status is 'R' or 'C')
            {
                if (offset + 1 >= fields.Count)
                {
                    isTruncated = true;
                    break;
                }

                var previousPath = fields[offset++];
                var path = fields[offset++];
                files.Add(new GitChangedPath(
                    status == 'R' ? GitChangedPathStatus.Renamed : GitChangedPathStatus.Copied,
                    path,
                    previousPath));
                continue;
            }

            if (offset >= fields.Count)
            {
                isTruncated = true;
                break;
            }

            var changedPath = fields[offset++];
            files.Add(new GitChangedPath(
                status switch
                {
                    'A' => GitChangedPathStatus.Added,
                    'M' => GitChangedPathStatus.Modified,
                    'D' => GitChangedPathStatus.Deleted,
                    'T' => GitChangedPathStatus.TypeChanged,
                    _ => throw new FormatException($"Unsupported Git change status '{status}'.")
                },
                changedPath,
                PreviousPath: null));
        }

        return (files, isTruncated);
    }

    private static IReadOnlyList<string> ParseNullDelimitedStrings(ReadOnlySpan<byte> output)
        => ParseNullDelimitedStrings(output, out _);

    private static IReadOnlyList<string> ParseNullDelimitedStrings(
        ReadOnlySpan<byte> output,
        out bool hasIncompleteField)
    {
        var fields = new List<string>();
        var offset = 0;
        hasIncompleteField = false;

        while (offset < output.Length)
        {
            var terminator = output[offset..].IndexOf((byte)0);
            if (terminator < 0)
            {
                hasIncompleteField = true;
                break;
            }

            fields.Add(StrictUtf8.GetString(output.Slice(offset, terminator)));
            offset += terminator + 1;
        }

        return fields;
    }

    private static GitRepositoryEntryType MapEntryType(string mode)
        => mode switch
        {
            "040000" => GitRepositoryEntryType.Directory,
            "100644" or "100755" => GitRepositoryEntryType.File,
            "120000" => GitRepositoryEntryType.Symlink,
            "160000" => GitRepositoryEntryType.Submodule,
            _ => throw new FormatException($"Unsupported Git tree mode '{mode}'.")
        };

    private static string GitFailure(ProcessBinaryExecutionResult result, string fallback)
        => string.IsNullOrWhiteSpace(result.StandardError)
            ? fallback
            : result.StandardError.Trim();

    private static bool IsFullObjectId(string value)
        => value.Length is 40 or 64 && value.All(Uri.IsHexDigit);

    private static string GetOrWriteSshKey(Guid accountId, string privateKey)
    {
        var contentHash = SHA256.HashData(Encoding.UTF8.GetBytes(privateKey));
        var shortHash = Convert.ToHexStringLower(contentHash.AsSpan(0, 4));
        var keyDir = Path.Combine(CitadelTempDir, "ssh");
        var keyFile = Path.Combine(keyDir, $"{accountId:N}_{shortHash}");

        if (SshKeyCache.TryGetValue(accountId, out var cached) && cached == keyFile && File.Exists(keyFile))
            return keyFile;

        Directory.CreateDirectory(keyDir);
        File.WriteAllText(keyFile, privateKey.TrimEnd() + "\n");
        SetKeyFilePermissions(keyFile);
        SshKeyCache[accountId] = keyFile;

        return keyFile;
    }
    private static void SetKeyFilePermissions(string keyFile)
    {
        if (RuntimeInformation.IsOSPlatform(OSPlatform.Windows))
        {
            var currentUser = WindowsIdentity.GetCurrent().User
                ?? throw new InvalidOperationException("Unable to resolve current Windows user SID for SSH key permissions.");

            var fileSecurity = new FileSecurity();
            fileSecurity.SetAccessRuleProtection(isProtected: true, preserveInheritance: false);
            fileSecurity.SetOwner(currentUser);
            fileSecurity.AddAccessRule(new FileSystemAccessRule(
                currentUser,
                FileSystemRights.Read | FileSystemRights.Write,
                AccessControlType.Allow));

            new FileInfo(keyFile).SetAccessControl(fileSecurity);
            return;
        }

        File.SetUnixFileMode(keyFile, UnixFileMode.UserRead | UnixFileMode.UserWrite);
    }

    private static string NormalizeLocalGitPath(string path)
    {
        if (!RuntimeInformation.IsOSPlatform(OSPlatform.Windows)
            || string.IsNullOrWhiteSpace(path)
            || path[0] != '/'
            || path.StartsWith("//", StringComparison.Ordinal))
        {
            return path;
        }

        return Path.GetFullPath(path);
    }
}
