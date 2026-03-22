using System.Collections.Concurrent;
using System.Runtime.InteropServices;
using System.Security.AccessControl;
using System.Security.Principal;
using Domain.Contracts.Interfaces;
using Domain.Entities.Git;
using Hosting.DockerClient.Services;
using Infrastructure.Repositories.Mappers;
using LightResults;

namespace Infrastructure.Repositories;

internal class GitCliRepository(IProcessService processService) : IGitCliRepository
{
    private const string GitExecutable = "git";

    private static readonly Dictionary<string, string> GitEnv = new()
    {
        { "GIT_TERMINAL_PROMPT", "0" },
        { "GIT_ASKPASS", "echo" },
        { "GIT_SSH_COMMAND", "ssh -o BatchMode=yes" } // Prevents SSH from hanging on unknown hosts
    };

    // accountId -> hash of last-written private key; rewrites only when the key changes
    private static readonly ConcurrentDictionary<Guid, int> SshKeyCache = new();

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

    public async Task<Result> FetchAsync(string repoPath, string branch, GitAccount? account, CancellationToken ct = default)
    {
        var (args, env) = PrepareRemoteCmd(account);
        args.Add("-C");
        args.Add(repoPath);
        args.Add("fetch");
        args.Add("origin");
        args.Add($"{branch}:fallback_branch");
        args.Add("--prune");
        args.Add("--depth=1");

        var result = await processService.ExecuteAsync(GitExecutable, args, env, ct);
        return result.Map();
    }

    public async Task<Result<string>> ResolveCommitHashAsync(string repoPath, string branch, CancellationToken ct = default)
    {
        var args = new[] { "-C", repoPath, "rev-parse", "FETCH_HEAD" };
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
        // Ensure target directory exists
        Directory.CreateDirectory(targetPath);

        // We use 'sh -c' to handle the pipe from git archive to tar
        // This avoids creating a temporary .tar file on disk
        var command = $"git --git-dir={repoPath} archive {commitHash} | tar -x -C {targetPath}";

        var result = await processService.ExecuteAsync("sh", ["-c", command], GitEnv, ct);

        return result.Map();
    }

    private static (List<string> Args, Dictionary<string, string> Env) PrepareRemoteCmd(GitAccount? account)
    {
        var args = new List<string>
        {
            // Disable interactive prompts globally for this command
            "-c",
            "core.askPass=echo"
        };

        var env = new Dictionary<string, string>(GitEnv);

        if (account?.Configuration is GitHttpAccount http && !string.IsNullOrEmpty(http.Token))
        {
            var helper = $"!f() {{ echo \"username={http.Username}\"; echo \"password={http.Token}\"; }}; f";
            args.Add("-c");
            args.Add($"credential.helper={helper}");
        }
        else if (account?.Configuration is GitSshAccount ssh)
        {
            var keyFile = GetOrWriteSshKey(account.Id, ssh.PrivateKey);
            var sshCmd = $"ssh -i {keyFile} -o IdentitiesOnly=yes -o StrictHostKeyChecking=no -o BatchMode=yes";

            if (!string.IsNullOrEmpty(ssh.Username))
                sshCmd += $" -l {ssh.Username}";

            env["GIT_SSH_COMMAND"] = sshCmd;
        }

        return (args, env);
    }

    private static string GetOrWriteSshKey(Guid accountId, string privateKey)
    {
        var keyFile = Path.Combine(Path.GetTempPath(), $"citadel_ssh_{accountId:N}");
        var contentHash = privateKey.GetHashCode();

        if (SshKeyCache.TryGetValue(accountId, out var cachedHash) && cachedHash == contentHash && File.Exists(keyFile))
            return keyFile;

        File.WriteAllText(keyFile, privateKey.TrimEnd() + "\n");
        SetKeyFilePermissions(keyFile);
        SshKeyCache[accountId] = contentHash;

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
}
