using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Stacks;
using Domain.Entities.Stacks;
using Hosting.DockerClient.Services;
using System.Runtime.CompilerServices;
using System.Runtime.InteropServices;
using System.Text;

namespace Infrastructure.Connectors.LocalConnectors;

internal sealed class LocalStackConnector(ICommandExecutor commandExecutor) : IStackConnector
{
    private const string DockerExecutable = "docker";
    private const string ComposeFileName = "compose.yml";
    private const string DockerConfigDirectory = "docker-config";
    private static readonly string CitadelTempDir = Path.Combine(Path.GetTempPath(), "citadel", "stacks");

    public async IAsyncEnumerable<StackApplyResult> StackApplyAsync(StackApplyCommand applyCommand, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var result in ExecuteApplyAsync(applyCommand, cancellationToken))
        {
            yield return result;
        }
    }

    private async IAsyncEnumerable<StackApplyResult> ExecuteApplyAsync(StackApplyCommand applyCommand, [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var workingDirectory = Path.Combine(CitadelTempDir, SanitizeSegment(applyCommand.StackName), Guid.NewGuid().ToString("N"));

        Directory.CreateDirectory(workingDirectory);

        try
        {
            var composeFilePath = Path.Combine(workingDirectory, ComposeFileName);

            await File.WriteAllTextAsync(
                composeFilePath,
                applyCommand.ComposeFileContent,
                Encoding.UTF8,
                cancellationToken);

            string? envFilePath = null;

            if (applyCommand.EnvironmentVariables is { Count: > 0 })
            {
                envFilePath = Path.Combine(
                    workingDirectory,
                    string.IsNullOrWhiteSpace(applyCommand.EnvironmentFilePath)
                        ? ".env"
                        : applyCommand.EnvironmentFilePath);

                var envDir = Path.GetDirectoryName(envFilePath);

                if (!string.IsNullOrWhiteSpace(envDir))
                {
                    Directory.CreateDirectory(envDir);
                }

                await File.WriteAllLinesAsync(
                    envFilePath,
                    applyCommand.EnvironmentVariables,
                    Encoding.UTF8,
                    cancellationToken);
            }

            var processEnvironment = BuildProcessEnvironment(applyCommand.EnvironmentVariables);

            if (applyCommand.PreDeploy is not null)
            {
                yield return StackApplyResult.SystemMessage("Running pre-deploy commands...");

                await foreach (var result in RunStackCommandAsync(
                    applyCommand.PreDeploy,
                    workingDirectory,
                    processEnvironment,
                    cancellationToken))
                {
                    yield return result;

                    if (result.Type == StackApplyEventType.CommandCompleted && result.ExitCode != 0)
                    {
                        yield break;
                    }
                }
            }

            string? dockerConfigDirectory = null;

            if (applyCommand.RegistryAuth is not null)
            {
                yield return StackApplyResult.SystemMessage($"Setting up registry config for: {applyCommand.RegistryName}...");

                dockerConfigDirectory = Path.Combine(
                    workingDirectory,
                    DockerConfigDirectory);

                Directory.CreateDirectory(dockerConfigDirectory);

                await File.WriteAllTextAsync(
                    Path.Combine(dockerConfigDirectory, "config.json"),
                    applyCommand.RegistryAuth,
                    Encoding.UTF8,
                    cancellationToken);
            }

            await foreach (var result in RunDockerComposeAsync(
                composeFilePath,
                envFilePath,
                applyCommand.ProjectName,
                workingDirectory,
                processEnvironment,
                dockerConfigDirectory,
                cancellationToken))
            {
                yield return result;

                if (result.Type == StackApplyEventType.CommandCompleted)
                {
                    if (result.ExitCode != 0)
                    {
                        yield break;
                    }
                }
            }

            if (applyCommand.PostDeploy is not null)
            {
                yield return StackApplyResult.SystemMessage("Running post-deploy commands...");
                await foreach (var result in RunStackCommandAsync(
                    applyCommand.PostDeploy,
                    workingDirectory,
                    processEnvironment,
                    cancellationToken))
                {
                    yield return result;

                    if (result.Type == StackApplyEventType.CommandCompleted && result.ExitCode != 0)
                    {
                        yield break;
                    }
                }
            }

        }
        finally
        {
            try
            {
                Directory.Delete(workingDirectory, true);
            }
            catch { }
        }
    }

    private async IAsyncEnumerable<StackApplyResult> RunDockerComposeAsync(
        string composeFilePath,
        string? envFilePath,
        string? projectName,
        string workingDirectory,
        Dictionary<string, string> environmentVariables,
        string? dockerConfigDirectory,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        await foreach (var output in commandExecutor.StreamAsync(
            DockerExecutable,
            BuildComposeArguments(composeFilePath, envFilePath, projectName),
            environmentVariables,
            workingDirectory,
            dockerConfigDirectory,
            cancellationToken))
        {
            if (!string.IsNullOrWhiteSpace(output.StdOut))
            {
                yield return StackApplyResult.StdOut(output.StdOut);
            }

            if (!string.IsNullOrWhiteSpace(output.StdErr))
            {
                yield return StackApplyResult.StdErr(output.StdErr);
            }

            if (output.ExitCode is int exitCode)
            {
                yield return StackApplyResult.Finished(exitCode);
            }
        }
    }

    private static string[] BuildComposeArguments(string composeFilePath, string? envFilePath, string? projectName)
    {
        var args = new List<string>
        {
            "compose",
            "-f",
            composeFilePath
        };

        if (!string.IsNullOrWhiteSpace(envFilePath))
        {
            args.Add("--env-file");
            args.Add(envFilePath);
        }

        if (!string.IsNullOrWhiteSpace(projectName))
        {
            args.Add("--project-name");
            args.Add(projectName);
        }

        args.Add("up");
        args.Add("-d");

        return [.. args];
    }

    private static Dictionary<string, string> BuildProcessEnvironment(IReadOnlyList<string>? environmentVariables)
    {
        var env = new Dictionary<string, string>(StringComparer.Ordinal);
        if (environmentVariables is null)
        {
            return env;
        }

        foreach (var variable in environmentVariables)
        {
            if (string.IsNullOrWhiteSpace(variable))
            {
                continue;
            }

            var separatorIndex = variable.IndexOf('=');
            if (separatorIndex <= 0)
            {
                continue;
            }

            env[variable[..separatorIndex]] = variable[(separatorIndex + 1)..];
        }

        return env;
    }

    private async IAsyncEnumerable<StackApplyResult> RunStackCommandAsync(
        StackCommand stackCommand,
        string rootWorkingDirectory,
        Dictionary<string, string> environmentVariables,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var commandWorkingDirectory = ResolveCommandWorkingDirectory(rootWorkingDirectory, stackCommand.Path);

        if (!Directory.Exists(commandWorkingDirectory))
        {
            yield return StackApplyResult.StdErr($"Command path '{stackCommand.Path}' does not exist.");

            yield break;
        }

        foreach (var command in stackCommand.Commands)
        {
            if (string.IsNullOrWhiteSpace(command))
            {
                continue;
            }

            await foreach (var result in RunShellCommandAsync(
                command,
                commandWorkingDirectory,
                environmentVariables,
                cancellationToken))
            {
                yield return result;

                if (result.Type == StackApplyEventType.CommandCompleted && result.ExitCode != 0)
                {
                    yield break;
                }
            }
        }
    }

    private async IAsyncEnumerable<StackApplyResult> RunShellCommandAsync(
        string command,
        string workingDirectory,
        IDictionary<string, string> environmentVariables,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var (fileName, arguments) = BuildShellInvocation(command);

        await foreach (var output in commandExecutor.StreamAsync(
            fileName,
            arguments,
            environmentVariables,
            workingDirectory,
            cancellationToken: cancellationToken))
        {
            if (!string.IsNullOrWhiteSpace(output.StdOut))
            {
                yield return StackApplyResult.StdOut(output.StdOut);
            }

            if (!string.IsNullOrWhiteSpace(output.StdErr))
            {
                yield return StackApplyResult.StdErr(output.StdErr);
            }

            if (output.ExitCode is int exitCode)
            {
                yield return StackApplyResult.Finished(exitCode);
            }
        }
    }

    private static string ResolveCommandWorkingDirectory(string rootWorkingDirectory, string? path)
    {
        if (string.IsNullOrWhiteSpace(path) || path == "./")
        {
            return rootWorkingDirectory;
        }

        var fullPath = Path.GetFullPath(Path.IsPathRooted(path)
            ? path
            : Path.Combine(rootWorkingDirectory, path));

        // Guard against directory traversal
        var root = Path.GetFullPath(rootWorkingDirectory);
        if (!root.EndsWith(Path.DirectorySeparatorChar))
        {
            root += Path.DirectorySeparatorChar;
        }

        if (!fullPath.StartsWith(root, StringComparison.OrdinalIgnoreCase))
        {
            throw new InvalidOperationException("Path escapes stack directory.");
        }

        return fullPath;
    }

    private static (string FileName, string[] Arguments) BuildShellInvocation(string command)
        => RuntimeInformation.IsOSPlatform(OSPlatform.Windows)
            ? ("powershell", ["-NoProfile", "-NonInteractive", "-Command", command])
            : ("/bin/sh", ["-lc", command]);

    private static string SanitizeSegment(string value)
    {
        var invalidChars = Path.GetInvalidFileNameChars();
        var builder = new StringBuilder(value.Length);
        foreach (var c in value)
        {
            builder.Append(Array.IndexOf(invalidChars, c) >= 0 ? '_' : c);
        }

        return builder.Length == 0 ? "stack" : builder.ToString();
    }
}
