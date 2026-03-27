using Domain.Contracts.Interfaces;
using Domain;
using Domain.Entities.Git;
using Hosting.DockerClient.Services;
using Infrastructure.Repositories.Mappers;
using LightResults;
using System.Collections.Concurrent;
using System.Collections.Frozen;
using System.Formats.Tar;
using System.Runtime.InteropServices;
using System.Security.AccessControl;
using System.Security.Cryptography;
using System.Security.Principal;
using System.Text;

namespace Infrastructure.Repositories;

/// <inheritdoc/>
internal class GitCliRepository(IProcessService processService) : IGitCliRepository
{
    private const string GitExecutable = "git";

    private static readonly FrozenDictionary<string, string> GitEnv = new Dictionary<string, string>
    {
        { "GIT_TERMINAL_PROMPT", "0" },
        { "GIT_ASKPASS", "echo" },
        { "GIT_SSH_COMMAND", "ssh -o BatchMode=yes" }
    }.ToFrozenDictionary();

    private static readonly string CitadelTempDir = Path.Combine(Path.GetTempPath(), "citadel");

    // accountId -> resolved key file path (content-addressed); rewrites only when the key changes
    private static readonly ConcurrentDictionary<Guid, string> SshKeyCache = new();

    public async Task<Result> CloneBareAsync(string url, string targetPath, GitAccount? account, CancellationToken ct = default)
    {
        var (args, env) = PrepareRemoteCmd(account);
        args.Add("clone");
        args.Add("--bare");
        args.Add(url);
        args.Add(targetPath);

        var result = await processService.ExecuteAsync(GitExecutable, args, env, ct);
        return result.Map();
    }

    public async Task<Result> FetchBranchSnapshotAsync(string repoPath, string branch, GitAccount? account, CancellationToken ct = default)
    {
        var (args, env) = PrepareRemoteCmd(account);
        args.Add("-C");
        args.Add(repoPath);
        args.Add("fetch");
        args.Add("origin");
        args.Add($"{branch}:{ToRefName(branch)}");
        args.Add("--prune");
        args.Add("--depth=1");

        var result = await processService.ExecuteAsync(GitExecutable, args, env, ct);
        return result.Map();
    }

    public async Task<Result<string>> ResolveSnapshotCommitAsync(string repoPath, string branch, CancellationToken ct = default)
    {
        var refName = ToRefName(branch);
        var args = new[] { "-C", repoPath, "rev-parse", refName };
        var result = await processService.ExecuteAsync(GitExecutable, args, GitEnv, ct);

        if (!result.IsSuccess)
        {
            var error = $"Failed to resolve commit hash for repo at {repoPath} and branch {branch}. ExitCode={result.ExitCode}. Error={result.StandardError}";
            return Result.Failure<string>(error);
        }
        return Result.Success(result.StandardOutput.Trim());
    }

    public async Task<Result> MaterializeAsync(string repoPath, string commitHash, string targetPath, CancellationToken ct = default)
    {
        Directory.CreateDirectory(targetPath);

        var archiveDir = Path.Combine(CitadelTempDir, "archives");
        Directory.CreateDirectory(archiveDir);
        var tarFile = Path.Combine(archiveDir, $"{Guid.NewGuid():N}.tar");
        try
        {
            var args = new[] { "--git-dir", repoPath, "archive", "--format=tar", $"--output={tarFile}", commitHash };
            var result = await processService.ExecuteAsync(GitExecutable, args, GitEnv, ct);

            if (!result.IsSuccess)
                return result.Map();

            await TarFile.ExtractToDirectoryAsync(tarFile, targetPath, overwriteFiles: true, ct);
            return Result.Success();
        }
        finally
        {
            try { File.Delete(tarFile); } catch { /* best-effort cleanup */ }
        }
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

    private static string ToRefName(string branch)
    {
        var hash = Convert.ToHexStringLower(SHA256.HashData(Encoding.UTF8.GetBytes(branch)));
        return $"refs/citadel/{SanitizeRef(branch)}_{hash[..8]}";
    }

    private static string SanitizeRef(string branch)
    {
        return string.Create(branch.Length, branch, static (span, src) =>
        {
            for (var i = 0; i < src.Length; i++)
            {
                var c = src[i];
                span[i] = char.IsLetterOrDigit(c) || c is '-' or '_' or '/' ? c : '_';
            }
        });
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
}
