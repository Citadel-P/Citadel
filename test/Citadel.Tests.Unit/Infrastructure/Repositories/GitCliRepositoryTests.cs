using Hosting.DockerClient.Services;
using Infrastructure.Repositories;
using Domain;
using Domain.Contracts.Resources.Git;
using Domain.Entities.Git;
using System.Diagnostics;

namespace Tests.Unit.Infrastructure.Repositories;

public sealed class GitCliRepositoryTests
{
    [Fact]
    public async Task ResolveSnapshotCommitAsync_Should_Prefer_Fetched_Remote_Branch_Over_Stale_Local_Branch()
    {
        var executor = new CapturingCommandExecutor(args =>
        {
            var candidate = args.Last();
            return candidate switch
            {
                "refs/remotes/origin/main" => new ProcessExecutionResult(0, "fresh-origin\n", ""),
                "refs/heads/main" => new ProcessExecutionResult(0, "old-local\n", ""),
                "main" => new ProcessExecutionResult(0, "old-local\n", ""),
                _ => new ProcessExecutionResult(1, "", "not found")
            };
        });

        var repository = new GitCliRepository(executor);

        var result = await repository.ResolveSnapshotCommitAsync("repo", "main", TestContext.Current.CancellationToken);

        Assert.True(result.IsSuccess(out var commit, out var error), error?.Message);
        Assert.Equal("fresh-origin", commit);
        Assert.Equal("refs/remotes/origin/main", executor.Calls.Single().Last());
    }

    [Fact]
    public async Task ResetWorkingTreeAsync_Should_Checkout_And_Reset_To_Fetched_Remote_Branch()
    {
        var executor = new CapturingCommandExecutor(_ => new ProcessExecutionResult(0, "", ""));
        var repository = new GitCliRepository(executor);

        var result = await repository.ResetWorkingTreeAsync("repo", "main", TestContext.Current.CancellationToken);

        Assert.False(result.IsFailure(out var error), error?.Message);
        Assert.Collection(
            executor.Calls,
            args => Assert.Equal(["-C", "repo", "checkout", "-B", "main", "refs/remotes/origin/main"], args),
            args => Assert.Equal(["-C", "repo", "reset", "--hard", "refs/remotes/origin/main"], args));
    }

    [Fact]
    public async Task ResetWorkingTreeAsync_Should_Stop_When_Checkout_Fails()
    {
        var executor = new CapturingCommandExecutor(args =>
            args.Contains("checkout")
                ? new ProcessExecutionResult(1, "", "checkout failed")
                : new ProcessExecutionResult(0, "", ""));
        var repository = new GitCliRepository(executor);

        var result = await repository.ResetWorkingTreeAsync("repo", "main", TestContext.Current.CancellationToken);

        Assert.True(result.IsFailure(out var error));
        Assert.Equal("ExitCode=1. Error=checkout failed", error.Message);
        Assert.Single(executor.Calls);
    }

    [Fact]
    public async Task RemoveCredentialFile_ShouldDeleteMaterializedSshKey()
    {
        var executor = new CapturingCommandExecutor(
            _ => new ProcessExecutionResult(0, "", ""));
        var repository = new GitCliRepository(executor);
        var account = new GitAccount(
            "test ssh",
            "example.invalid",
            GitTransport.Ssh,
            GitAuthType.SshKey,
            Guid.CreateVersion7(),
            new SshKeyAuth("git", "private-key-material", Passphrase: null));

        try
        {
            var result = await repository.TestConnectionAsync(
                "git@example.invalid:owner/repository.git",
                account,
                TestContext.Current.CancellationToken);

            Assert.True(result.IsSuccess());
            var sshCommand = Assert.IsType<string>(
                executor.LastEnvironment?["GIT_SSH_COMMAND"]);
            var pathStart = sshCommand.IndexOf("-i ", StringComparison.Ordinal) + 3;
            var pathEnd = sshCommand.IndexOf(" -o ", pathStart, StringComparison.Ordinal);
            var keyPath = sshCommand[pathStart..pathEnd];
            Assert.True(File.Exists(keyPath));

            repository.RemoveCredentialFile(account.Id);

            Assert.False(File.Exists(keyPath));
        }
        finally
        {
            repository.RemoveCredentialFile(account.Id);
        }
    }

