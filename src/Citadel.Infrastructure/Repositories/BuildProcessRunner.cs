using Domain;
using Domain.Contracts.Interfaces;
using Domain.Contracts.Resources.Images;
using Infrastructure.EdgeAgents;
using Hosting.DockerClient.Services;
using System.Runtime.CompilerServices;
using System.Text.RegularExpressions;

namespace Infrastructure.Repositories;

internal sealed partial class BuildProcessRunner(IConnectorFactory<IImageConnector> imageConnectorFactory) : IBuildProcessRunner
{
    private const int MinUnboundedRedactionLength = 8;

    public async IAsyncEnumerable<BuildProcessEvent> RunAsync(
        BuildProcessCommand command,
        [EnumeratorCancellation] CancellationToken cancellationToken)
    {
        using var timeoutCts = new CancellationTokenSource(command.Timeout);
        using var linkedCts = CancellationTokenSource.CreateLinkedTokenSource(cancellationToken, timeoutCts.Token);
        var ct = linkedCts.Token;
        var redactionValues = PrepareRedactionValues(command);

        string? digest = null;
        var imageConnector = imageConnectorFactory.GetConnector(command.PlatformConnectorType);
        (byte[] Archive, DockerBuildContext Context)? contextArchive = null;
        if (RequiresPackagedContext(command.PlatformConnectorType))
        {
            string? sizeError = null;
            try
            {
                contextArchive = await BuildContextArchive.CreateBytesAsync(
                    command.ContextPath,
                    command.DockerfilePath,
                    EdgeAgentDefaults.MaxEnvelopePayloadBytes,
                    ct);
            }
            catch (BuildContextSizeLimitExceededException ex)
            {
                sizeError = ex.Message;
            }

            if (sizeError is not null)
            {
                yield return new BuildProcessEvent(
                    BuildProcessStream.StdErr,
                    sizeError);
                yield return new BuildProcessEvent(BuildProcessStream.Exit, ExitCode: 1);
                yield break;
            }
        }

        yield return new BuildProcessEvent(BuildProcessStream.StdOut, "Starting Docker build.");
        var buildFailed = false;
        await foreach (var message in imageConnector.BuildImageProgressStreamAsync(ToBuildImageCommand(command, contextArchive), ct))
        {
            foreach (var item in MapMessage(message, command.MaxLineBytes, redactionValues))
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
            await foreach (var message in imageConnector.PushImageProgressStreamAsync(ToPushImageCommand(command, imageReference), ct))
            {
                foreach (var item in MapMessage(message, command.MaxLineBytes, redactionValues))
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

    private static BuildImageCommand ToBuildImageCommand(
        BuildProcessCommand command,
        (byte[] Archive, DockerBuildContext Context)? contextArchive)
        => new(
            command.PlatformAddress,
            command.ContextPath,
            command.DockerfilePath,
            command.ImageReferences,
            command.BuildArgs.ToDictionary(static x => x.Name, static x => x.Value, StringComparer.Ordinal),
            command.Target,
            command.RegistryCredential?.RegistryAuth,
            command.RegistryCredential?.RegistryHost,
            command.Timeout,
            command.MaxLineBytes,
            contextArchive?.Archive,
            contextArchive?.Context.DockerfileEntryName,
            [.. command.Secrets.Select(static secret => new BuildImageSecret(secret.Id, secret.Value))]);

    private static bool RequiresPackagedContext(PlatformConnectorType connectorType)
        => connectorType is PlatformConnectorType.Agent or PlatformConnectorType.EdgeAgent;

    private static PushImageCommand ToPushImageCommand(BuildProcessCommand command, string imageReference)
        => new(command.PlatformAddress, imageReference, command.RegistryCredential?.RegistryAuth);

    private static IEnumerable<BuildProcessEvent> MapMessage(
        ImageBuildStreamItem message,
        int maxLineBytes,
        IReadOnlyList<string> redactionValues)
    {
        if (!string.IsNullOrWhiteSpace(message.ErrorMessage) || message.Error is not null)
        {
            yield return new BuildProcessEvent(
                BuildProcessStream.StdErr,
                Sanitize(
                    message.ErrorMessage ?? message.Error?.Message ?? "Docker API returned an error.",
                    maxLineBytes,
                    redactionValues));
            yield break;
        }

        if (!string.IsNullOrWhiteSpace(message.Stream))
            yield return new BuildProcessEvent(
                BuildProcessStream.StdOut,
                Sanitize(message.Stream.TrimEnd(), maxLineBytes, redactionValues));

        if (!string.IsNullOrWhiteSpace(message.Status))
        {
            var line = string.IsNullOrWhiteSpace(message.Id)
                ? message.Status
                : $"{message.Id}: {message.Status}";

            if (!string.IsNullOrWhiteSpace(message.ProgressMessage))
                line += $" {message.ProgressMessage}";

            yield return new BuildProcessEvent(
                BuildProcessStream.StdOut,
                Sanitize(line, maxLineBytes, redactionValues));
        }
    }

    private static string Sanitize(
        string value,
        int maxLineBytes,
        IReadOnlyList<string> redactionValues)
    {
        var sanitized = AnsiRegex().Replace(value, string.Empty).Replace("\0", string.Empty, StringComparison.Ordinal);

        foreach (var secret in redactionValues)
        {
            sanitized = RedactValue(sanitized, secret);
        }

        return sanitized.Length <= maxLineBytes
            ? sanitized
            : sanitized[..maxLineBytes] + "...";
    }

    private static string[] PrepareRedactionValues(BuildProcessCommand command)
        => [.. command.Secrets
            .Select(static x => x.Value)
            .Concat(command.BuildArgs.Select(static x => x.Value))
            .Append(command.RegistryCredential?.RegistryAuth ?? string.Empty)
            .Where(static x => !string.IsNullOrEmpty(x))
            .Distinct(StringComparer.Ordinal)
            .OrderByDescending(static x => x.Length)];

    private static string RedactValue(string value, string secret)
    {
        if (secret.Length >= MinUnboundedRedactionLength)
            return value.Replace(secret, "********", StringComparison.Ordinal);

        var result = value;
        var searchStart = 0;
        while (searchStart < result.Length)
        {
            var index = result.IndexOf(secret, searchStart, StringComparison.Ordinal);
            if (index < 0)
                return result;

            var end = index + secret.Length;
            if (IsDelimited(result, index, end))
            {
                result = result[..index] + "********" + result[end..];
                searchStart = index + "********".Length;
            }
            else
            {
                searchStart = end;
            }
        }

        return result;
    }

    private static bool IsDelimited(string value, int start, int end)
    {
        var beforeDelimited = start == 0 || !char.IsLetterOrDigit(value[start - 1]);
        var afterDelimited = end >= value.Length || !char.IsLetterOrDigit(value[end]);
        return beforeDelimited && afterDelimited;
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
