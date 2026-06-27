using Hosting.DockerClient.Services;
using Infrastructure.Repositories;

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
        Assert.Equal("checkout failed", error.Message);
        Assert.Single(executor.Calls);
    }

    private sealed class CapturingCommandExecutor(Func<IReadOnlyList<string>, ProcessExecutionResult> handler)
        : ICommandExecutor
    {
        public List<IReadOnlyList<string>> Calls { get; } = [];

        public Task<ProcessExecutionResult> ExecuteAsync(
            string fileName,
            IEnumerable<string> arguments,
            IDictionary<string, string>? environmentVariables = null,
            string? workingDirectory = null,
            CancellationToken cancellationToken = default)
        {
            var args = arguments.ToArray();
            Calls.Add(args);
            return Task.FromResult(handler(args));
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
}