    [Fact]
    public async Task Browser_Operations_Should_Read_Immutable_Git_Objects()
    {
        using var repositoryDirectory = new TemporaryGitRepository();
        Directory.CreateDirectory(Path.Combine(repositoryDirectory.Path, "config"));
        await File.WriteAllTextAsync(
            Path.Combine(repositoryDirectory.Path, "compose.yaml"),
            "services:\n  web:\n    image: nginx\n",
            TestContext.Current.CancellationToken);
        await File.WriteAllBytesAsync(
            Path.Combine(repositoryDirectory.Path, "config", "binary.dat"),
            [(byte)'A', 0, (byte)'B'],
            TestContext.Current.CancellationToken);
        var firstCommit = repositoryDirectory.Commit("initial");

        var git = new GitCliRepository(new CommandExecutor());
        var resolved = await git.ResolveCommitAsync(
            repositoryDirectory.Path,
            firstCommit,
            TestContext.Current.CancellationToken);
        Assert.True(resolved.IsSuccess(out var resolvedCommit, out var resolveError), resolveError?.Message);
        Assert.Equal(firstCommit, resolvedCommit);

        var root = await git.ListTreeAsync(
            repositoryDirectory.Path,
            firstCommit,
            "",
            maximumEntries: 100,
            maximumOutputBytes: 1024 * 1024,
            TestContext.Current.CancellationToken);
        Assert.True(root.IsSuccess(out var rootTree, out var rootError), rootError?.Message);
        Assert.Contains(rootTree.Entries, entry =>
            entry.Path == "config" && entry.Type == GitRepositoryEntryType.Directory);
        Assert.Contains(rootTree.Entries, entry =>
            entry.Path == "compose.yaml" && entry.Type == GitRepositoryEntryType.File);

        var nested = await git.ListTreeAsync(
            repositoryDirectory.Path,
            firstCommit,
            "config",
            maximumEntries: 100,
            maximumOutputBytes: 1024 * 1024,
            TestContext.Current.CancellationToken);
        Assert.True(nested.IsSuccess(out var nestedTree, out var nestedError), nestedError?.Message);
        var binaryEntry = Assert.Single(nestedTree.Entries);
        Assert.Equal("config/binary.dat", binaryEntry.Path);

        var blob = await git.ReadBlobAsync(
            repositoryDirectory.Path,
            binaryEntry.ObjectId,
            maximumBytes: 1024,
            TestContext.Current.CancellationToken);
        Assert.True(blob.IsSuccess(out var binaryBlob, out var blobError), blobError?.Message);
        Assert.Equal(new byte[] { (byte)'A', 0, (byte)'B' }, binaryBlob.Content.ToArray());

        File.Move(
            Path.Combine(repositoryDirectory.Path, "compose.yaml"),
            Path.Combine(repositoryDirectory.Path, "stack.yaml"));
        await File.WriteAllTextAsync(
            Path.Combine(repositoryDirectory.Path, "README.md"),
            "# stack\n",
            TestContext.Current.CancellationToken);
        var secondCommit = repositoryDirectory.Commit("rename compose");

        var comparison = await git.CompareCommitsAsync(
            repositoryDirectory.Path,
            firstCommit,
            secondCommit,
            maximumEntries: 100,
            maximumOutputBytes: 1024 * 1024,
            TestContext.Current.CancellationToken);
        Assert.True(comparison.IsSuccess(out var changes, out var comparisonError), comparisonError?.Message);
        Assert.Contains(changes.Files, file =>
            file.Status == GitChangedPathStatus.Renamed
            && file.PreviousPath == "compose.yaml"
            && file.Path == "stack.yaml");
        Assert.Contains(changes.Files, file =>
            file.Status == GitChangedPathStatus.Added && file.Path == "README.md");

    }

