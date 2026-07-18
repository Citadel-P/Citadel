using Domain.Contracts.Interfaces;
using Hosting.DockerClient.Services;
using System.Runtime.CompilerServices;
using System.Text.Json;
using System.Text.RegularExpressions;
using System.Text;

namespace Infrastructure.Repositories;

internal sealed partial class BuildProcessRunner(ICommandExecutor commandExecutor) : IBuildProcessRunner
{
    public async IAsyncEnumerable<BuildProcessEvent> RunAsync(
        BuildProcessCommand command,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        var tempRoot = Path.Combine(Path.GetTempPath(), "citadel-builds", Guid.NewGuid().ToString("N"));
        var dockerConfigDirectory = PrepareDockerConfig(command, tempRoot);
        var secretFiles = PrepareSecretFiles(command, tempRoot);
        var environment = new Dictionary<string, string>(StringComparer.Ordinal)
        {
            ["DOCKER_BUILDKIT"] = "1"
        };

        using var timeoutCts = new CancellationTokenSource(command.Timeout);
        using var linkedCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken, timeoutCts.Token);
        var ct = linkedCts.Token;
        string? digest = null;

        try
        {
            yield return new BuildProcessEvent(BuildProcessStream.StdOut, "Starting Docker build.");
            var buildExitCode = 0;
            await foreach (var output in commandExecutor.StreamAsync(
                               "docker",
                               BuildArgs(command, secretFiles),
                               environment,
                               command.WorkingDirectory,
                               dockerConfigDirectory,
                               ct))
            {
                if (output.StdOut is not null)
                    yield return new BuildProcessEvent(BuildProcessStream.StdOut, Sanitize(output.StdOut, command));

                if (output.StdErr is not null)
                    yield return new BuildProcessEvent(BuildProcessStream.StdErr, Sanitize(output.StdErr, command));

                if (output.ExitCode.HasValue)
                    buildExitCode = output.ExitCode.Value;
            }

            if (buildExitCode != 0)
            {
                yield return new BuildProcessEvent(BuildProcessStream.Exit, ExitCode: buildExitCode);
                yield break;
            }

            foreach (var imageReference in command.ImageReferences)
            {
                yield return new BuildProcessEvent(BuildProcessStream.StdOut, $"Pushing {imageReference}.");
                var pushExitCode = 0;
                await foreach (var output in commandExecutor.StreamAsync(
                                   "docker",
                                   ["push", imageReference],
                                   environment,
                                   command.WorkingDirectory,
                                   dockerConfigDirectory,
                                   ct))
                {
                    if (output.StdOut is not null)
                    {
                        var message = Sanitize(output.StdOut, command);
                        digest ??= TryParseDigest(message);
                        yield return new BuildProcessEvent(BuildProcessStream.StdOut, message);
                    }

                    if (output.StdErr is not null)
                    {
                        var message = Sanitize(output.StdErr, command);
                        digest ??= TryParseDigest(message);
                        yield return new BuildProcessEvent(BuildProcessStream.StdErr, message);
                    }

                    if (output.ExitCode.HasValue)
                        pushExitCode = output.ExitCode.Value;
                }

                if (pushExitCode != 0)
                {
                    yield return new BuildProcessEvent(BuildProcessStream.Exit, ExitCode: pushExitCode, Digest: digest);
                    yield break;
                }
            }

            yield return new BuildProcessEvent(BuildProcessStream.Exit, ExitCode: 0, Digest: digest);
        }
        finally
        {
            TryDelete(tempRoot);
        }
    }

    private static IReadOnlyList<string> BuildArgs(
        BuildProcessCommand command,
        IReadOnlyDictionary<string, string> secretFiles)
    {
        var args = new List<string>
        {
            "build",
            "--progress=plain",
            "-f",
            command.DockerfilePath
        };

        if (!string.IsNullOrWhiteSpace(command.Target))
        {
            args.Add("--target");
            args.Add(command.Target);
        }

        foreach (var imageReference in command.ImageReferences)
        {
            args.Add("-t");
            args.Add(imageReference);
        }

        foreach (var buildArg in command.BuildArgs)
        {
            args.Add("--build-arg");
            args.Add($"{buildArg.Name}={buildArg.Value}");
        }

        foreach (var kv in secretFiles)
        {
            args.Add("--secret");
            args.Add($"id={kv.Key},src={kv.Value}");
        }

        args.Add(command.ContextPath);
        return args;
    }

    private static string? PrepareDockerConfig(BuildProcessCommand command, string tempRoot)
    {
        if (command.RegistryCredential is null)
            return null;

        var credential = command.RegistryCredential;
        var dockerConfigDirectory = Path.Combine(tempRoot, "docker-config");
        Directory.CreateDirectory(dockerConfigDirectory);
        var auth = Convert.ToBase64String(Encoding.UTF8.GetBytes($"{credential.UserName}:{credential.Password}"));
        var config = "{\"auths\":{\""
                     + EscapeJson(credential.RegistryHost)
                     + "\":{\"auth\":\""
                     + EscapeJson(auth)
                     + "\"}}}";
        File.WriteAllText(Path.Combine(dockerConfigDirectory, "config.json"), config);
        return dockerConfigDirectory;
    }

    private static IReadOnlyDictionary<string, string> PrepareSecretFiles(BuildProcessCommand command, string tempRoot)
    {
        if (command.Secrets.Count == 0)
            return new Dictionary<string, string>();

        var secretDirectory = Path.Combine(tempRoot, "secrets");
        Directory.CreateDirectory(secretDirectory);
        var files = new Dictionary<string, string>(StringComparer.Ordinal);
        foreach (var secret in command.Secrets)
        {
            var path = Path.Combine(secretDirectory, SanitizeFileName(secret.Id));
            File.WriteAllText(path, secret.Value);
            files[secret.Id] = path;
        }

        return files;
    }

    private static string Sanitize(string value, BuildProcessCommand command)
    {
        var sanitized = AnsiRegex().Replace(value, string.Empty);
        var redactionValues = command.Secrets
            .Select(static x => x.Value)
            .Concat(command.BuildArgs.Select(static x => x.Value))
            .Append(command.RegistryCredential?.Password ?? string.Empty)
            .Where(static x => !string.IsNullOrEmpty(x))
            .Distinct(StringComparer.Ordinal)
            .OrderByDescending(static x => x.Length);

        foreach (var secret in redactionValues)
        {
            sanitized = sanitized.Replace(secret, "********", StringComparison.Ordinal);
        }

        return sanitized.Length <= command.MaxLineBytes
            ? sanitized
            : sanitized[..command.MaxLineBytes] + "...";
    }

    private static string? TryParseDigest(string message)
    {
        var match = DigestRegex().Match(message);
        return match.Success ? match.Value : null;
    }

    private static string SanitizeFileName(string value)
        => string.Concat(value.Select(static ch => char.IsLetterOrDigit(ch) || ch is '-' or '_' or '.' ? ch : '_'));

    private static string EscapeJson(string value)
        => JsonEncodedText.Encode(value).ToString();

    private static void TryDelete(string path)
    {
        try
        {
            if (Directory.Exists(path))
                Directory.Delete(path, recursive: true);
        }
        catch
        {
        }
    }

    [GeneratedRegex(@"\x1B\[[0-?]*[ -/]*[@-~]", RegexOptions.Compiled)]
    private static partial Regex AnsiRegex();

    [GeneratedRegex(@"sha256:[a-fA-F0-9]{64}", RegexOptions.Compiled)]
    private static partial Regex DigestRegex();
}
