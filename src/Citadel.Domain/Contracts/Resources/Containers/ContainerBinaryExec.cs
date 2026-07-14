namespace Domain.Contracts.Resources.Containers;

public sealed record ContainerBinaryExecRequest(
    string ContainerId,
    IReadOnlyList<string> Command,
    IReadOnlyDictionary<string, string>? Environment = null,
    bool AttachStdout = true,
    bool AttachStderr = true,
    bool Tty = false);

public sealed record ContainerBinaryExecChunk(
    ContainerExecStream Stream,
    ReadOnlyMemory<byte> Data);

public enum ContainerExecStream
{
    Stdout,
    Stderr
}

public sealed class ContainerBinaryExecResult : IAsyncDisposable
{
    public required IAsyncEnumerable<ContainerBinaryExecChunk> Output { get; init; }

    public required Func<CancellationToken, Task<int?>> GetExitCodeAsync { get; init; }

    public required Func<ValueTask> CleanupAsync { get; init; }

    public ValueTask DisposeAsync()
        => CleanupAsync();
}