    private sealed class CapturingCommandExecutor(Func<IReadOnlyList<string>, ProcessExecutionResult> handler)
        : ICommandExecutor
    {
        public List<IReadOnlyList<string>> Calls { get; } = [];
        public IReadOnlyDictionary<string, string>? LastEnvironment { get; private set; }

        public Task<ProcessExecutionResult> ExecuteAsync(
            string fileName,
            IEnumerable<string> arguments,
            IDictionary<string, string>? environmentVariables = null,
            string? workingDirectory = null,
            CancellationToken cancellationToken = default)
        {
            var args = arguments.ToArray();
            Calls.Add(args);
            LastEnvironment = environmentVariables is null
                ? null
                : new Dictionary<string, string>(environmentVariables);
            return Task.FromResult(handler(args));
        }

        public Task<ProcessBinaryExecutionResult> ExecuteBoundedAsync(
            string fileName,
            IEnumerable<string> arguments,
            int maximumStandardOutputBytes,
            int maximumStandardErrorBytes,
            IDictionary<string, string>? environmentVariables = null,
            string? workingDirectory = null,
            CancellationToken cancellationToken = default)
        {
            var args = arguments.ToArray();
            Calls.Add(args);
            var result = handler(args);
            var bytes = System.Text.Encoding.UTF8.GetBytes(result.StandardOutput);
            return Task.FromResult(new ProcessBinaryExecutionResult(
                result.ExitCode,
                bytes.AsMemory(0, Math.Min(bytes.Length, maximumStandardOutputBytes)),
                result.StandardError[..Math.Min(result.StandardError.Length, maximumStandardErrorBytes)],
                bytes.Length > maximumStandardOutputBytes));
        }

        public IAsyncEnumerable<ProcessOutput> StreamAsync(
            string fileName,
            IEnumerable<string> arguments,
            IDictionary<string, string>? environmentVariables = null,
            string? workingDirectory = null,
            string? dockerConfigDirectory = null,
            CancellationToken cancellationToken = default)
            => AsyncEnumerable.Empty<ProcessOutput>();
    }

    private sealed class TemporaryGitRepository : IDisposable
    {
        public TemporaryGitRepository()
        {
            Path = System.IO.Path.Combine(System.IO.Path.GetTempPath(), "citadel-git-browser", Guid.NewGuid().ToString("N"));
            Directory.CreateDirectory(Path);
            Run("init");
            Run("config", "user.email", "citadel-tests@example.invalid");
            Run("config", "user.name", "Citadel Tests");
        }

        public string Path { get; }

        public string Commit(string message)
        {
            Run("add", "-A");
            Run("commit", "-m", message);
            return Run("rev-parse", "HEAD").Trim();
        }

        public void Dispose()
        {
            if (!Directory.Exists(Path))
                return;

            foreach (var file in Directory.EnumerateFiles(Path, "*", SearchOption.AllDirectories))
                File.SetAttributes(file, FileAttributes.Normal);

            foreach (var directory in Directory.EnumerateDirectories(Path, "*", SearchOption.AllDirectories))
                File.SetAttributes(directory, FileAttributes.Normal);

            File.SetAttributes(Path, FileAttributes.Normal);
            Directory.Delete(Path, recursive: true);
        }

        private string Run(params string[] arguments)
        {
            var startInfo = new ProcessStartInfo
            {
                FileName = "git",
                WorkingDirectory = Path,
                RedirectStandardOutput = true,
                RedirectStandardError = true,
                UseShellExecute = false,
                CreateNoWindow = true
            };
            foreach (var argument in arguments)
                startInfo.ArgumentList.Add(argument);

            using var process = Process.Start(startInfo)!;
            var standardOutput = process.StandardOutput.ReadToEnd();
            var standardError = process.StandardError.ReadToEnd();
            process.WaitForExit();
            Assert.True(process.ExitCode == 0, standardError);
            return standardOutput;
        }
    }
}
