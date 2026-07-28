using Domain.Contracts.Interfaces;
using Npgsql;
using System.Text;

namespace Infrastructure.Repositories;

internal sealed class PostgresDumpRunner(IResticProcessRunner processRunner) : IPostgresDumpRunner
{
    public async Task<PostgresDumpResult> CreateDumpAsync(
        PostgresDumpCommand command,
        CancellationToken cancellationToken)
    {
        NpgsqlConnectionStringBuilder connection;
        try
        {
            connection = new NpgsqlConnectionStringBuilder(command.ConnectionString);
        }
        catch (ArgumentException)
        {
            return new PostgresDumpResult(-1, null, "The PostgreSQL connection string is invalid.");
        }

        if (string.IsNullOrWhiteSpace(connection.Host)
            || string.IsNullOrWhiteSpace(connection.Database)
            || string.IsNullOrWhiteSpace(connection.Username))
        {
            return new PostgresDumpResult(
                -1,
                null,
                "The PostgreSQL connection string must specify host, database, and username.");
        }

        string serverVersion;
        try
        {
            await using var database = new NpgsqlConnection(command.ConnectionString);
            await database.OpenAsync(cancellationToken);
            await using var versionCommand = new NpgsqlCommand("SHOW server_version", database);
            serverVersion = (await versionCommand.ExecuteScalarAsync(cancellationToken))?.ToString()
                ?? "unknown";
        }
        catch (OperationCanceledException)
        {
            throw;
        }
        catch (Exception ex)
        {
            return new PostgresDumpResult(
                -1,
                null,
                $"Citadel could not connect to PostgreSQL: {Sanitize(ex.Message, connection.Password, command.MaxErrorBytes)}");
        }

        var errors = new StringBuilder(Math.Min(command.MaxErrorBytes, 4096));
        var exitCode = -1;
        await foreach (var item in processRunner.RunAsync(
            BuildProcessCommand(command, connection),
            cancellationToken))
        {
            if (item.ExitCode.HasValue)
            {
                exitCode = item.ExitCode.Value;
                continue;
            }

            if (item.Stream == ResticProcessStream.StdErr && !string.IsNullOrWhiteSpace(item.Message))
                AppendBounded(errors, item.Message, command.MaxErrorBytes);
        }

        if (exitCode == 0 && File.Exists(command.OutputPath))
            return new PostgresDumpResult(0, serverVersion, null);

        var error = BuildProcessError(command.ExecutablePath, exitCode, errors.ToString());
        return new PostgresDumpResult(exitCode, serverVersion, error);
    }

    internal static string BuildProcessError(
        string executablePath,
        int exitCode,
        string standardError)
    {
        if (exitCode == -1
            && (standardError.Contains("No such file or directory", StringComparison.OrdinalIgnoreCase)
                || standardError.Contains("The system cannot find the file specified", StringComparison.OrdinalIgnoreCase)))
        {
            return $"Citadel could not start the PostgreSQL dump executable \"{executablePath}\". "
                   + "Install PostgreSQL client tools or configure Backups:PostgresDumpPath.";
        }

        if (!string.IsNullOrWhiteSpace(standardError))
            return standardError;

        return exitCode == -2
            ? "The PostgreSQL dump timed out."
            : $"pg_dump exited with code {exitCode}.";
    }

    internal static ResticProcessCommand BuildProcessCommand(
        PostgresDumpCommand command,
        NpgsqlConnectionStringBuilder connection)
    {
        var redactionValues = string.IsNullOrWhiteSpace(connection.Password)
            ? Array.Empty<string>()
            : new[] { connection.Password };
        var arguments = new[]
        {
            "--format=custom",
            "--no-owner",
            "--no-privileges",
            $"--file={command.OutputPath}"
        };

        return new ResticProcessCommand(
            command.ExecutablePath,
            arguments,
            BuildEnvironment(connection),
            command.WorkingDirectory,
            command.Timeout,
            redactionValues,
            Math.Max(1024, Math.Min(command.MaxErrorBytes, 16 * 1024)),
            OperationName: "PostgreSQL dump");
    }

    private static Dictionary<string, string> BuildEnvironment(NpgsqlConnectionStringBuilder connection)
    {
        var environment = new Dictionary<string, string>(StringComparer.Ordinal)
        {
            ["PGHOST"] = connection.Host!,
            ["PGPORT"] = connection.Port.ToString(),
            ["PGDATABASE"] = connection.Database!,
            ["PGUSER"] = connection.Username!,
            ["PGSSLMODE"] = ToLibPqSslMode(connection.SslMode)
        };

        AddIfConfigured(environment, "PGPASSWORD", connection.Password);
        AddIfConfigured(environment, "PGPASSFILE", connection.Passfile);
        AddIfConfigured(environment, "PGSSLCERT", connection.SslCertificate);
        AddIfConfigured(environment, "PGSSLKEY", connection.SslKey);
        AddIfConfigured(environment, "PGSSLROOTCERT", connection.RootCertificate);
        return environment;
    }

    private static string ToLibPqSslMode(SslMode mode)
        => mode switch
        {
            SslMode.Disable => "disable",
            SslMode.Allow => "allow",
            SslMode.Prefer => "prefer",
            SslMode.Require => "require",
            SslMode.VerifyCA => "verify-ca",
            SslMode.VerifyFull => "verify-full",
            _ => "prefer"
        };

    private static void AddIfConfigured(
        IDictionary<string, string> environment,
        string key,
        string? value)
    {
        if (!string.IsNullOrWhiteSpace(value))
            environment[key] = value;
    }

    private static void AppendBounded(StringBuilder builder, string value, int maxBytes)
    {
        var maxChars = Math.Max(256, maxBytes / 2);
        if (builder.Length >= maxChars)
            return;

        if (builder.Length > 0)
            builder.AppendLine();

        var remaining = maxChars - builder.Length;
        builder.Append(value.AsSpan(0, Math.Min(value.Length, remaining)));
    }

    private static string Sanitize(string message, string? password, int maxBytes)
    {
        var sanitized = string.IsNullOrEmpty(password)
            ? message
            : message.Replace(password, "********", StringComparison.Ordinal);
        var maxChars = Math.Max(256, maxBytes / 2);
        return sanitized.Length <= maxChars ? sanitized : sanitized[..maxChars] + "...";
    }
}
