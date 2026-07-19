using Domain.Contracts.Interfaces;
using Hosting.DockerClient.Models.Images;
using Hosting.DockerClient.Services;
using System.Runtime.CompilerServices;
using System.Text.RegularExpressions;

namespace Infrastructure.Repositories;

internal sealed partial class BuildProcessRunner(IImageService imageService) : IBuildProcessRunner
{
    public async IAsyncEnumerable<BuildProcessEvent> RunAsync(
        BuildProcessCommand command,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        if (command.Secrets.Count > 0)
        {
            yield return new BuildProcessEvent(
                BuildProcessStream.StdErr,
                "Build secrets are not supported by Docker Engine API builds yet. Remove build secrets or use a BuildKit-native builder.");
            yield return new BuildProcessEvent(BuildProcessStream.Exit, ExitCode: 1);
            yield break;
        }

        using var timeoutCts = new CancellationTokenSource(command.Timeout);
        using var linkedCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken, timeoutCts.Token);
        var ct = linkedCts.Token;

        string? digest = null;

        yield return new BuildProcessEvent(BuildProcessStream.StdOut, "Starting Docker build.");
        var buildFailed = false;
        await foreach (var message in imageService.StreamBuildImage(ToBuildImageCommand(command), ct))
        {
            foreach (var item in MapMessage(message, command))
            {
                if (item.Stream is BuildProcessStream.StdErr)
                    buildFailed = true;
                yield return item;
            }
        }

        if (buildFailed)
        {
            yield return new BuildProcessEvent(BuildProcessStream.Exit, ExitCode: 1);
            yield break;
        }

        foreach (var imageReference in command.ImageReferences)
        {
            yield return new BuildProcessEvent(BuildProcessStream.StdOut, $"Pushing {imageReference}.");
            var pushFailed = false;
            await foreach (var message in imageService.StreamPushImage(ToPushImageCommand(command, imageReference), ct))
            {
                foreach (var item in MapMessage(message, command))
                {
                    if (item.Message is not null)
                        digest ??= TryParseDigest(item.Message);
                    if (item.Stream is BuildProcessStream.StdErr)
                        pushFailed = true;
                    yield return item;
                }
            }

            if (pushFailed)
            {
                yield return new BuildProcessEvent(BuildProcessStream.Exit, ExitCode: 1, Digest: digest);
                yield break;
            }
        }

        yield return new BuildProcessEvent(BuildProcessStream.Exit, ExitCode: 0, Digest: digest);
    }

    private static BuildImageStreamCommand ToBuildImageCommand(BuildProcessCommand command)
        => new(
            command.ContextPath,
            command.DockerfilePath,
            command.ImageReferences,
            command.BuildArgs.ToDictionary(static x => x.Name, static x => x.Value, StringComparer.Ordinal),
            command.Target,
            command.RegistryCredential?.RegistryAuth,
            command.RegistryCredential?.RegistryHost,
            command.Timeout,
            command.MaxLineBytes);

    private static PushImageStreamCommand ToPushImageCommand(BuildProcessCommand command, string imageReference)
        => new(imageReference, command.RegistryCredential?.RegistryAuth);

    private static IEnumerable<BuildProcessEvent> MapMessage(Hosting.DockerClient.HttpClient.JSONMessage message, BuildProcessCommand command)
    {
        if (!string.IsNullOrWhiteSpace(message.ErrorMessage) || message.Error is not null)
        {
            yield return new BuildProcessEvent(
                BuildProcessStream.StdErr,
                Sanitize(message.ErrorMessage ?? message.Error?.Message ?? "Docker API returned an error.", command));
            yield break;
        }

        if (!string.IsNullOrWhiteSpace(message.Stream))
            yield return new BuildProcessEvent(BuildProcessStream.StdOut, Sanitize(message.Stream.TrimEnd(), command));

        if (!string.IsNullOrWhiteSpace(message.Status))
        {
            var line = string.IsNullOrWhiteSpace(message.ID)
                ? message.Status
                : $"{message.ID}: {message.Status}";

            if (!string.IsNullOrWhiteSpace(message.ProgressMessage))
                line += $" {message.ProgressMessage}";

            yield return new BuildProcessEvent(BuildProcessStream.StdOut, Sanitize(line, command));
        }
    }

    private static string Sanitize(string value, BuildProcessCommand command)
    {
        var sanitized = AnsiRegex().Replace(value, string.Empty).Replace("\0", string.Empty, StringComparison.Ordinal);
        var redactionValues = command.Secrets
            .Select(static x => x.Value)
            .Concat(command.BuildArgs.Select(static x => x.Value))
            .Append(command.RegistryCredential?.RegistryAuth ?? string.Empty)
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

    [GeneratedRegex(@"\x1B\[[0-?]*[ -/]*[@-~]", RegexOptions.Compiled)]
    private static partial Regex AnsiRegex();

    [GeneratedRegex(@"sha256:[a-fA-F0-9]{64}", RegexOptions.Compiled)]
    private static partial Regex DigestRegex();
}
