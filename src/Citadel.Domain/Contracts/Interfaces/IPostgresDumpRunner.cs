namespace Domain.Contracts.Interfaces;

public interface IPostgresDumpRunner
{
    Task<PostgresDumpResult> CreateDumpAsync(
        PostgresDumpCommand command,
        CancellationToken cancellationToken);
}

public sealed record PostgresDumpCommand(
    string ExecutablePath,
    string ConnectionString,
    string OutputPath,
    string WorkingDirectory,
    TimeSpan Timeout,
    int MaxErrorBytes);

public sealed record PostgresDumpResult(
    int ExitCode,
    string? ServerVersion,
    string? ErrorMessage)
{
    public bool Succeeded => ExitCode == 0;
}
